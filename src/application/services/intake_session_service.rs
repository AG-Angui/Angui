use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, RuntimeErr, Set, SqlxError,
    TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{
        audit_events, case_attachments, cases, intake_answer_revisions, intake_profile_drafts,
        intake_question_definitions, intake_session_answers, intake_session_photos,
        intake_sessions,
    },
    error::ApiError,
    models::{
        AcknowledgeIntakeAiInitialReviewRequest, AuthenticatedUser, ConfirmIntakeSessionRequest,
        ConfirmIntakeSessionResponse, ConfirmedIntakeProfile, CreateCaseRequest,
        CreateIntakeSessionRequest, IntakeAiFollowUp, IntakeAiFollowUpResponse,
        IntakeAiInitialReviewIssue, IntakeAiInitialReviewResponse, IntakeAnswerRevisionResponse,
        IntakeInitialAnswers, IntakePhaseProgress, IntakeProfileDraft,
        IntakeProfileDraftDiffResponse, IntakeProfileDraftFieldMetadata, IntakeProfileDraftFields,
        IntakeProfileDraftVersionsResponse, IntakeQuestion, IntakeSessionResponse,
        IntakeStructuredFacts, RestoreIntakeAnswerRequest, RestoreIntakeProfileDraftRequest,
        ReviewIntakeProfileDraftRequest, StartIntakeAiInitialReviewRequest,
        SubmitIntakeAnswerRequest, SubmitIntakeAnswerResponse,
    },
    roles::AccountType,
};

