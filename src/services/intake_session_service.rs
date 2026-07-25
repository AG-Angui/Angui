use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder, RuntimeErr, Set, SqlxError, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{
        audit_events, cases, intake_answer_revisions, intake_question_definitions,
        intake_session_answers, intake_sessions,
    },
    error::ApiError,
    models::{
        AuthenticatedUser, ConfirmIntakeSessionRequest, ConfirmIntakeSessionResponse,
        CreateCaseRequest, CreateIntakeSessionRequest, IntakeInitialAnswers, IntakePhaseProgress,
        IntakeProfileDraft, IntakeProfileDraftFields, IntakeQuestion, IntakeSessionResponse,
        IntakeStructuredFacts, SubmitIntakeAnswerRequest, SubmitIntakeAnswerResponse,
    },
    roles::AccountType,
};

pub async fn create_intake_session(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    request: CreateIntakeSessionRequest,
    answer_hard_max: usize,
) -> Result<IntakeSessionResponse, ApiError> {
    if auth.account_type != AccountType::Member {
        return Err(ApiError::Forbidden(
            "only operational member accounts can create intake sessions".to_owned(),
        ));
    }

    let transaction = db.begin().await?;
    let questions = active_questions(&transaction).await?;
    let question_set_version = questions
        .first()
        .map(|question| question.version)
        .ok_or(ApiError::Internal)?;
    let answers = normalize_answers(request.initial_answers, &questions, answer_hard_max)?;
    let phase = IntakePhaseProgress::for_answers(&answers);
    if phase_two_answers_present(&answers) && !phase.phase_transition_ready {
        return Err(ApiError::Validation(
            "complete required phase-one fields before submitting phase-two answers".to_owned(),
        ));
    }
    let answers_json = serde_json::to_string(&answers).map_err(|_| ApiError::Internal)?;
    let timestamp = now();
    let session_id = Uuid::new_v4().to_string();
    let status = if required_fields_are_complete(&answers, &questions) {
        "ready_for_confirmation"
    } else {
        "collecting"
    };

    let session = intake_sessions::ActiveModel {
        id: Set(session_id.clone()),
        created_by_user_id: Set(auth.id.clone()),
        case_id: Set(None),
        question_set_version: Set(question_set_version),
        status: Set(status.to_owned()),
        answers_json: Set(answers_json),
        assessment_json: Set("[]".to_owned()),
        structured_answers_json: Set("{}".to_owned()),
        confirmed_by_user_id: Set(None),
        confirmed_at: Set(None),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp),
    }
    .insert(&transaction)
    .await?;

    // Deliberately omit answers from audit metadata: they can include health and
    // identifying information and must not become broadly queryable metadata.
    write_audit(
        &transaction,
        auth,
        "intake_session.created",
        session_id,
        Some(json!({ "status": status, "guidance_mode": "rule_based" })),
    )
    .await?;

    transaction.commit().await?;
    Ok(response_for(session, answers, &questions))
}

pub async fn submit_intake_answer(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    request: SubmitIntakeAnswerRequest,
    answer_hard_max: usize,
) -> Result<SubmitIntakeAnswerResponse, ApiError> {
    let amap_service = crate::amap_service::AmapService::disabled();
    submit_intake_answer_with_map(
        db,
        auth,
        session_id,
        request,
        answer_hard_max,
        &amap_service,
    )
    .await
}

