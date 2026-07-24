use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{
        audit_events, intake_question_definitions, intake_session_answers, intake_sessions,
    },
    error::ApiError,
    models::{
        AuthenticatedUser, CreateIntakeSessionRequest, IntakeInitialAnswers, IntakeQuestion,
        IntakeSessionResponse, SubmitIntakeAnswerRequest, SubmitIntakeAnswerResponse,
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
    let answers_json = serde_json::to_string(&answers).map_err(|_| ApiError::Internal)?;
    let timestamp = now();
    let session_id = Uuid::new_v4().to_string();

    let session = intake_sessions::ActiveModel {
        id: Set(session_id.clone()),
        created_by_user_id: Set(auth.id.clone()),
        case_id: Set(None),
        question_set_version: Set(question_set_version),
        status: Set("collecting".to_owned()),
        answers_json: Set(answers_json),
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
        Some(json!({ "status": "collecting", "guidance_mode": "rule_based" })),
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
    if answer_for(&answers, &field).is_some()
        || intake_session_answers::Entity::find()
            .filter(intake_session_answers::Column::SessionId.eq(session_id))
            .filter(intake_session_answers::Column::FieldCode.eq(field.as_str()))
            .one(&transaction)
            .await?
            .is_some()
    {
        return Err(ApiError::Conflict(
            "an answer for this intake field has already been submitted".to_owned(),
        ));
    }
    set_answer(&mut answers, &field, raw_answer.clone())?;

    let timestamp = now();
    let answer = intake_session_answers::ActiveModel {
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
    .await?;

    let missing_fields = missing_fields(&answers, &questions);
    let next_question = next_question(&questions, &missing_fields);
    let next_status = if required_fields_are_complete(&answers, &questions) {
        "ready_for_confirmation"
    } else {
        "collecting"
    };
    let mut updated_session = session.into_active_model();
    updated_session.answers_json =
        Set(serde_json::to_string(&answers).map_err(|_| ApiError::Internal)?);
    updated_session.status = Set(next_status.to_owned());
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
    ))
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
    let next_question = next_question(questions, &missing_fields);
    IntakeSessionResponse::new(model, answers, missing_fields, next_question)
}

fn next_question(
    questions: &[intake_question_definitions::Model],
    missing_fields: &[String],
) -> Option<IntakeQuestion> {
    questions
        .iter()
        .find(|question| missing_fields.contains(&question.field_code))
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

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