use crate::ai_gateway::{
    AiCapability, AiExecutionResult, AiPurpose, AiRequest, AiTaskStatus, DataLevel,
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
    let phase = IntakePhaseProgress::for_answers(&answers, question_set_version);
    if phase_two_answers_present(&answers, question_set_version) && !phase.phase_transition_ready {
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
        ai_initial_review_status: Set("not_started".to_owned()),
        ai_initial_review_json: Set("[]".to_owned()),
        ai_initial_review_profile_json: Set(None),
        ai_initial_reviewed_at: Set(None),
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
    validate_report_field(&field, &raw_answer, session.question_set_version >= 3)?;
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
    let phase_before = IntakePhaseProgress::for_answers(&answers, session.question_set_version);
    if question_phase(&field, session.question_set_version) == "phase_two"
        && !phase_before.phase_transition_ready
    {
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
    let phase = IntakePhaseProgress::for_answers(&answers, session.question_set_version);
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
    // A correction changes the exact material seen by the initial review. Do
    // not let a family reuse acknowledgements that were made against the old
    // values or profile snapshot.
    updated_session.ai_initial_review_status = Set("not_started".to_owned());
    updated_session.ai_initial_review_json = Set("[]".to_owned());
    updated_session.ai_initial_review_profile_json = Set(None);
    updated_session.ai_initial_reviewed_at = Set(None);
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
    if let Some(draft) = intake_profile_drafts::Entity::find()
        .filter(intake_profile_drafts::Column::SessionId.eq(session_id))
        .order_by_desc(intake_profile_drafts::Column::Version)
        .one(db)
        .await?
    {
        return profile_draft_from_model(draft);
    }
    let questions = questions_for_version(db, session.question_set_version).await?;
    let answers = parse_answers(&session)?;
    let stored_answers = intake_session_answers::Entity::find()
        .filter(intake_session_answers::Column::SessionId.eq(session_id))
        .all(db)
        .await?;
    let assessments = parse_assessments(&session)?;
    let confirmation_blocked_reasons = assessments
        .iter()
        .filter(|assessment| assessment.severity == "blocking")
        .map(|assessment| assessment.suggested_action.clone())
        .collect();

    Ok(IntakeProfileDraft {
        id: "unpersisted-family-draft".to_owned(),
        status: "draft".to_owned(),
        source_scope: "family_provided intake answers from this session only".to_owned(),
        generated_at: session.updated_at.clone(),
        provider_model: None,
        template_version: "intake-profile-family-draft-v1".to_owned(),
        degradation_status: "model_generation_pending".to_owned(),
        version: 0,
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
        field_metadata: profile_draft_field_metadata(&answers, &stored_answers, &session),
        missing_fields: missing_fields(&answers, &questions),
        assessments,
        confirmation_blocked_reasons,
        direction_hypotheses: direction_hypotheses(&answers, &session.updated_at),
    })
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileExtractionOutput {
    profile: IntakeProfileDraftFields,
    field_sources: std::collections::BTreeMap<String, Option<ProfileFieldSource>>,
}

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileFieldSource {
    source_field: String,
    source_excerpt: String,
}

/// Compatibility shape observed from Responses-compatible providers that ignore
/// the requested `text.format` schema. It is normalized and then subjected to
/// the same source-excerpt checks as the canonical response.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileCandidatesOutput {
    profile_candidates: ProfileCandidates,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileCandidates {
    name: Option<ProfileCandidate>,
    gender: Option<ProfileCandidate>,
    age: Option<ProfileCandidate>,
    height: Option<ProfileCandidate>,
    appearance: Option<ProfileCandidate>,
    behavior_habit: Option<ProfileCandidate>,
    belongings: Option<ProfileCandidate>,
    #[serde(rename = "follow_up_clues")]
    _follow_up_clues: Option<ProfileCandidate>,
    frequent_locations: Option<ProfileCandidate>,
    health_status: Option<ProfileCandidate>,
    last_seen: Option<ProfileCandidate>,
    suspicious_motive: Option<ProfileCandidate>,
    transport_ability: Option<ProfileCandidate>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileCandidate {
    value: String,
    source_answer_field: String,
    supporting_excerpt: String,
}

/// Produces a versioned candidate from the family's immutable answer snapshot.
/// It can never alter answers or create a case; all returned values remain a
/// family-confirmed draft until the existing second confirmation flow.
pub async fn generate_intake_profile_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    gateway: &crate::ai_gateway::AiGateway,
) -> Result<IntakeProfileDraft, ApiError> {
    require_operational_member(auth)?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;
    if session.case_id.is_some() || session.status == "confirmed" {
        return Err(ApiError::Conflict(
            "confirmed intake sessions cannot generate a profile candidate".to_owned(),
        ));
    }
    let answers = parse_answers(&session)?;
    let input = serde_json::to_string(&json!({"family_answers": answers_for_ai(&answers)}))
        .map_err(|_| ApiError::Internal)?;
    let request = AiRequest {
        capability: AiCapability::StructuredExtraction,
        data_level: DataLevel::Sensitive,
        purpose: AiPurpose::IntakeDraft,
        data_region: "CN".to_owned(),
        system_instruction: Some(profile_extraction_system_instruction()),
        output_schema: Some(profile_extraction_schema()),
        output_schema_name: Some("intake_profile_candidate".to_owned()),
        input,
        requested_output_tokens: 900,
        template_version: "intake-profile-extraction-v1".to_owned(),
        input_scope_reference: "intake-session-family-answers-only".to_owned(),
        redaction_policy_version: "intake-sensitive-minimization-v1".to_owned(),
    };
    let execution = gateway.execute(&request).await;
    let execution_audits = crate::ai_gateway::execution_attempt_audits(&request, &execution);
    let (profile, metadata, provider_model, degradation_status, _audit_status) = match execution {
        AiExecutionResult::Completed { route, output, .. } => {
            match decode_profile_extraction_output(&output, &answers) {
                Ok((out, used_provider_compatibility_shape)) => {
                    if used_provider_compatibility_shape {
                        log::debug!(
                            target: "angui::ai_gateway::diagnostic",
                            "AI profile extraction used the provider compatibility schema"
                        );
                    }
                    match validate_profile_extraction(out, &answers) {
                        Ok((profile, metadata)) => (
                            profile,
                            metadata,
                            Some(route.model),
                            "manual_review_required".to_owned(),
                            AiTaskStatus::Completed,
                        ),
                        Err(reason) => {
                            log::debug!(
                                target: "angui::ai_gateway::diagnostic",
                                "AI profile extraction rejected: {reason}"
                            );
                            profile_fallback(&answers, &session)
                        }
                    }
                }
                Err(reason) => {
                    log::debug!(
                        target: "angui::ai_gateway::diagnostic",
                        "AI profile extraction rejected: {reason}"
                    );
                    profile_fallback(&answers, &session)
                }
            }
        }
        AiExecutionResult::Degraded { .. } => profile_fallback(&answers, &session),
        AiExecutionResult::Failed { .. } => profile_fallback(&answers, &session),
    };
    let timestamp = now();
    let transaction = db.begin().await?;
    let latest = intake_profile_drafts::Entity::find()
        .filter(intake_profile_drafts::Column::SessionId.eq(session_id))
        .order_by_desc(intake_profile_drafts::Column::Version)
        .one(&transaction)
        .await?;
    let version = latest.as_ref().map_or(1, |value| value.version + 1);
    let model = intake_profile_drafts::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        session_id: Set(session_id.to_owned()),
        version: Set(version),
        parent_draft_id: Set(latest.map(|value| value.id)),
        profile_json: Set(serde_json::to_string(&profile).map_err(|_| ApiError::Internal)?),
        field_metadata_json: Set(serde_json::to_string(&metadata).map_err(|_| ApiError::Internal)?),
        status: Set("draft".to_owned()),
        degradation_status: Set(degradation_status),
        provider_model: Set(provider_model),
        template_version: Set("intake-profile-extraction-v1".to_owned()),
        generated_at: Set(timestamp.clone()),
        confirmed_by_user_id: Set(None),
        confirmed_at: Set(None),
        created_at: Set(timestamp),
    }
    .insert(&transaction)
    .await?;
    crate::ai_gateway::persist_execution_audits(&transaction, &execution_audits, &auth.id, None)
        .await?;
    write_audit(&transaction, auth, "intake_profile_draft.generated", session_id.to_owned(), Some(json!({"version": version, "degradation_status": model.degradation_status, "provider_configured": model.provider_model.is_some()}))).await?;
    transaction.commit().await?;
    profile_draft_from_model(model)
}

/// Runs the controlled initial review after the family's first confirmation.
/// The result remains a family-only draft and cannot create a case, change an
/// answer, or make a location/fact conclusion. The exact reviewed profile is
/// saved so the later second confirmation cannot silently submit different
/// content.
pub async fn start_ai_initial_review(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    request: StartIntakeAiInitialReviewRequest,
    gateway: &crate::ai_gateway::AiGateway,
) -> Result<IntakeAiInitialReviewResponse, ApiError> {
    require_operational_member(auth)?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;
    if session.case_id.is_some() || session.status == "confirmed" {
        return Err(ApiError::Conflict(
            "a confirmed intake session cannot be reviewed again".to_owned(),
        ));
    }
    if !matches!(
        session.status.as_str(),
        "ready_for_confirmation" | "awaiting_family_review" | "ready_for_second_confirmation"
    ) {
        return Err(ApiError::Conflict(
            "complete the required intake answers before starting initial review".to_owned(),
        ));
    }

    let answers = parse_answers(&session)?;
    let ai_answers = answers_for_ai(&answers);
    let assessments = parse_assessments(&session)?;
    let input = serde_json::to_string(&json!({
        "family_answers": ai_answers,
        "family_reviewed_profile": request.profile,
        "rule_consistency_checks": assessments,
        "review_task": "Identify only ambiguous, incomplete, or internally inconsistent family-provided information that needs the family's confirmation."
    }))
    .map_err(|_| ApiError::Internal)?;
    let reviewed_at = now();
    let ai_request = AiRequest {
        capability: AiCapability::Inquiry,
        data_level: DataLevel::Sensitive,
        purpose: AiPurpose::IntakeDraft,
        data_region: "CN".to_owned(),
        system_instruction: Some("Return JSON only: {issues:[{field,severity,evidence_summary,clarification_question,source_fields}]}. `severity` must be `needs_confirmation` or `warning`. Raise at most 12 concrete items based only on the supplied family answers and which configured answer fields are still unanswered. `field` and `source_fields` must be `profile` or one of: basic_information, health_status, behavior_habits, last_seen, frequent_locations, belongings, transport_ability, follow_up_clues, suspicious_motive, police_report_status. Flag placeholder, repeated-character, pure-number, or low-information descriptions for family clarification. An unanswered configured field may be cited only to explain that information is missing. Do not diagnose, decide whether any report is true, infer a current or future location, advise an emergency action, add facts, or rewrite the family's answers. An empty issues array is valid.".to_owned()),
        output_schema: Some(initial_review_schema()),
        output_schema_name: Some("intake_initial_review".to_owned()),
        input,
        requested_output_tokens: 700,
        template_version: "intake-initial-review-ai-v1".to_owned(),
        input_scope_reference: "intake-session-authorized-family-review".to_owned(),
        redaction_policy_version: "intake-sensitive-minimization-v1".to_owned(),
    };
    let execution = gateway.execute(&ai_request).await;
    let execution_audits = crate::ai_gateway::execution_attempt_audits(&ai_request, &execution);
    let (mut issues, review_status, _audit_status) = match execution {
        AiExecutionResult::Completed { output, .. } => {
            match gateway.decode_json::<InitialReviewModelOutput>(&output) {
                Ok(output) => match validate_initial_review_output(output) {
                    Ok(issues) => (
                        issues,
                        "available_pending".to_owned(),
                        AiTaskStatus::Completed,
                    ),
                    Err(error) => {
                        log::debug!(
                            target: "angui::ai_gateway::diagnostic",
                            "AI initial review rejected: {error}"
                        );
                        (
                            rule_based_initial_review_issues(&assessments),
                            "rule_based_fallback_pending".to_owned(),
                            AiTaskStatus::Failed,
                        )
                    }
                },
                Err(_) => {
                    log::debug!(
                        target: "angui::ai_gateway::diagnostic",
                        "AI initial review rejected: output did not match the review schema"
                    );
                    (
                        rule_based_initial_review_issues(&assessments),
                        "rule_based_fallback_pending".to_owned(),
                        AiTaskStatus::Failed,
                    )
                }
            }
        }
        AiExecutionResult::Degraded { .. } => (
            rule_based_initial_review_issues(&assessments),
            "rule_based_fallback_pending".to_owned(),
            AiTaskStatus::Degraded,
        ),
        AiExecutionResult::Failed { .. } => (
            rule_based_initial_review_issues(&assessments),
            "rule_based_fallback_pending".to_owned(),
            AiTaskStatus::Failed,
        ),
    };
    append_content_quality_issues(&mut issues, &answers);

    let transaction = db.begin().await?;
    let current = intake_sessions::Entity::find_by_id(session_id)
        .lock_exclusive()
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&current, auth)?;
    if current.updated_at != session.updated_at
        || current.case_id.is_some()
        || current.status == "confirmed"
    {
        return Err(ApiError::Conflict(
            "intake answers changed while the initial review was running; start it again"
                .to_owned(),
        ));
    }
    let profile_json = serde_json::to_string(&request.profile).map_err(|_| ApiError::Internal)?;
    let issues_json = serde_json::to_string(&issues).map_err(|_| ApiError::Internal)?;
    let mut updated = current.into_active_model();
    updated.status = Set("awaiting_family_review".to_owned());
    updated.ai_initial_review_status = Set(review_status);
    updated.ai_initial_review_json = Set(issues_json);
    updated.ai_initial_review_profile_json = Set(Some(profile_json));
    updated.ai_initial_reviewed_at = Set(Some(reviewed_at.clone()));
    updated.updated_at = Set(reviewed_at.clone());
    let updated = updated.update(&transaction).await?;
    crate::ai_gateway::persist_execution_audits(&transaction, &execution_audits, &auth.id, None)
        .await?;
    write_audit(
        &transaction,
        auth,
        "intake_session.ai_initial_review_completed",
        session_id.to_owned(),
        Some(json!({
            "review_status": updated.ai_initial_review_status,
            "issue_count": issues.len(),
            "has_blocking_rule_checks": assessments.iter().any(|item| item.severity == "blocking"),
        })),
    )
    .await?;
    transaction.commit().await?;
    initial_review_response(&updated)
}

pub async fn get_ai_initial_review(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
) -> Result<IntakeAiInitialReviewResponse, ApiError> {
    require_operational_member(auth)?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;
    initial_review_response(&session)
}

pub async fn acknowledge_ai_initial_review(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    request: AcknowledgeIntakeAiInitialReviewRequest,
) -> Result<IntakeAiInitialReviewResponse, ApiError> {
    require_operational_member(auth)?;
    if !request.human_confirmed {
        return Err(ApiError::Validation(
            "human_confirmed must be true before the second confirmation".to_owned(),
        ));
    }
    let transaction = db.begin().await?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .lock_exclusive()
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;
    if session.status != "awaiting_family_review" {
        return Err(ApiError::Conflict(
            "this intake session is not awaiting family review acknowledgement".to_owned(),
        ));
    }
    let issues = parse_initial_review_issues(&session)?;
    let mut expected = issues
        .iter()
        .map(|issue| issue.id.clone())
        .collect::<Vec<_>>();
    expected.sort();
    let mut submitted = request.confirmed_issue_ids;
    submitted.sort();
    submitted.dedup();
    if submitted != expected {
        return Err(ApiError::Validation(
            "confirm every displayed initial-review item, or correct an answer and run initial review again"
                .to_owned(),
        ));
    }
    let assessments = parse_assessments(&session)?;
    let next_review_status = acknowledged_review_status(&session.ai_initial_review_status);
    let mut updated = session.into_active_model();
    updated.status = Set("ready_for_second_confirmation".to_owned());
    updated.ai_initial_review_status = Set(next_review_status);
    updated.updated_at = Set(now());
    let updated = updated.update(&transaction).await?;
    write_audit(
        &transaction,
        auth,
        "intake_session.ai_initial_review_acknowledged",
        session_id.to_owned(),
        Some(json!({
            "issue_count": issues.len(),
            "has_blocking_rule_checks": assessments.iter().any(|item| item.severity == "blocking"),
        })),
    )
    .await?;
    transaction.commit().await?;
    initial_review_response(&updated)
}

pub async fn get_ai_follow_up(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    gateway: &crate::ai_gateway::AiGateway,
) -> Result<IntakeAiFollowUpResponse, ApiError> {
    require_operational_member(auth)?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;
    if !can_request_ai_follow_up(&session.status) {
        return Ok(IntakeAiFollowUpResponse {
            question: None,
            degradation_status: "not_applicable".to_owned(),
            generated_at: now(),
        });
    }
    let questions = questions_for_version(db, session.question_set_version).await?;
    let answers = parse_answers(&session)?;
    let missing = missing_fields(&answers, &questions);
    let phase = IntakePhaseProgress::for_answers(&answers, session.question_set_version);
    let fallback = next_follow_up_question(&questions, &missing, &phase);
    let Some(fallback) = fallback else {
        return Ok(IntakeAiFollowUpResponse {
            question: None,
            degradation_status: "not_applicable".to_owned(),
            generated_at: now(),
        });
    };
    let input = serde_json::to_string(&serde_json::json!({
        "answers": answers_for_ai(&answers),
        "missing_fields": missing,
        "fallback_field": fallback.field.clone(),
        "fallback_required": fallback.required,
    }))
    .map_err(|_| ApiError::Internal)?;
    let ai_request = AiRequest {
        capability: AiCapability::Inquiry,
        data_level: DataLevel::Sensitive,
        purpose: AiPurpose::IntakeDraft,
        data_region: "CN".to_owned(),
        system_instruction: Some("Return JSON only: {field,prompt,purpose,missing_fields,skippable}. Ask one factual question for fallback_field only. Set skippable to false when fallback_required is true and true otherwise. Do not infer a location, diagnosis, action, or emergency conclusion.".to_owned()),
        output_schema: Some(follow_up_schema()),
        output_schema_name: Some("intake_follow_up".to_owned()),
        input,
        requested_output_tokens: 220,
        template_version: "intake-follow-up-ai-v1".to_owned(),
        input_scope_reference: "intake-session-authorized-answers".to_owned(),
        redaction_policy_version: "intake-sensitive-minimization-v1".to_owned(),
    };
    let execution = gateway.execute(&ai_request).await;
    let execution_audits = crate::ai_gateway::execution_attempt_audits(&ai_request, &execution);
    let (question, degradation_status, _audit_status) = match execution {
        AiExecutionResult::Completed { output, .. } => {
            match gateway.decode_json::<IntakeAiFollowUp>(&output) {
                Ok(question) if valid_follow_up(&question, &fallback, &missing) => (
                    Some(normalize_follow_up(question)),
                    "available".to_owned(),
                    AiTaskStatus::Completed,
                ),
                _ => (
                    Some(static_follow_up(fallback, missing.clone())),
                    "rule_based_fallback".to_owned(),
                    AiTaskStatus::Failed,
                ),
            }
        }
        AiExecutionResult::Degraded { .. } => (
            Some(static_follow_up(fallback, missing.clone())),
            "rule_based_fallback".to_owned(),
            AiTaskStatus::Degraded,
        ),
        AiExecutionResult::Failed { .. } => (
            Some(static_follow_up(fallback, missing.clone())),
            "rule_based_fallback".to_owned(),
            AiTaskStatus::Failed,
        ),
    };
    crate::ai_gateway::persist_execution_audits(db, &execution_audits, &auth.id, None).await?;
    Ok(IntakeAiFollowUpResponse {
        question,
        degradation_status,
        generated_at: now(),
    })
}

pub async fn list_answer_revisions(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
) -> Result<Vec<IntakeAnswerRevisionResponse>, ApiError> {
    require_operational_member(auth)?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;
    Ok(intake_answer_revisions::Entity::find()
        .filter(intake_answer_revisions::Column::SessionId.eq(session_id))
        .order_by_asc(intake_answer_revisions::Column::CreatedAt)
        .all(db)
        .await?
        .into_iter()
        .map(|revision| IntakeAnswerRevisionResponse {
            id: revision.id,
            field: revision.field_code,
            answer: revision.raw_answer,
            revision_kind: revision.revision_kind,
            created_at: revision.created_at,
        })
        .collect())
}

pub async fn restore_answer_revision(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    field: &str,
    request: RestoreIntakeAnswerRequest,
    answer_hard_max: usize,
) -> Result<SubmitIntakeAnswerResponse, ApiError> {
    require_operational_member(auth)?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;
    let revision = intake_answer_revisions::Entity::find_by_id(request.revision_id)
        .one(db)
        .await?
        .filter(|revision| revision.session_id == session_id && revision.field_code == field)
        .ok_or_else(|| ApiError::NotFound("intake answer revision was not found".to_owned()))?;
    submit_intake_answer(
        db,
        auth,
        session_id,
        SubmitIntakeAnswerRequest {
            field: field.to_owned(),
            answer: revision.raw_answer,
            replace: true,
            structured: None,
        },
        answer_hard_max,
    )
    .await
}

fn static_follow_up(question: IntakeQuestion, missing_fields: Vec<String>) -> IntakeAiFollowUp {
    IntakeAiFollowUp {
        field: question.field,
        prompt: question.prompt,
        purpose: "Collect a missing factual field for human review.".to_owned(),
        missing_fields,
        skippable: !question.required,
    }
}

fn valid_follow_up(
    question: &IntakeAiFollowUp,
    fallback: &IntakeQuestion,
    missing: &[String],
) -> bool {
    question.field == fallback.field
        && question.skippable == !fallback.required
        && missing.contains(&question.field)
        && !question.prompt.trim().is_empty()
        && question.prompt.chars().count() <= 500
        && !question.purpose.trim().is_empty()
        && question.purpose.chars().count() <= 300
        && question.missing_fields.contains(&question.field)
        && question
            .missing_fields
            .iter()
            .all(|field| missing.contains(field))
}

fn normalize_follow_up(mut question: IntakeAiFollowUp) -> IntakeAiFollowUp {
    question.prompt = question.prompt.trim().chars().take(500).collect();
    question.purpose = question.purpose.trim().chars().take(300).collect();
    question.missing_fields.sort();
    question.missing_fields.dedup();
    question
}

fn profile_draft_field_metadata(
    answers: &IntakeInitialAnswers,
    stored_answers: &[intake_session_answers::Model],
    session: &intake_sessions::Model,
) -> Vec<IntakeProfileDraftFieldMetadata> {
    [
        (
            "physical_description",
            "basic_information",
            &answers.basic_information,
        ),
        ("clothing_description", "belongings", &answers.belongings),
        ("health_notes", "health_status", &answers.health_status),
        ("mobility_notes", "health_status", &answers.health_status),
        (
            "transportation_ability",
            "transport_ability",
            &answers.transport_ability,
        ),
        (
            "frequent_locations",
            "frequent_locations",
            &answers.frequent_locations,
        ),
        ("last_seen_information", "last_seen", &answers.last_seen),
        (
            "behavior_habits",
            "behavior_habits",
            &answers.behavior_habits,
        ),
        (
            "suspicious_motive",
            "suspicious_motive",
            &answers.suspicious_motive,
        ),
    ]
    .into_iter()
    .filter_map(|(field, source_field, value)| {
        value.as_ref()?;
        let stored = stored_answers
            .iter()
            .find(|answer| answer.field_code == source_field);
        Some(IntakeProfileDraftFieldMetadata {
            field: field.to_owned(),
            source_field: source_field.to_owned(),
            source: stored
                .map(|answer| answer.source.clone())
                .unwrap_or_else(|| "family_provided".to_owned()),
            status: stored
                .map(|answer| answer.status.clone())
                .unwrap_or_else(|| "draft".to_owned()),
            generated_at: stored
                .map(|answer| answer.generated_at.clone())
                .unwrap_or_else(|| session.created_at.clone()),
            source_excerpt: value.clone(),
            provider_model: stored.and_then(|answer| answer.model.clone()),
            template_version: stored
                .and_then(|answer| answer.template_version.clone())
                .unwrap_or_else(|| "intake-profile-family-draft-v1".to_owned()),
        })
    })
    .collect()
}

pub async fn list_intake_profile_draft_versions(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
) -> Result<IntakeProfileDraftVersionsResponse, ApiError> {
    require_operational_member(auth)?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;
    Ok(IntakeProfileDraftVersionsResponse {
        items: intake_profile_drafts::Entity::find()
            .filter(intake_profile_drafts::Column::SessionId.eq(session_id))
            .order_by_desc(intake_profile_drafts::Column::Version)
            .all(db)
            .await?
            .into_iter()
            .map(profile_draft_from_model)
            .collect::<Result<_, _>>()?,
    })
}

pub async fn diff_intake_profile_drafts(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    from_id: &str,
    to_id: &str,
) -> Result<IntakeProfileDraftDiffResponse, ApiError> {
    let _ = list_intake_profile_draft_versions(db, auth, session_id).await?;
    let from = intake_profile_drafts::Entity::find_by_id(from_id)
        .one(db)
        .await?
        .filter(|item| item.session_id == session_id)
        .ok_or_else(|| ApiError::NotFound("profile draft version was not found".to_owned()))?;
    let from = profile_draft_from_model(from)?;
    let to = intake_profile_drafts::Entity::find_by_id(to_id)
        .one(db)
        .await?
        .filter(|item| item.session_id == session_id)
        .ok_or_else(|| ApiError::NotFound("profile draft version was not found".to_owned()))?;
    let to = profile_draft_from_model(to)?;
    let mut changed_fields = Vec::new();
    for field in [
        "physical_description",
        "clothing_description",
        "health_notes",
        "mobility_notes",
        "transportation_ability",
        "frequent_locations",
        "last_seen_information",
        "behavior_habits",
        "suspicious_motive",
    ] {
        if serde_json::to_value(&from.profile)
            .ok()
            .and_then(|value| value.get(field).cloned())
            != serde_json::to_value(&to.profile)
                .ok()
                .and_then(|value| value.get(field).cloned())
        {
            changed_fields.push(field.to_owned());
        }
    }
    Ok(IntakeProfileDraftDiffResponse {
        from_version: from.version,
        to_version: to.version,
        changed_fields,
    })
}

pub async fn review_intake_profile_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    draft_id: &str,
    request: ReviewIntakeProfileDraftRequest,
) -> Result<IntakeProfileDraft, ApiError> {
    require_operational_member(auth)?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;
    let action = request.action.trim().to_lowercase();
    if !matches!(action.as_str(), "confirm" | "reject") {
        return Err(ApiError::Validation(
            "action must be confirm or reject".to_owned(),
        ));
    }
    validate_review_reason(request.reason)?;
    let draft = intake_profile_drafts::Entity::find_by_id(draft_id)
        .one(db)
        .await?
        .filter(|item| item.session_id == session_id && item.status == "draft")
        .ok_or_else(|| ApiError::Conflict("profile draft is not awaiting review".to_owned()))?;
    let mut active = draft.into_active_model();
    active.status = Set(if action == "confirm" {
        "confirmed".to_owned()
    } else {
        "superseded".to_owned()
    });
    active.confirmed_by_user_id = Set(Some(auth.id.clone()));
    active.confirmed_at = Set(Some(now()));
    let model = active.update(db).await?;
    profile_draft_from_model(model)
}