pub async fn submit_intake_answer_with_map(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    request: SubmitIntakeAnswerRequest,
    answer_hard_max: usize,
    amap_service: &crate::amap_service::AmapService,
) -> Result<SubmitIntakeAnswerResponse, ApiError> {
    if auth.account_type != AccountType::Member {
        return Err(ApiError::Forbidden(
            "only operational member accounts can submit intake answers".to_owned(),
        ));
    }

    let transaction = db.begin().await?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    if session.created_by_user_id != auth.id {
        return Err(ApiError::NotFound(
            "intake session was not found".to_owned(),
        ));
    }
    if session.status == "closed" || session.status == "confirmed" {
        return Err(ApiError::Conflict(
            "closed or confirmed intake sessions cannot receive answers".to_owned(),
        ));
    }
    if !matches!(
        session.status.as_str(),
        "collecting" | "ready_for_confirmation"
    ) {
        return Err(ApiError::Conflict(
            "intake session is not accepting answers".to_owned(),
        ));
    }

    let questions = questions_for_version(&transaction, session.question_set_version).await?;
    let field = request.field.trim().to_owned();
    let question = questions
        .iter()
        .find(|question| question.field_code == field)
        .ok_or_else(|| {
            ApiError::Validation("field is not enabled for this intake session".to_owned())
        })?;
    let raw_answer =
        normalize_single_answer(request.answer, question.max_answer_chars, answer_hard_max)?;
    let mut answers: IntakeInitialAnswers =
        serde_json::from_str(&session.answers_json).map_err(|_| ApiError::Internal)?;
    let existing_answer = intake_session_answers::Entity::find()
        .filter(intake_session_answers::Column::SessionId.eq(session_id))
        .filter(intake_session_answers::Column::FieldCode.eq(field.as_str()))
        .one(&transaction)
        .await?;
    if (answer_for(&answers, &field).is_some() || existing_answer.is_some()) && !request.replace {
        return Err(duplicate_answer_conflict());
    }
    let phase_before = IntakePhaseProgress::for_answers(&answers);
    if question_phase(&field) == "phase_two" && !phase_before.phase_transition_ready {
        return Err(ApiError::Conflict(
            "complete required phase-one fields before submitting phase-two answers".to_owned(),
        ));
    }
    set_answer(&mut answers, &field, raw_answer.clone())?;

    let timestamp = now();
    let structured = request.structured.unwrap_or_default();
    let mut structured_answers = parse_structured_answers(&session)?;
    merge_structured_facts(&mut structured_answers, structured.clone());
    let assessments = crate::intake_assessment::evaluate(&structured_answers, amap_service).await;
    let structured_json = serde_json::to_string(&structured).map_err(|_| ApiError::Internal)?;
    intake_answer_revisions::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        session_id: Set(session.id.clone()),
        field_code: Set(field.clone()),
        raw_answer: Set(raw_answer.clone()),
        structured_json: Set(
            (structured != IntakeStructuredFacts::default()).then_some(structured_json)
        ),
        revision_kind: Set(if existing_answer.is_some() {
            "corrected"
        } else {
            "submitted"
        }
        .to_owned()),
        created_by_user_id: Set(auth.id.clone()),
        created_at: Set(timestamp.clone()),
    }
    .insert(&transaction)
    .await?;
    let answer = if let Some(existing_answer) = existing_answer {
        let mut active = existing_answer.into_active_model();
        active.raw_answer = Set(raw_answer.clone());
        active.candidate_value = Set(raw_answer);
        active.generated_at = Set(timestamp.clone());
        active.updated_at = Set(timestamp.clone());
        active.update(&transaction).await?
    } else {
        intake_session_answers::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            session_id: Set(session.id.clone()),
            field_code: Set(field.clone()),
            raw_answer: Set(raw_answer.clone()),
            candidate_value: Set(raw_answer),
            source: Set("family_provided".to_owned()),
            status: Set("draft".to_owned()),
            generated_at: Set(timestamp.clone()),
            model: Set(None),
            template_version: Set(None),
            created_at: Set(timestamp.clone()),
            updated_at: Set(timestamp.clone()),
        }
        .insert(&transaction)
        .await
        .map_err(|error| {
            if is_unique_constraint_error(&error) {
                duplicate_answer_conflict()
            } else {
                ApiError::Database(error)
            }
        })?
    };

    let missing_fields = missing_fields(&answers, &questions);
    let phase = IntakePhaseProgress::for_answers(&answers);
    let next_question = next_question_for_phase(&questions, &missing_fields, &phase);
    let next_status = if required_fields_are_complete(&answers, &questions) {
        "ready_for_confirmation"
    } else {
        "collecting"
    };
    let mut updated_session = session.into_active_model();
    updated_session.answers_json =
        Set(serde_json::to_string(&answers).map_err(|_| ApiError::Internal)?);
    updated_session.status = Set(next_status.to_owned());
    updated_session.structured_answers_json =
        Set(serde_json::to_string(&structured_answers).map_err(|_| ApiError::Internal)?);
    updated_session.assessment_json =
        Set(serde_json::to_string(&assessments).map_err(|_| ApiError::Internal)?);
    updated_session.updated_at = Set(timestamp);
    let updated_session = updated_session.update(&transaction).await?;

    write_audit(
        &transaction,
        auth,
        "intake_session.answer_submitted",
        updated_session.id.clone(),
        Some(json!({
            "field": field,
            "candidate_source": "family_provided",
            "candidate_status": "draft",
            "guidance_mode": "rule_based",
        })),
    )
    .await?;

    transaction.commit().await?;
    Ok(SubmitIntakeAnswerResponse::new(
        updated_session,
        answer,
        missing_fields,
        next_question,
        phase,
        assessments,
    ))
}

pub async fn get_intake_profile_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
) -> Result<IntakeProfileDraft, ApiError> {
    require_operational_member(auth)?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;
    let questions = questions_for_version(db, session.question_set_version).await?;
    let answers = parse_answers(&session)?;
    let assessments = parse_assessments(&session)?;
    let confirmation_blocked_reasons = assessments
        .iter()
        .filter(|assessment| assessment.severity == "blocking")
        .map(|assessment| assessment.suggested_action.clone())
        .collect();

    Ok(IntakeProfileDraft {
        status: "draft".to_owned(),
        source_scope: "family_provided intake answers from this session only".to_owned(),
        generated_at: session.updated_at.clone(),
        requires_human_confirmation: true,
        profile: IntakeProfileDraftFields {
            physical_description: answers.basic_information.clone(),
            clothing_description: answers.belongings.clone(),
            health_notes: answers.health_status.clone(),
            mobility_notes: answers.health_status.clone(),
            transportation_ability: answers.transport_ability.clone(),
            frequent_locations: answers.frequent_locations.clone(),
            last_seen_information: answers.last_seen.clone(),
            behavior_habits: answers.behavior_habits.clone(),
            suspicious_motive: answers.suspicious_motive.clone(),
        },
        missing_fields: missing_fields(&answers, &questions),
        assessments,
        confirmation_blocked_reasons,
        direction_hypotheses: direction_hypotheses(&answers, &session.updated_at),
    })
}

pub async fn confirm_intake_session(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    request: ConfirmIntakeSessionRequest,
) -> Result<ConfirmIntakeSessionResponse, ApiError> {
    require_operational_member(auth)?;
    if !request.human_confirmed {
        return Err(ApiError::Validation(
            "human_confirmed must be true before creating a case".to_owned(),
        ));
    }
    let case_request = case_request_from_confirmed_profile(request);
    let transaction = db.begin().await?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;

    if let Some(case_id) = &session.case_id {
        let case_model = cases::Entity::find_by_id(case_id)
            .one(&transaction)
            .await?
            .ok_or(ApiError::Internal)?;
        transaction.commit().await?;
        return Ok(confirmed_response(case_model, session.confirmed_at));
    }
    if session.status != "ready_for_confirmation" {
        return Err(ApiError::Conflict(
            "intake session is not ready for confirmation".to_owned(),
        ));
    }
    let assessments = parse_assessments(&session)?;
    let blocking_reasons = assessments
        .iter()
        .filter(|assessment| assessment.severity == "blocking")
        .map(|assessment| assessment.suggested_action.clone())
        .collect::<Vec<_>>();
    if !blocking_reasons.is_empty() {
        return Err(ApiError::Conflict(format!(
            "intake session has blocking consistency checks: {}",
            blocking_reasons.join(" | ")
        )));
    }

    let timestamp = now();
    let case_model =
        crate::services::case_service::insert_case_records(&transaction, &case_request, &timestamp)
            .await?;
    crate::services::case_service::insert_membership(
        &transaction,
        &case_model.id,
        &auth.id,
        crate::roles::CaseRole::Family,
        Some(&auth.id),
        &timestamp,
    )
    .await?;
    crate::services::case_service::write_audit(
        &transaction,
        Some(case_model.id.clone()),
        auth,
        "case.created",
        "case",
        case_model.id.clone(),
        Some(json!({ "status": "active", "source": "intake_session_confirmation" })),
    )
    .await?;

    let mut updated_session = session.into_active_model();
    updated_session.case_id = Set(Some(case_model.id.clone()));
    updated_session.status = Set("confirmed".to_owned());
    updated_session.confirmed_by_user_id = Set(Some(auth.id.clone()));
    updated_session.confirmed_at = Set(Some(timestamp.clone()));
    updated_session.updated_at = Set(timestamp.clone());
    updated_session
        .update(&transaction)
        .await
        .map_err(|error| {
            if is_unique_constraint_error(&error) {
                ApiError::Conflict(
                "intake session was confirmed by a concurrent request; retry to retrieve the case"
                    .to_owned(),
            )
            } else {
                ApiError::Database(error)
            }
        })?;
    write_audit(
        &transaction,
        auth,
        "intake_session.confirmed",
        session_id.to_owned(),
        Some(json!({ "case_id": case_model.id, "confirmation_status": "human_confirmed" })),
    )
    .await?;

    transaction.commit().await?;
    Ok(confirmed_response(case_model, Some(timestamp)))
}