pub async fn restore_intake_profile_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    request: RestoreIntakeProfileDraftRequest,
) -> Result<IntakeProfileDraft, ApiError> {
    require_operational_member(auth)?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_session_creator(&session, auth)?;
    validate_review_reason(request.reason)?;
    let source = intake_profile_drafts::Entity::find_by_id(&request.draft_id)
        .one(db)
        .await?
        .filter(|item| item.session_id == session_id)
        .ok_or_else(|| ApiError::NotFound("profile draft version was not found".to_owned()))?;
    let version = intake_profile_drafts::Entity::find()
        .filter(intake_profile_drafts::Column::SessionId.eq(session_id))
        .order_by_desc(intake_profile_drafts::Column::Version)
        .one(db)
        .await?
        .map_or(1, |item| item.version + 1);
    let timestamp = now();
    let model = intake_profile_drafts::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        session_id: Set(session_id.to_owned()),
        version: Set(version),
        parent_draft_id: Set(Some(source.id)),
        profile_json: Set(source.profile_json),
        field_metadata_json: Set(source.field_metadata_json),
        status: Set("draft".to_owned()),
        degradation_status: Set("restored_human_review_required".to_owned()),
        provider_model: Set(None),
        template_version: Set("intake-profile-restored-v1".to_owned()),
        generated_at: Set(timestamp.clone()),
        confirmed_by_user_id: Set(None),
        confirmed_at: Set(None),
        created_at: Set(timestamp),
    }
    .insert(db)
    .await?;
    profile_draft_from_model(model)
}