fn require_operational_member(auth: &AuthenticatedUser) -> Result<(), ApiError> {
    if auth.account_type != AccountType::Member {
        return Err(ApiError::Forbidden(
            "only operational member accounts can use intake sessions".to_owned(),
        ));
    }
    Ok(())
}

fn require_session_creator(
    session: &intake_sessions::Model,
    auth: &AuthenticatedUser,
) -> Result<(), ApiError> {
    if session.created_by_user_id != auth.id {
        return Err(ApiError::NotFound(
            "intake session was not found".to_owned(),
        ));
    }
    Ok(())
}

fn parse_answers(session: &intake_sessions::Model) -> Result<IntakeInitialAnswers, ApiError> {
    serde_json::from_str(&session.answers_json).map_err(|_| ApiError::Internal)
}

fn parse_structured_answers(
    session: &intake_sessions::Model,
) -> Result<IntakeStructuredFacts, ApiError> {
    serde_json::from_str(&session.structured_answers_json).map_err(|_| ApiError::Internal)
}

fn parse_assessments(
    session: &intake_sessions::Model,
) -> Result<Vec<crate::models::IntakeAssessment>, ApiError> {
    serde_json::from_str(&session.assessment_json).map_err(|_| ApiError::Internal)
}

fn merge_structured_facts(target: &mut IntakeStructuredFacts, source: IntakeStructuredFacts) {
    if source.last_seen_at.is_some() {
        target.last_seen_at = source.last_seen_at;
    }
    if source.last_seen_location.is_some() {
        target.last_seen_location = source.last_seen_location;
    }
    if source.follow_up_at.is_some() {
        target.follow_up_at = source.follow_up_at;
    }
    if source.follow_up_location.is_some() {
        target.follow_up_location = source.follow_up_location;
    }
    if source.mobility.is_some() {
        target.mobility = source.mobility;
    }
    if !source.transport_modes.is_empty() {
        target.transport_modes = source.transport_modes;
    }
    if source.companion_status.is_some() {
        target.companion_status = source.companion_status;
    }
    if !source.belongings.is_empty() {
        target.belongings = source.belongings;
    }
}

fn question_phase(field: &str) -> &'static str {
    match field {
        "basic_information" | "health_status" | "behavior_habits" | "last_seen" => "phase_one",
        _ => "phase_two",
    }
}

fn phase_two_answers_present(answers: &IntakeInitialAnswers) -> bool {
    answers.frequent_locations.is_some()
        || answers.suspicious_motive.is_some()
        || answers.belongings.is_some()
        || answers.transport_ability.is_some()
        || answers.follow_up_clues.is_some()
}

fn case_request_from_confirmed_profile(request: ConfirmIntakeSessionRequest) -> CreateCaseRequest {
    CreateCaseRequest {
        display_name: request.profile.display_name,
        age: request.profile.age,
        gender: request.profile.gender,
        physical_description: request.profile.physical_description,
        clothing_description: request.profile.clothing_description,
        health_notes: request.profile.health_notes,
        last_seen_at: request.profile.last_seen_at,
        last_seen_location: Some(request.profile.last_seen_location),
    }
}

fn confirmed_response(
    case_model: cases::Model,
    confirmed_at: Option<String>,
) -> ConfirmIntakeSessionResponse {
    ConfirmIntakeSessionResponse {
        case_id: case_model.id,
        case_code: case_model.case_code,
        status: case_model.status,
        confirmation_status: "human_confirmed".to_owned(),
        confirmed_at: confirmed_at.unwrap_or(case_model.updated_at),
    }
}

fn duplicate_answer_conflict() -> ApiError {
    ApiError::Conflict("an answer for this intake field has already been submitted".to_owned())
}

fn is_unique_constraint_error(error: &DbErr) -> bool {
    matches!(
        error,
        DbErr::Exec(RuntimeErr::SqlxError(SqlxError::Database(database_error)))
            if database_error.is_unique_violation()
    )
}

async fn active_questions<C: ConnectionTrait>(
    db: &C,
) -> Result<Vec<intake_question_definitions::Model>, ApiError> {
    let latest = intake_question_definitions::Entity::find()
        .filter(intake_question_definitions::Column::Status.eq("active"))
        .order_by_desc(intake_question_definitions::Column::Version)
        .one(db)
        .await?
        .ok_or(ApiError::Internal)?;
    let questions = intake_question_definitions::Entity::find()
        .filter(intake_question_definitions::Column::Status.eq("active"))
        .filter(intake_question_definitions::Column::Version.eq(latest.version))
        .order_by_asc(intake_question_definitions::Column::DisplayOrder)
        .all(db)
        .await?;
    validate_question_configuration(&questions)?;
    Ok(questions)
}

async fn questions_for_version<C: ConnectionTrait>(
    db: &C,
    version: i32,
) -> Result<Vec<intake_question_definitions::Model>, ApiError> {
    let questions = intake_question_definitions::Entity::find()
        .filter(intake_question_definitions::Column::Version.eq(version))
        .order_by_asc(intake_question_definitions::Column::DisplayOrder)
        .all(db)
        .await?;
    validate_question_configuration(&questions)?;
    Ok(questions)
}

async fn write_audit<C: ConnectionTrait>(
    db: &C,
    auth: &AuthenticatedUser,
    action: &str,
    session_id: String,
    metadata: Option<serde_json::Value>,
) -> Result<(), ApiError> {
    let metadata_json = metadata.map(|mut value| {
        if let Some(object) = value.as_object_mut() {
            object.insert("actor_account_type".to_owned(), json!(auth.account_type));
            object.insert(
                "actor_global_capabilities".to_owned(),
                json!(auth.global_capabilities),
            );
        }
        value.to_string()
    });
    audit_events::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        case_id: Set(None),
        actor: Set(auth.id.clone()),
        action: Set(action.to_owned()),
        entity_type: Set("intake_session".to_owned()),
        entity_id: Set(session_id),
        metadata_json: Set(metadata_json),
        created_at: Set(now()),
    }
    .insert(db)
    .await?;
    Ok(())
}

fn normalize_answers(
    mut answers: IntakeInitialAnswers,
    questions: &[intake_question_definitions::Model],
    answer_hard_max: usize,
) -> Result<IntakeInitialAnswers, ApiError> {
    for (field_code, answer) in [
        ("basic_information", &mut answers.basic_information),
        ("health_status", &mut answers.health_status),
        ("behavior_habits", &mut answers.behavior_habits),
        ("last_seen", &mut answers.last_seen),
        ("frequent_locations", &mut answers.frequent_locations),
        ("suspicious_motive", &mut answers.suspicious_motive),
        ("belongings", &mut answers.belongings),
        ("transport_ability", &mut answers.transport_ability),
        ("follow_up_clues", &mut answers.follow_up_clues),
    ]
    .into_iter()
    {
        let Some(answer) = answer else {
            continue;
        };
        let question = questions
            .iter()
            .find(|question| question.field_code == field_code)
            .ok_or_else(|| {
                ApiError::Validation(format!(
                    "{field_code} is not enabled in the active intake question set"
                ))
            })?;
        let answer_limit = usize::try_from(question.max_answer_chars)
            .map_err(|_| ApiError::Internal)?
            .min(answer_hard_max);
        let trimmed = answer.trim();
        if trimmed.is_empty() || trimmed.chars().count() > answer_limit {
            return Err(ApiError::Validation(format!(
                "{field_code} must contain between 1 and {answer_limit} characters"
            )));
        }
        *answer = trimmed.to_owned();
    }
    Ok(answers)
}

fn response_for(
    model: intake_sessions::Model,
    answers: IntakeInitialAnswers,
    questions: &[intake_question_definitions::Model],
) -> IntakeSessionResponse {
    let missing_fields = missing_fields(&answers, questions);
    let phase = IntakePhaseProgress::for_answers(&answers);
    let next_question = next_question_for_phase(questions, &missing_fields, &phase);
    IntakeSessionResponse::new(model, answers, missing_fields, next_question)
}

fn next_question_for_phase(
    questions: &[intake_question_definitions::Model],
    missing_fields: &[String],
    phase: &IntakePhaseProgress,
) -> Option<IntakeQuestion> {
    let wanted_phase = if phase.phase_transition_ready {
        "phase_two"
    } else {
        "phase_one"
    };
    questions
        .iter()
        .find(|question| {
            missing_fields.contains(&question.field_code)
                && question_phase(&question.field_code) == wanted_phase
        })
        .or_else(|| {
            questions
                .iter()
                .find(|question| missing_fields.contains(&question.field_code))
        })
        .map(|question| IntakeQuestion {
            field: question.field_code.clone(),
            prompt: question.prompt.clone(),
            required: question.is_required,
        })
}