fn validate_review_reason(reason: String) -> Result<(), ApiError> {
    let length = reason.trim().chars().count();
    if !(1..=1_000).contains(&length) {
        return Err(ApiError::Validation(
            "reason must contain 1 to 1000 characters".to_owned(),
        ));
    }
    Ok(())
}

fn profile_draft_from_model(
    model: intake_profile_drafts::Model,
) -> Result<IntakeProfileDraft, ApiError> {
    let requires_human_confirmation = model.status == "draft";
    Ok(IntakeProfileDraft {
        id: model.id,
        status: model.status,
        source_scope: "family_provided intake answers from this session only".to_owned(),
        generated_at: model.generated_at,
        provider_model: model.provider_model,
        template_version: model.template_version,
        degradation_status: model.degradation_status,
        version: model.version,
        requires_human_confirmation,
        profile: serde_json::from_str(&model.profile_json).map_err(|_| ApiError::Internal)?,
        field_metadata: serde_json::from_str(&model.field_metadata_json)
            .map_err(|_| ApiError::Internal)?,
        missing_fields: Vec::new(),
        assessments: Vec::new(),
        confirmation_blocked_reasons: Vec::new(),
        direction_hypotheses: Vec::new(),
    })
}

fn profile_fallback(
    answers: &IntakeInitialAnswers,
    session: &intake_sessions::Model,
) -> (
    IntakeProfileDraftFields,
    Vec<IntakeProfileDraftFieldMetadata>,
    Option<String>,
    String,
    AiTaskStatus,
) {
    let fields = IntakeProfileDraftFields {
        physical_description: answers.basic_information.clone(),
        clothing_description: answers.belongings.clone(),
        health_notes: answers.health_status.clone(),
        mobility_notes: answers.health_status.clone(),
        transportation_ability: answers.transport_ability.clone(),
        frequent_locations: answers.frequent_locations.clone(),
        last_seen_information: answers.last_seen.clone(),
        behavior_habits: answers.behavior_habits.clone(),
        suspicious_motive: answers.suspicious_motive.clone(),
    };
    let metadata = profile_draft_field_metadata(answers, &[], session)
        .into_iter()
        .map(|mut item| {
            item.source = "family_provided_fallback".to_owned();
            item.template_version = "intake-profile-family-fallback-v1".to_owned();
            item
        })
        .collect();
    (
        fields,
        metadata,
        None,
        "rule_based_fallback".to_owned(),
        AiTaskStatus::Degraded,
    )
}

fn profile_extraction_system_instruction() -> String {
    format!(
        "Return exactly one JSON object matching this schema: {}. Do not return a `profile_candidates` object, Markdown, commentary, or any keys not present in the schema. Extract concise profile candidates only from supplied family answers. `field_sources` must include every profile field: use a source object with its source answer field and exact supporting excerpt when the profile value is non-null, otherwise use null. Do not diagnose, confirm facts, infer current or future location, add facts, issue instructions, or change answers.",
        profile_extraction_schema()
    )
}

fn decode_profile_extraction_output(
    output: &str,
    answers: &IntakeInitialAnswers,
) -> Result<(ProfileExtractionOutput, bool), String> {
    if let Ok(canonical) = serde_json::from_str::<ProfileExtractionOutput>(output) {
        return Ok((canonical, false));
    }

    let compatibility = serde_json::from_str::<ProfileCandidatesOutput>(output)
        .map_err(|_| "output did not match the profile schema".to_owned())?;
    Ok((normalize_profile_candidates(compatibility, answers)?, true))
}

fn normalize_profile_candidates(
    output: ProfileCandidatesOutput,
    answers: &IntakeInitialAnswers,
) -> Result<ProfileExtractionOutput, String> {
    let candidates = output.profile_candidates;
    validate_profile_candidate_sources(&candidates, answers)?;
    let (physical_description, physical_source) = combine_profile_candidates(vec![
        ("姓名或称呼", candidates.name),
        ("性别", candidates.gender),
        ("年龄", candidates.age),
        ("身高", candidates.height),
        ("外观特征", candidates.appearance),
    ])?;
    let (clothing_description, clothing_source) =
        profile_field_from_candidate(candidates.belongings);
    let (health_notes, health_source) = profile_field_from_candidate(candidates.health_status);
    let (transportation_ability, transportation_source) =
        profile_field_from_candidate(candidates.transport_ability);
    let (frequent_locations, frequent_locations_source) =
        profile_field_from_candidate(candidates.frequent_locations);
    let (last_seen_information, last_seen_source) =
        profile_field_from_candidate(candidates.last_seen);
    let (behavior_habits, behavior_source) =
        profile_field_from_candidate(candidates.behavior_habit);
    let (suspicious_motive, suspicious_motive_source) =
        profile_field_from_candidate(candidates.suspicious_motive);

    let field_sources = std::collections::BTreeMap::from([
        ("physical_description".to_owned(), physical_source),
        ("clothing_description".to_owned(), clothing_source),
        ("health_notes".to_owned(), health_source),
        ("mobility_notes".to_owned(), None),
        ("transportation_ability".to_owned(), transportation_source),
        ("frequent_locations".to_owned(), frequent_locations_source),
        ("last_seen_information".to_owned(), last_seen_source),
        ("behavior_habits".to_owned(), behavior_source),
        ("suspicious_motive".to_owned(), suspicious_motive_source),
    ]);

    Ok(ProfileExtractionOutput {
        profile: IntakeProfileDraftFields {
            physical_description,
            clothing_description,
            health_notes,
            mobility_notes: None,
            transportation_ability,
            frequent_locations,
            last_seen_information,
            behavior_habits,
            suspicious_motive,
        },
        field_sources,
    })
}

fn combine_profile_candidates(
    candidates: Vec<(&str, Option<ProfileCandidate>)>,
) -> Result<(Option<String>, Option<ProfileFieldSource>), String> {
    let candidates = candidates
        .into_iter()
        .filter_map(|(label, candidate)| candidate.map(|candidate| (label, candidate)))
        .filter(|(_, candidate)| !candidate.value.trim().is_empty())
        .collect::<Vec<_>>();
    let Some(first) = candidates.first() else {
        return Ok((None, None));
    };
    if candidates
        .iter()
        .any(|(_, candidate)| candidate.source_answer_field != first.1.source_answer_field)
    {
        return Err(
            "physical_description candidates must cite the same family answer field".to_owned(),
        );
    }

    let value = candidates
        .iter()
        .map(|(label, candidate)| format!("{label}：{}", candidate.value.trim()))
        .collect::<Vec<_>>()
        .join("；");
    Ok((
        Some(value),
        Some(ProfileFieldSource {
            source_field: first.1.source_answer_field.clone(),
            source_excerpt: first.1.supporting_excerpt.clone(),
        }),
    ))
}

fn validate_profile_candidate_sources(
    candidates: &ProfileCandidates,
    answers: &IntakeInitialAnswers,
) -> Result<(), String> {
    for candidate in [
        &candidates.name,
        &candidates.gender,
        &candidates.age,
        &candidates.height,
        &candidates.appearance,
        &candidates.behavior_habit,
        &candidates.belongings,
        &candidates.frequent_locations,
        &candidates.health_status,
        &candidates.last_seen,
        &candidates.suspicious_motive,
        &candidates.transport_ability,
    ] {
        let Some(candidate) = candidate else {
            continue;
        };
        if candidate.value.trim().is_empty() {
            continue;
        }
        validate_profile_source(
            &candidate.source_answer_field,
            &candidate.supporting_excerpt,
            answers,
        )?;
    }
    Ok(())
}

fn profile_field_from_candidate(
    candidate: Option<ProfileCandidate>,
) -> (Option<String>, Option<ProfileFieldSource>) {
    let Some(candidate) = candidate else {
        return (None, None);
    };
    let value = candidate.value.trim();
    if value.is_empty() {
        return (None, None);
    }
    (
        Some(value.to_owned()),
        Some(ProfileFieldSource {
            source_field: candidate.source_answer_field,
            source_excerpt: candidate.supporting_excerpt,
        }),
    )
}

fn validate_profile_extraction(
    output: ProfileExtractionOutput,
    answers: &IntakeInitialAnswers,
) -> Result<
    (
        IntakeProfileDraftFields,
        Vec<IntakeProfileDraftFieldMetadata>,
    ),
    String,
> {
    let values = [
        ("physical_description", &output.profile.physical_description),
        ("clothing_description", &output.profile.clothing_description),
        ("health_notes", &output.profile.health_notes),
        ("mobility_notes", &output.profile.mobility_notes),
        (
            "transportation_ability",
            &output.profile.transportation_ability,
        ),
        ("frequent_locations", &output.profile.frequent_locations),
        (
            "last_seen_information",
            &output.profile.last_seen_information,
        ),
        ("behavior_habits", &output.profile.behavior_habits),
        ("suspicious_motive", &output.profile.suspicious_motive),
    ];
    let mut metadata = Vec::new();
    for (field, value) in values {
        if value.as_ref().is_some_and(|value| !value.trim().is_empty()) {
            let source = output
                .field_sources
                .get(field)
                .and_then(Option::as_ref)
                .ok_or_else(|| format!("{field} is missing its source citation"))?;
            validate_profile_source(&source.source_field, &source.source_excerpt, answers)
                .map_err(|reason| format!("{field} {reason}"))?;
            metadata.push(IntakeProfileDraftFieldMetadata {
                field: field.to_owned(),
                source_field: source.source_field.clone(),
                source: "ai_extracted".to_owned(),
                status: "draft".to_owned(),
                generated_at: now(),
                source_excerpt: Some(source.source_excerpt.trim().to_owned()),
                provider_model: None,
                template_version: "intake-profile-extraction-v1".to_owned(),
            });
        }
    }
    Ok((output.profile, metadata))
}

fn validate_profile_source(
    source_field: &str,
    source_excerpt: &str,
    answers: &IntakeInitialAnswers,
) -> Result<(), String> {
    if !is_profile_source_field(source_field) || source_excerpt.trim().is_empty() {
        return Err("has an invalid source citation".to_owned());
    }
    let answer = answer_for(answers, source_field)
        .ok_or_else(|| "cites a source field without a family answer".to_owned())?;
    if !answer.contains(source_excerpt.trim()) {
        return Err("source excerpt does not occur in its cited family answer".to_owned());
    }
    Ok(())
}

fn is_profile_source_field(field: &str) -> bool {
    matches!(
        field,
        "basic_information"
            | "belongings"
            | "health_status"
            | "transport_ability"
            | "frequent_locations"
            | "last_seen"
            | "behavior_habits"
            | "suspicious_motive"
    )
}