fn missing_fields(
    answers: &IntakeInitialAnswers,
    questions: &[intake_question_definitions::Model],
) -> Vec<String> {
    questions
        .iter()
        .filter(|question| answer_for(answers, &question.field_code).is_none())
        .map(|question| question.field_code.clone())
        .collect()
}

fn answer_for<'a>(answers: &'a IntakeInitialAnswers, field_code: &str) -> Option<&'a String> {
    match field_code {
        "basic_information" => answers.basic_information.as_ref(),
        "health_status" => answers.health_status.as_ref(),
        "behavior_habits" => answers.behavior_habits.as_ref(),
        "last_seen" => answers.last_seen.as_ref(),
        "frequent_locations" => answers.frequent_locations.as_ref(),
        "suspicious_motive" => answers.suspicious_motive.as_ref(),
        "belongings" => answers.belongings.as_ref(),
        "transport_ability" => answers.transport_ability.as_ref(),
        "follow_up_clues" => answers.follow_up_clues.as_ref(),
        _ => None,
    }
}

fn set_answer(
    answers: &mut IntakeInitialAnswers,
    field_code: &str,
    answer: String,
) -> Result<(), ApiError> {
    match field_code {
        "basic_information" => answers.basic_information = Some(answer),
        "health_status" => answers.health_status = Some(answer),
        "behavior_habits" => answers.behavior_habits = Some(answer),
        "last_seen" => answers.last_seen = Some(answer),
        "frequent_locations" => answers.frequent_locations = Some(answer),
        "suspicious_motive" => answers.suspicious_motive = Some(answer),
        "belongings" => answers.belongings = Some(answer),
        "transport_ability" => answers.transport_ability = Some(answer),
        "follow_up_clues" => answers.follow_up_clues = Some(answer),
        _ => {
            return Err(ApiError::Validation(
                "field is not enabled for this intake session".to_owned(),
            ));
        }
    }
    Ok(())
}

fn normalize_single_answer(
    answer: String,
    configured_limit: i32,
    answer_hard_max: usize,
) -> Result<String, ApiError> {
    let answer_limit = usize::try_from(configured_limit)
        .map_err(|_| ApiError::Internal)?
        .min(answer_hard_max);
    let trimmed = answer.trim();
    if trimmed.is_empty() || trimmed.chars().count() > answer_limit {
        return Err(ApiError::Validation(format!(
            "answer must contain between 1 and {answer_limit} characters"
        )));
    }
    Ok(trimmed.to_owned())
}

fn required_fields_are_complete(
    answers: &IntakeInitialAnswers,
    questions: &[intake_question_definitions::Model],
) -> bool {
    questions
        .iter()
        .filter(|question| question.is_required)
        .all(|question| answer_for(answers, &question.field_code).is_some())
}

fn validate_question_configuration(
    questions: &[intake_question_definitions::Model],
) -> Result<(), ApiError> {
    if questions.is_empty()
        || questions.iter().any(|question| {
            question.max_answer_chars <= 0
                || !matches!(
                    question.field_code.as_str(),
                    "basic_information"
                        | "health_status"
                        | "behavior_habits"
                        | "last_seen"
                        | "frequent_locations"
                        | "suspicious_motive"
                        | "belongings"
                        | "transport_ability"
                        | "follow_up_clues"
                )
        })
    {
        return Err(ApiError::Internal);
    }
    Ok(())
}

fn direction_hypotheses(
    answers: &IntakeInitialAnswers,
    generated_at: &str,
) -> Vec<crate::models::IntakeDirectionHypothesis> {
    let Some(frequent_locations) = answers.frequent_locations.as_ref() else {
        return Vec::new();
    };
    vec![crate::models::IntakeDirectionHypothesis {
        status: "hypothesis".to_owned(),
        source_fields: vec!["frequent_locations".to_owned(), "last_seen".to_owned()],
        generated_at: generated_at.to_owned(),
        uncertainty_notice: "This is an unverified direction candidate derived from family-provided draft answers. It is not a fact, task, or publication decision.".to_owned(),
        description: format!("Consider verifying reported frequent locations: {frequent_locations}"),
    }]
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