fn profile_extraction_schema() -> serde_json::Value {
    let fields = [
        "physical_description",
        "clothing_description",
        "health_notes",
        "mobility_notes",
        "transportation_ability",
        "frequent_locations",
        "last_seen_information",
        "behavior_habits",
        "suspicious_motive",
    ];
    let profile_properties = fields
        .iter()
        .map(|field| ((*field).to_owned(), json!({ "type": ["string", "null"] })))
        .collect::<serde_json::Map<_, _>>();
    let field_source = json!({
        "type": ["object", "null"],
        "additionalProperties": false,
        "properties": {
            "source_field": { "type": "string" },
            "source_excerpt": { "type": "string" }
        },
        "required": ["source_field", "source_excerpt"]
    });
    let field_source_properties = fields
        .iter()
        .map(|field| ((*field).to_owned(), field_source.clone()))
        .collect::<serde_json::Map<_, _>>();

    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["profile", "field_sources"],
        "properties": {
            "profile": {
                "type": "object",
                "additionalProperties": false,
                "required": fields,
                "properties": profile_properties
            },
            "field_sources": {
                "type": "object",
                "additionalProperties": false,
                "required": fields,
                "properties": field_source_properties
            }
        }
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
            "human_confirmed must be true before the second confirmation creates a case".to_owned(),
        ));
    }
    let submitted_profile = request.profile.clone();
    let case_request = case_request_from_confirmed_profile(request);
    let transaction = db.begin().await?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .lock_exclusive()
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
    if session.status != "ready_for_second_confirmation" {
        return Err(ApiError::Conflict(
            "complete the initial review and family acknowledgement before the second confirmation"
                .to_owned(),
        ));
    }
    let reviewed_profile = session
        .ai_initial_review_profile_json
        .as_deref()
        .ok_or_else(|| {
            ApiError::Conflict(
                "the initial review profile is unavailable; start initial review again".to_owned(),
            )
        })
        .and_then(|profile| {
            serde_json::from_str::<ConfirmedIntakeProfile>(profile).map_err(|_| ApiError::Internal)
        })?;
    if reviewed_profile != submitted_profile {
        return Err(ApiError::Conflict(
            "profile values changed after initial review; review the updated information again"
                .to_owned(),
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

    let photos = intake_session_photos::Entity::find()
        .filter(intake_session_photos::Column::SessionId.eq(session_id))
        .all(&transaction)
        .await?;
    if session.question_set_version >= 3 && photos.is_empty() {
        return Err(ApiError::Conflict(
            "upload at least one missing-person photo before confirming the case".to_owned(),
        ));
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
    for photo in photos {
        case_attachments::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            case_id: Set(case_model.id.clone()),
            storage_key: Set(photo.storage_key),
            original_filename: Set(photo.original_filename),
            content_type: Set(photo.content_type),
            byte_size: Set(photo.byte_size),
            sha256: Set(photo.sha256),
            source: Set("family".to_owned()),
            review_status: Set("pending_review".to_owned()),
            created_by_user_id: Set(auth.id.clone()),
            created_at: Set(timestamp.clone()),
            updated_at: Set(timestamp.clone()),
        }
        .insert(&transaction)
        .await?;
    }
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
        Some(json!({
            "case_id": case_model.id,
            "confirmation_status": "human_confirmed_after_ai_initial_review"
        })),
    )
    .await?;

    transaction.commit().await?;
    Ok(confirmed_response(case_model, Some(timestamp)))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InitialReviewModelOutput {
    issues: Vec<InitialReviewModelIssue>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InitialReviewModelIssue {
    field: String,
    severity: String,
    evidence_summary: String,
    clarification_question: String,
    source_fields: Vec<String>,
}

fn initial_review_response(
    session: &intake_sessions::Model,
) -> Result<IntakeAiInitialReviewResponse, ApiError> {
    let issues = parse_initial_review_issues(session)?;
    let blocking_assessments = parse_assessments(session)?
        .into_iter()
        .filter(|assessment| assessment.severity == "blocking")
        .collect::<Vec<_>>();
    let degradation_status = if session
        .ai_initial_review_status
        .starts_with("rule_based_fallback")
    {
        "rule_based_fallback"
    } else if session.ai_initial_review_status == "not_started" {
        "not_started"
    } else {
        "available"
    };
    Ok(IntakeAiInitialReviewResponse {
        session_id: session.id.clone(),
        status: session.status.clone(),
        degradation_status: degradation_status.to_owned(),
        issues,
        blocking_assessments,
        generated_at: session
            .ai_initial_reviewed_at
            .clone()
            .unwrap_or_else(|| session.updated_at.clone()),
        requires_family_acknowledgement: session.status == "awaiting_family_review",
        ready_for_second_confirmation: session.status == "ready_for_second_confirmation",
    })
}

fn parse_initial_review_issues(
    session: &intake_sessions::Model,
) -> Result<Vec<IntakeAiInitialReviewIssue>, ApiError> {
    serde_json::from_str(&session.ai_initial_review_json).map_err(|_| ApiError::Internal)
}

fn acknowledged_review_status(status: &str) -> String {
    match status {
        "available_pending" => "available_acknowledged".to_owned(),
        "rule_based_fallback_pending" => "rule_based_fallback_acknowledged".to_owned(),
        other => other.to_owned(),
    }
}

fn validate_initial_review_output(
    output: InitialReviewModelOutput,
) -> Result<Vec<IntakeAiInitialReviewIssue>, ApiError> {
    if output.issues.len() > 12 {
        return Err(ApiError::Validation(
            "initial review returned too many issues".to_owned(),
        ));
    }
    let allowed_fields = [
        "basic_information",
        "health_status",
        "behavior_habits",
        "last_seen",
        "frequent_locations",
        "belongings",
        "transport_ability",
        "follow_up_clues",
        "suspicious_motive",
        "profile",
    ]
    .into_iter()
    .collect::<std::collections::HashSet<_>>();
    let mut seen = std::collections::HashSet::new();
    output
        .issues
        .into_iter()
        .map(|candidate| {
            let field = candidate.field.trim().to_owned();
            let severity = candidate.severity.trim().to_owned();
            let evidence_summary = candidate.evidence_summary.trim().to_owned();
            let clarification_question = candidate.clarification_question.trim().to_owned();
            let mut source_fields = candidate
                .source_fields
                .into_iter()
                .map(|value| value.trim().to_owned())
                .collect::<Vec<_>>();
            source_fields.sort();
            source_fields.dedup();
            if !allowed_fields.contains(field.as_str())
                || !matches!(severity.as_str(), "needs_confirmation" | "warning")
                || evidence_summary.is_empty()
                || evidence_summary.chars().count() > 360
                || clarification_question.is_empty()
                || clarification_question.chars().count() > 300
                || source_fields.is_empty()
                || source_fields.len() > 4
                || source_fields
                    .iter()
                    .any(|source| !allowed_fields.contains(source.as_str()))
            {
                return Err(ApiError::Validation(
                    "initial review output did not match the approved schema".to_owned(),
                ));
            }
            let fingerprint = format!(
                "{field}|{severity}|{evidence_summary}|{clarification_question}|{}",
                source_fields.join("|")
            );
            if !seen.insert(fingerprint) {
                return Err(ApiError::Validation(
                    "initial review output contained duplicate issues".to_owned(),
                ));
            }
            Ok(IntakeAiInitialReviewIssue {
                id: Uuid::new_v4().to_string(),
                field,
                severity,
                evidence_summary,
                clarification_question,
                source_fields,
            })
        })
        .collect()
}

fn rule_based_initial_review_issues(
    assessments: &[crate::models::IntakeAssessment],
) -> Vec<IntakeAiInitialReviewIssue> {
    assessments
        .iter()
        .filter(|assessment| assessment.severity != "blocking")
        .take(12)
        .map(|assessment| IntakeAiInitialReviewIssue {
            id: Uuid::new_v4().to_string(),
            field: assessment.field_path.clone(),
            severity: "needs_confirmation".to_owned(),
            evidence_summary: assessment.evidence_summary.clone(),
            clarification_question: assessment.suggested_action.clone(),
            source_fields: vec![assessment.field_path.clone()],
        })
        .collect()
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

fn can_request_ai_follow_up(status: &str) -> bool {
    matches!(status, "collecting" | "ready_for_confirmation")
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

fn question_phase(field: &str, question_set_version: i32) -> &'static str {
    let phase_one_fields = if question_set_version >= 3 {
        [
            "basic_information",
            "last_seen",
            "suspicious_motive",
            "police_report_status",
            "family_phone",
        ]
    } else {
        [
            "basic_information",
            "health_status",
            "behavior_habits",
            "last_seen",
            "",
        ]
    };
    if phase_one_fields.contains(&field) {
        "phase_one"
    } else {
        "phase_two"
    }
}

fn phase_two_answers_present(answers: &IntakeInitialAnswers, question_set_version: i32) -> bool {
    if question_set_version >= 3 {
        answers.health_status.is_some()
            || answers.behavior_habits.is_some()
            || answers.frequent_locations.is_some()
            || answers.belongings.is_some()
            || answers.transport_ability.is_some()
            || answers.follow_up_clues.is_some()
    } else {
        answers.frequent_locations.is_some()
            || answers.belongings.is_some()
            || answers.transport_ability.is_some()
            || answers.follow_up_clues.is_some()
            || answers.suspicious_motive.is_some()
    }
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
        confirmation_status: "human_confirmed_after_ai_initial_review".to_owned(),
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

fn append_content_quality_issues(
    issues: &mut Vec<IntakeAiInitialReviewIssue>,
    answers: &IntakeInitialAnswers,
) {
    for field in [
        "basic_information",
        "last_seen",
        "suspicious_motive",
        "belongings",
    ] {
        if issues.len() >= 12
            || !answer_for(answers, field).is_some_and(|value| {
                low_information_text(value)
                    || (field == "basic_information"
                        && basic_information_details(value)
                            .iter()
                            .any(|detail| low_information_text(detail)))
            })
        {
            continue;
        }
        issues.push(IntakeAiInitialReviewIssue {
            id: Uuid::new_v4().to_string(),
            field: field.to_owned(),
            severity: "needs_confirmation".to_owned(),
            evidence_summary: "该项看起来像占位符、纯数字、重复字符或信息不足的描述。".to_owned(),
            clarification_question: "请补充可由家属核实的具体信息；不清楚时请明确填写“不清楚”。"
                .to_owned(),
            source_fields: vec![field.to_owned()],
        });
    }
}

fn low_information_text(value: &str) -> bool {
    let normalized = value.trim().to_lowercase();
    if matches!(
        normalized.as_str(),
        "123" | "456" | "test" | "测试" | "未知"
    ) {
        return true;
    }
    let characters = normalized
        .chars()
        .filter(|value| !value.is_whitespace())
        .collect::<Vec<_>>();
    characters.len() < 3
        || characters.iter().all(|value| value.is_ascii_digit())
        || characters.windows(2).all(|pair| pair[0] == pair[1])
}

/// Returns the answer subset that can be sent to an AI provider. Family phone
/// numbers stay in the owner-scoped intake record and never enter AI requests,
/// execution events, or AI audit records.
fn answers_for_ai(answers: &IntakeInitialAnswers) -> IntakeInitialAnswers {
    let mut minimized = answers.clone();
    minimized.family_phone = None;
    minimized
}

fn basic_information_details(value: &str) -> Vec<&str> {
    value
        .split(['\n', ';', '；'])
        .filter_map(|segment| {
            let (label, detail) = segment.split_once(['：', ':'])?;
            matches!(
                label.trim(),
                "姓名" | "姓名或称呼" | "外观特征" | "特征描述"
            )
            .then_some(detail.trim())
        })
        .collect()
}

/// Records only attachment identifiers and MIME metadata; image contents,
/// filenames, phone numbers, and other report values never enter audit data.
pub(crate) async fn write_attachment_audit<C: ConnectionTrait>(
    db: &C,
    auth: &AuthenticatedUser,
    action: &str,
    session_id: &str,
    photo_id: &str,
    content_type: &str,
) -> Result<(), ApiError> {
    write_audit(
        db,
        auth,
        action,
        session_id.to_owned(),
        Some(json!({ "photo_id": photo_id, "content_type": content_type })),
    )
    .await
}

fn normalize_answers(
    mut answers: IntakeInitialAnswers,
    questions: &[intake_question_definitions::Model],
    answer_hard_max: usize,
) -> Result<IntakeInitialAnswers, ApiError> {
    for (field_code, answer) in [
        ("basic_information", &mut answers.basic_information),
        ("police_report_status", &mut answers.police_report_status),
        ("family_phone", &mut answers.family_phone),
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
        validate_report_field(field_code, trimmed, question.version >= 3)?;
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
    let phase = IntakePhaseProgress::for_answers(&answers, model.question_set_version);
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
                && question_phase(&question.field_code, question.version) == wanted_phase
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

/// AI may phrase the current next intake question, but it cannot change the
/// deterministic question order, requiredness, or phase transition rules.
fn next_follow_up_question(
    questions: &[intake_question_definitions::Model],
    missing_fields: &[String],
    phase: &IntakePhaseProgress,
) -> Option<IntakeQuestion> {
    next_question_for_phase(questions, missing_fields, phase)
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
        "police_report_status" => answers.police_report_status.as_ref(),
        "family_phone" => answers.family_phone.as_ref(),
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
        "police_report_status" => answers.police_report_status = Some(answer),
        "family_phone" => answers.family_phone = Some(answer),
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

fn validate_report_field(
    field: &str,
    value: &str,
    require_report_details: bool,
) -> Result<(), ApiError> {
    match field {
        "basic_information" if require_report_details => validate_basic_information_details(value),
        "police_report_status" if !matches!(value, "已报警" | "未报警" | "不清楚") => {
            Err(ApiError::Validation(
                "police_report_status must be 已报警, 未报警, or 不清楚".to_owned(),
            ))
        }
        "family_phone" => {
            let digits = value.chars().filter(char::is_ascii_digit).count();
            if !(7..=15).contains(&digits)
                || value
                    .chars()
                    .any(|ch| !ch.is_ascii_digit() && !matches!(ch, '+' | '-' | ' ' | '(' | ')'))
            {
                return Err(ApiError::Validation(
                    "family_phone must be a valid phone number".to_owned(),
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_basic_information_details(value: &str) -> Result<(), ApiError> {
    let mut name_present = false;
    let mut height = None;
    let mut appearance_present = false;
    for segment in value.split(['\n', ';', '；']) {
        let Some((label, detail)) = segment.split_once(['：', ':']) else {
            continue;
        };
        let detail = detail.trim();
        match label.trim() {
            "姓名" | "姓名或称呼" => name_present = !detail.is_empty(),
            "身高" => {
                height = detail
                    .chars()
                    .skip_while(|character| !character.is_ascii_digit())
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse::<u16>()
                    .ok();
            }
            "外观特征" | "特征描述" => appearance_present = !detail.is_empty(),
            _ => {}
        }
    }
    if !name_present || !matches!(height, Some(30..=250)) || !appearance_present {
        return Err(ApiError::Validation(
            "basic_information must include a name, height between 30 and 250 cm, and an appearance description"
                .to_owned(),
        ));
    }
    Ok(())
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
                        | "police_report_status"
                        | "family_phone"
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

fn initial_review_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "issues": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "field": { "type": "string" },
                        "severity": { "type": "string", "enum": ["needs_confirmation", "warning"] },
                        "evidence_summary": { "type": "string" },
                        "clarification_question": { "type": "string" },
                        "source_fields": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["field", "severity", "evidence_summary", "clarification_question", "source_fields"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["issues"],
        "additionalProperties": false
    })
}

fn follow_up_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "field": { "type": "string" },
            "prompt": { "type": "string" },
            "purpose": { "type": "string" },
            "missing_fields": { "type": "array", "items": { "type": "string" } },
            "skippable": { "type": "boolean" }
        },
        "required": ["field", "prompt", "purpose", "missing_fields", "skippable"],
        "additionalProperties": false
    })
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::{
        InitialReviewModelIssue, InitialReviewModelOutput, IntakeAiFollowUp, IntakeInitialAnswers,
        IntakeQuestion, answers_for_ai, append_content_quality_issues, can_request_ai_follow_up,
        decode_profile_extraction_output, profile_extraction_schema,
        profile_extraction_system_instruction, valid_follow_up, validate_basic_information_details,
        validate_initial_review_output, validate_profile_extraction,
    };
    fn answers() -> IntakeInitialAnswers {
        IntakeInitialAnswers {
            basic_information: Some(
                "姓名或称呼：dfw\n性别：男\n年龄：70 岁\n身高：170 厘米\n外观特征：戴帽子"
                    .to_owned(),
            ),
            health_status: Some("依赖吸氧机".to_owned()),
            behavior_habits: Some("和人谈论下象棋".to_owned()),
            last_seen: Some("家门口".to_owned()),
            ..IntakeInitialAnswers::default()
        }
    }

    #[test]
    fn normalizes_observed_profile_candidates_provider_shape() {
        let answers = answers();
        let raw = r#"{
            "profile_candidates": {
                "name": {"value":"dfw","source_answer_field":"basic_information","supporting_excerpt":"姓名或称呼：dfw"},
                "gender": {"value":"男","source_answer_field":"basic_information","supporting_excerpt":"性别：男"},
                "age": {"value":"70 岁","source_answer_field":"basic_information","supporting_excerpt":"年龄：70 岁"},
                "height": {"value":"170 厘米","source_answer_field":"basic_information","supporting_excerpt":"身高：170 厘米"},
                "appearance": {"value":"戴帽子","source_answer_field":"basic_information","supporting_excerpt":"外观特征：戴帽子"},
                "behavior_habit": {"value":"和人谈论下象棋","source_answer_field":"behavior_habits","supporting_excerpt":"和人谈论下象棋"},
                "belongings": null,
                "follow_up_clues": null,
                "frequent_locations": null,
                "health_status": {"value":"依赖吸氧机","source_answer_field":"health_status","supporting_excerpt":"依赖吸氧机"},
                "last_seen": {"value":"家门口","source_answer_field":"last_seen","supporting_excerpt":"家门口"},
                "suspicious_motive": null,
                "transport_ability": null
            }
        }"#;

        let (output, used_compatibility_shape) =
            decode_profile_extraction_output(raw, &answers).expect("provider shape normalizes");
        let (profile, metadata) = validate_profile_extraction(output, &answers)
            .expect("normalized values retain provenance");

        assert!(used_compatibility_shape);
        assert_eq!(
            profile.physical_description.as_deref(),
            Some("姓名或称呼：dfw；性别：男；年龄：70 岁；身高：170 厘米；外观特征：戴帽子")
        );
        assert_eq!(profile.health_notes.as_deref(), Some("依赖吸氧机"));
        assert_eq!(profile.behavior_habits.as_deref(), Some("和人谈论下象棋"));
        assert_eq!(metadata.len(), 4);
        assert!(metadata.iter().all(|field| field.source == "ai_extracted"));
    }

    #[test]
    fn rejects_provider_candidates_with_an_unverifiable_source_excerpt() {
        let answers = answers();
        let raw = r#"{
            "profile_candidates": {
                "name": {"value":"dfw","source_answer_field":"basic_information","supporting_excerpt":"not in the family answer"}
            }
        }"#;

        let error = match decode_profile_extraction_output(raw, &answers) {
            Ok(_) => panic!("an unverified source excerpt must not be accepted"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            "source excerpt does not occur in its cited family answer"
        );
    }

    #[test]
    fn profile_extraction_prompt_repeats_the_canonical_contract() {
        let instruction = profile_extraction_system_instruction();
        assert!(instruction.contains("\"profile\""));
        assert!(instruction.contains("\"field_sources\""));
        assert!(instruction.contains("Do not return a `profile_candidates` object"));
    }

    #[test]
    fn profile_extraction_schema_uses_fixed_strict_source_fields() {
        let schema = profile_extraction_schema();
        let field_sources = &schema["properties"]["field_sources"];
        assert_eq!(field_sources["additionalProperties"], false);
        assert_eq!(field_sources["required"].as_array().map(Vec::len), Some(9));
        assert_eq!(
            field_sources["properties"]["physical_description"]["type"],
            serde_json::json!(["object", "null"])
        );
    }

    #[test]
    fn initial_review_accepts_configured_fields_that_are_still_unanswered() {
        let output = InitialReviewModelOutput {
            issues: vec![
                InitialReviewModelIssue {
                    field: "profile".to_owned(),
                    severity: "warning".to_owned(),
                    evidence_summary: "Clothing and belongings are still missing.".to_owned(),
                    clarification_question: "Can you add clothing or belongings?".to_owned(),
                    source_fields: vec!["basic_information".to_owned(), "belongings".to_owned()],
                },
                InitialReviewModelIssue {
                    field: "transport_ability".to_owned(),
                    severity: "warning".to_owned(),
                    evidence_summary: "Travel ability is still missing.".to_owned(),
                    clarification_question: "Can you add travel ability?".to_owned(),
                    source_fields: vec!["transport_ability".to_owned()],
                },
            ],
        };

        let issues = validate_initial_review_output(output)
            .expect("configured unanswered fields are valid completeness sources");
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[1].field, "transport_ability");
    }

    #[test]
    fn initial_review_rejects_unknown_source_fields() {
        let output = InitialReviewModelOutput {
            issues: vec![InitialReviewModelIssue {
                field: "profile".to_owned(),
                severity: "warning".to_owned(),
                evidence_summary: "Unknown field.".to_owned(),
                clarification_question: "Please confirm.".to_owned(),
                source_fields: vec!["unrecognized_field".to_owned()],
            }],
        };

        assert!(validate_initial_review_output(output).is_err());
    }

    #[test]
    fn removes_family_phone_from_every_ai_answer_snapshot() {
        let answers = IntakeInitialAnswers {
            family_phone: Some("+86 138-0000-0000".to_owned()),
            basic_information: Some("姓名：虚构老人；身高：168 厘米；特征描述：戴帽子".to_owned()),
            ..IntakeInitialAnswers::default()
        };

        let minimized = answers_for_ai(&answers);
        assert_eq!(minimized.family_phone, None);
        assert_eq!(minimized.basic_information, answers.basic_information);
    }

    #[test]
    fn v3_basic_information_requires_name_height_and_appearance() {
        assert!(
            validate_basic_information_details("姓名：虚构老人；身高：168 厘米；特征描述：123")
                .is_ok()
        );
        assert!(validate_basic_information_details("姓名：虚构老人；身高：168 厘米").is_err());
        assert!(
            validate_basic_information_details("姓名：虚构老人；身高：280 厘米；特征描述：戴帽子")
                .is_err()
        );
        assert!(
            validate_basic_information_details(
                "姓名：虚构老人；身高：168 厘米，鞋码 42；特征描述：戴帽子"
            )
            .is_ok()
        );
        assert!(
            validate_basic_information_details("姓名：虚构老人；身高：1.68 米；特征描述：戴帽子")
                .is_err()
        );
    }

    #[test]
    fn initial_review_flags_low_information_appearance_without_rewriting_it() {
        let answers = IntakeInitialAnswers {
            basic_information: Some("姓名：虚构老人；身高：168 厘米；特征描述：123".to_owned()),
            ..IntakeInitialAnswers::default()
        };
        let mut issues = Vec::new();

        append_content_quality_issues(&mut issues, &answers);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].field, "basic_information");
        assert!(issues[0].evidence_summary.contains("信息不足"));
    }

    #[test]
    fn follow_up_must_preserve_the_current_question_and_requiredness() {
        let fallback = IntakeQuestion {
            field: "frequent_locations".to_owned(),
            prompt: "Which places are frequently visited?".to_owned(),
            required: false,
        };
        let missing = vec!["frequent_locations".to_owned()];
        let valid = IntakeAiFollowUp {
            field: "frequent_locations".to_owned(),
            prompt: "Which places are frequently visited?".to_owned(),
            purpose: "Collect an optional location for human review.".to_owned(),
            missing_fields: missing.clone(),
            skippable: true,
        };

        assert!(valid_follow_up(&valid, &fallback, &missing));

        let mut wrong_requiredness = valid.clone();
        wrong_requiredness.skippable = false;
        assert!(!valid_follow_up(&wrong_requiredness, &fallback, &missing));

        let mut unknown = valid;
        unknown.field = "family_phone".to_owned();
        assert!(!valid_follow_up(&unknown, &fallback, &missing));

        let required_fallback = IntakeQuestion {
            field: "last_seen".to_owned(),
            prompt: "When was the person last seen?".to_owned(),
            required: true,
        };
        let required_missing = vec!["last_seen".to_owned()];
        let required = IntakeAiFollowUp {
            field: "last_seen".to_owned(),
            prompt: "When and where was the person last seen?".to_owned(),
            purpose: "Collect the required last-seen information.".to_owned(),
            missing_fields: required_missing.clone(),
            skippable: false,
        };
        assert!(valid_follow_up(
            &required,
            &required_fallback,
            &required_missing
        ));
    }

    #[test]
    fn follow_up_is_available_only_while_collecting_or_ready_for_confirmation() {
        assert!(can_request_ai_follow_up("collecting"));
        assert!(can_request_ai_follow_up("ready_for_confirmation"));
        assert!(!can_request_ai_follow_up("awaiting_family_review"));
        assert!(!can_request_ai_follow_up("ready_for_second_confirmation"));
        assert!(!can_request_ai_follow_up("confirmed"));
        assert!(!can_request_ai_follow_up("closed"));
    }
}
