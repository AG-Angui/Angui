use chrono::{SecondsFormat, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::json;

use crate::{
    ai_gateway::{
        AiCapability, AiExecutionAudit, AiExecutionResult, AiPurpose, AiRequest, AiTaskStatus,
        DataLevel,
    },
    amap_service::{Coordinate, PoiSearch},
    entities::{archive_drafts, case_places, cases, clue_drafts, clues, summary_drafts, tasks},
    error::ApiError,
    models::{
        ArchiveDraftResponse, AuthenticatedUser, CasePoiItem, CasePoiQuery, CasePoiResponse,
        CasePublicProgressItem, CasePublicProgressResponse, ClueDraftCandidate,
        ClueDraftFieldDecision, ClueDraftResponse, CreateClueDraftRequest, CreateClueRequest,
        CreateSummaryDraftRequest, DeidentifyArchiveDraftRequest, ReviewArchiveDraftRequest,
        ReviewClueDraftRequest, ReviewSummaryDraftRequest, SummaryDraftDiffResponse,
        SummaryDraftResponse, SummaryDraftVersionResponse,
    },
    roles::{CaseRole, GlobalCapability},
    services::{case_service, case_summary_service, task_service},
};

const DRAFT_TEMPLATE_VERSION: &str = "case-summary-rule-v1";
const ARCHIVE_DRAFT_TEMPLATE_VERSION: &str = "case-archive-safe-metadata-v1";

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchiveOrganizationCandidate {
    timeline: Vec<String>,
    lessons: Vec<String>,
    uncertainty: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SummaryCandidate {
    content: String,
}

pub async fn create_archive_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    gateway: &crate::ai_gateway::AiGateway,
) -> Result<ArchiveDraftResponse, ApiError> {
    let transaction = db.begin().await?;
    case_service::require_case_role(&transaction, &auth.id, case_id, &[CaseRole::Commander])
        .await?;
    let case = cases::Entity::find_by_id(case_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("case was not found".to_owned()))?;
    if !matches!(case.status.as_str(), "resolved" | "closed") {
        return Err(ApiError::Conflict(
            "archive drafts can only be created for resolved or closed cases".to_owned(),
        ));
    }

    let confirmed_clue_count = clues::Entity::find()
        .filter(clues::Column::CaseId.eq(case_id))
        .filter(clues::Column::Status.eq("confirmed"))
        .count(&transaction)
        .await?;
    let completed_task_count = tasks::Entity::find()
        .filter(tasks::Column::CaseId.eq(case_id))
        .filter(tasks::Column::Status.eq("completed"))
        .count(&transaction)
        .await?;
    let source_scope = vec![
        "confirmed_clue_metadata".to_owned(),
        "completed_task_metadata".to_owned(),
    ];
    let ai_request = AiRequest {
        capability: AiCapability::CaseOrganization,
        data_level: DataLevel::Internal,
        purpose: AiPurpose::CaseArchiveDraft,
        data_region: "CN".to_owned(),
        system_instruction: Some("Return JSON only: {timeline:string[],lessons:string[],uncertainty:string}. Use only supplied aggregate metadata. Do not infer persons, locations, health information, causes, or operational outcomes.".to_owned()),
        output_schema: Some(archive_candidate_schema()),
        output_schema_name: Some("case_archive_candidate".to_owned()),
        input: serde_json::to_string(&json!({
            "case_status": case.status,
            "confirmed_clue_count": confirmed_clue_count,
            "completed_task_count": completed_task_count,
            "source_scope": source_scope,
        })).map_err(|_| ApiError::Internal)?,
        requested_output_tokens: 500,
        template_version: "case-archive-ai-v1".to_owned(),
        input_scope_reference: "deidentified_aggregate_case_metadata".to_owned(),
        redaction_policy_version: "archive-aggregate-only-v1".to_owned(),
    };
    let execution = gateway.execute(&ai_request).await;
    let decision = execution.decision();
    let (content, provider_model, template_version, audit_status) = match execution {
        AiExecutionResult::Completed { route, output } => {
            match gateway.decode_json::<ArchiveOrganizationCandidate>(&output) {
                Ok(candidate) if valid_archive_candidate(&candidate) => (
                    archive_candidate_content(candidate),
                    Some(route.model),
                    "case-archive-ai-v1".to_owned(),
                    AiTaskStatus::Completed,
                ),
                _ => (
                    deterministic_archive_draft_content(
                        &case.status,
                        confirmed_clue_count,
                        completed_task_count,
                    ),
                    None,
                    ARCHIVE_DRAFT_TEMPLATE_VERSION.to_owned(),
                    AiTaskStatus::Failed,
                ),
            }
        }
        AiExecutionResult::Degraded { .. } => (
            deterministic_archive_draft_content(
                &case.status,
                confirmed_clue_count,
                completed_task_count,
            ),
            None,
            ARCHIVE_DRAFT_TEMPLATE_VERSION.to_owned(),
            AiTaskStatus::Degraded,
        ),
        AiExecutionResult::Failed { .. } => (
            deterministic_archive_draft_content(
                &case.status,
                confirmed_clue_count,
                completed_task_count,
            ),
            None,
            ARCHIVE_DRAFT_TEMPLATE_VERSION.to_owned(),
            AiTaskStatus::Failed,
        ),
    };
    let execution_audit = AiExecutionAudit::for_request(&ai_request, &decision, audit_status);
    let timestamp = now();
    let model = archive_drafts::ActiveModel {
        id: Set(case_service::new_id()),
        case_id: Set(case_id.to_owned()),
        status: Set("draft".to_owned()),
        content: Set(content),
        source_scope_json: Set(
            serde_json::to_string(&source_scope).map_err(|_| ApiError::Internal)?
        ),
        deidentification_status: Set("manual_review_required".to_owned()),
        template_version: Set(template_version),
        provider_model: Set(provider_model),
        created_by_user_id: Set(auth.id.clone()),
        deidentified_by_user_id: Set(None),
        deidentified_at: Set(None),
        deidentification_reason: Set(None),
        reviewed_by_user_id: Set(None),
        reviewed_at: Set(None),
        review_reason: Set(None),
        version: Set(1),
        usage_scope: Set("internal_archive".to_owned()),
        retention_status: Set("retained".to_owned()),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp),
    }
    .insert(&transaction)
    .await?;
    crate::ai_gateway::persist_execution_audit(
        &transaction,
        &execution_audit,
        &auth.id,
        Some(case_id),
    )
    .await?;
    case_service::write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "archive_draft.created",
        "archive_draft",
        model.id.clone(),
        Some(json!({
            "status": model.status,
            "deidentification_status": model.deidentification_status,
            "template_version": model.template_version,
            "source_scope": source_scope,
            "confirmed_clue_count": confirmed_clue_count,
            "completed_task_count": completed_task_count,
        })),
    )
    .await?;
    transaction.commit().await?;
    archive_draft_response(model)
}

pub async fn deidentify_archive_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    draft_id: &str,
    request: DeidentifyArchiveDraftRequest,
) -> Result<ArchiveDraftResponse, ApiError> {
    require_admin(auth)?;
    let outcome = request.outcome.trim().to_lowercase();
    if !matches!(outcome.as_str(), "confirm" | "reject") {
        return Err(ApiError::Validation(
            "outcome must be confirm or reject".to_owned(),
        ));
    }
    let reason = required_text("reason", request.reason, 1_000)?;
    let transaction = db.begin().await?;
    let existing = archive_drafts::Entity::find_by_id(draft_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("archive draft was not found".to_owned()))?;
    if existing.status != "draft" || existing.deidentification_status != "manual_review_required" {
        return Err(ApiError::Conflict(
            "archive draft is not awaiting de-identification review".to_owned(),
        ));
    }
    let next_version = existing.version.checked_add(1).ok_or(ApiError::Internal)?;
    let timestamp = now();
    let next_deidentification_status = if outcome == "confirm" {
        "deidentified"
    } else {
        "rejected"
    };
    let next_status = if outcome == "confirm" {
        "pending_review"
    } else {
        "rejected"
    };
    let update = archive_drafts::Entity::update_many()
        .col_expr(archive_drafts::Column::Status, Expr::value(next_status))
        .col_expr(
            archive_drafts::Column::DeidentificationStatus,
            Expr::value(next_deidentification_status),
        )
        .col_expr(
            archive_drafts::Column::DeidentifiedByUserId,
            Expr::value(Some(auth.id.clone())),
        )
        .col_expr(
            archive_drafts::Column::DeidentifiedAt,
            Expr::value(Some(timestamp.clone())),
        )
        .col_expr(
            archive_drafts::Column::DeidentificationReason,
            Expr::value(Some(reason.clone())),
        )
        .col_expr(archive_drafts::Column::Version, Expr::value(next_version))
        .col_expr(archive_drafts::Column::UpdatedAt, Expr::value(timestamp))
        .filter(archive_drafts::Column::Id.eq(draft_id))
        .filter(archive_drafts::Column::Version.eq(existing.version))
        .filter(archive_drafts::Column::Status.eq(&existing.status))
        .filter(
            archive_drafts::Column::DeidentificationStatus.eq(&existing.deidentification_status),
        )
        .exec(&transaction)
        .await?;
    if update.rows_affected != 1 {
        return Err(ApiError::Conflict(
            "archive draft changed before de-identification could be recorded".to_owned(),
        ));
    }
    let model = archive_drafts::Entity::find_by_id(draft_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::Internal)?;
    case_service::write_audit(
        &transaction,
        Some(model.case_id.clone()),
        auth,
        "archive_draft.deidentification_reviewed",
        "archive_draft",
        model.id.clone(),
        Some(json!({
            "outcome": outcome,
            "status": model.status,
            "deidentification_status": model.deidentification_status,
            "reason_length": reason.chars().count(),
            "version": model.version,
        })),
    )
    .await?;
    transaction.commit().await?;
    archive_draft_response(model)
}

pub async fn review_archive_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    draft_id: &str,
    request: ReviewArchiveDraftRequest,
) -> Result<ArchiveDraftResponse, ApiError> {
    require_admin(auth)?;
    let action = request.action.trim().to_lowercase();
    if !matches!(action.as_str(), "publish" | "reject" | "withdraw") {
        return Err(ApiError::Validation(
            "action must be publish, reject, or withdraw".to_owned(),
        ));
    }
    let reason = required_text("reason", request.reason, 1_000)?;
    let transaction = db.begin().await?;
    let existing = archive_drafts::Entity::find_by_id(draft_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("archive draft was not found".to_owned()))?;
    let transition_allowed = matches!(
        (existing.status.as_str(), action.as_str()),
        ("pending_review", "publish" | "reject") | ("published", "withdraw")
    );
    if !transition_allowed {
        return Err(ApiError::Conflict(
            "archive draft cannot transition from its current status".to_owned(),
        ));
    }
    if action == "publish" && existing.deidentification_status != "deidentified" {
        return Err(ApiError::Conflict(
            "archive draft must have confirmed de-identification before publication".to_owned(),
        ));
    }
    let next_version = existing.version.checked_add(1).ok_or(ApiError::Internal)?;
    let timestamp = now();
    let next_status = match action.as_str() {
        "publish" => "published",
        "reject" => "rejected",
        "withdraw" => "withdrawn",
        _ => return Err(ApiError::Internal),
    };
    let next_usage_scope = if action == "publish" {
        "learning_resource"
    } else {
        "internal_archive"
    };
    let next_retention_status = if action == "withdraw" {
        "withdrawn"
    } else {
        "retained"
    };
    let update = archive_drafts::Entity::update_many()
        .col_expr(archive_drafts::Column::Status, Expr::value(next_status))
        .col_expr(
            archive_drafts::Column::ReviewedByUserId,
            Expr::value(Some(auth.id.clone())),
        )
        .col_expr(
            archive_drafts::Column::ReviewedAt,
            Expr::value(Some(timestamp.clone())),
        )
        .col_expr(
            archive_drafts::Column::ReviewReason,
            Expr::value(Some(reason.clone())),
        )
        .col_expr(
            archive_drafts::Column::UsageScope,
            Expr::value(next_usage_scope),
        )
        .col_expr(
            archive_drafts::Column::RetentionStatus,
            Expr::value(next_retention_status),
        )
        .col_expr(archive_drafts::Column::Version, Expr::value(next_version))
        .col_expr(archive_drafts::Column::UpdatedAt, Expr::value(timestamp))
        .filter(archive_drafts::Column::Id.eq(draft_id))
        .filter(archive_drafts::Column::Version.eq(existing.version))
        .filter(archive_drafts::Column::Status.eq(&existing.status))
        .filter(
            archive_drafts::Column::DeidentificationStatus.eq(&existing.deidentification_status),
        )
        .exec(&transaction)
        .await?;
    if update.rows_affected != 1 {
        return Err(ApiError::Conflict(
            "archive draft changed before review could be recorded".to_owned(),
        ));
    }
    let model = archive_drafts::Entity::find_by_id(draft_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::Internal)?;
    case_service::write_audit(
        &transaction,
        Some(model.case_id.clone()),
        auth,
        "archive_draft.reviewed",
        "archive_draft",
        model.id.clone(),
        Some(json!({
            "action": action,
            "status": model.status,
            "usage_scope": model.usage_scope,
            "retention_status": model.retention_status,
            "reason_length": reason.chars().count(),
            "version": model.version,
        })),
    )
    .await?;
    transaction.commit().await?;
    archive_draft_response(model)
}

/// Returns archive drafts for the administrator review queue. This is kept
/// separate from case membership because administrators perform the final
/// de-identification and learning-resource approval lifecycle.
pub async fn list_archive_drafts_for_admin(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
) -> Result<Vec<ArchiveDraftResponse>, ApiError> {
    require_admin(auth)?;
    archive_drafts::Entity::find()
        .order_by_desc(archive_drafts::Column::UpdatedAt)
        .all(db)
        .await?
        .into_iter()
        .map(archive_draft_response)
        .collect()
}

pub async fn get_public_progress(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<CasePublicProgressResponse, ApiError> {
    let detail = case_service::get_case(db, auth, case_id).await?;
    if detail.access_role != CaseRole::Family {
        return Err(ApiError::Forbidden(
            "only family members can access public progress".to_owned(),
        ));
    }
    let confirmed_progress = detail
        .clues
        .iter()
        .filter(|clue| clue.status == "confirmed")
        .map(|clue| CasePublicProgressItem {
            clue_id: clue.id.clone(),
            progress_type: "confirmed_update".to_owned(),
            review_status: clue.status.clone(),
            updated_at: clue.updated_at.clone(),
        })
        .collect();
    let requested_family_information = detail
        .clues
        .iter()
        .filter(|clue| {
            matches!(
                clue.status.as_str(),
                "pending_review" | "needs_verification"
            ) && clue.is_own_submission
        })
        .map(|clue| CasePublicProgressItem {
            clue_id: clue.id.clone(),
            progress_type: "family_follow_up".to_owned(),
            review_status: clue.status.clone(),
            updated_at: clue.updated_at.clone(),
        })
        .collect();
    Ok(CasePublicProgressResponse {
        case_id: detail.id,
        status: detail.status,
        generated_at: now(),
        confirmed_progress,
        requested_family_information,
        safety_and_contact_reminders: vec![
            "Only human-reviewed confirmed progress is shared here.".to_owned(),
            "Contact the case commander before sharing information externally.".to_owned(),
        ],
    })
}

pub async fn create_clue_drafts(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: CreateClueDraftRequest,
    gateway: &crate::ai_gateway::AiGateway,
) -> Result<Vec<ClueDraftResponse>, ApiError> {
    case_service::require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let text = required_text("text", request.text, 4_000)?;
    let source_type = request
        .source_type
        .unwrap_or_else(|| "manual_report".to_owned())
        .trim()
        .to_lowercase();
    if !matches!(source_type.as_str(), "manual_report" | "field_report") {
        return Err(ApiError::Validation(
            "source_type must be manual_report or field_report".to_owned(),
        ));
    }
    let ai_request = AiRequest {
        capability: AiCapability::StructuredExtraction,
        data_level: DataLevel::Collaborative,
        purpose: AiPurpose::ClueDraft,
        data_region: "CN".to_owned(),
        system_instruction: Some("Return JSON only using the supplied schema. Extract only candidate fields explicitly supported by the supplied text. Keep unknown fields null or empty. Do not decide whether a report is true, infer a current or future location, add facts, or issue an action instruction. Include a source excerpt and per-field source excerpts only from the supplied text.".to_owned()),
        output_schema: Some(clue_candidate_schema()),
        output_schema_name: Some("clue_draft_candidate".to_owned()),
        input: text.clone(),
        requested_output_tokens: 400,
        template_version: "clue-draft-rule-v1".to_owned(),
        input_scope_reference: "case_authorized_text".to_owned(),
        redaction_policy_version: "case-collaboration-v1".to_owned(),
    };
    let execution = gateway.execute(&ai_request).await;
    let decision = execution.decision();
    let (candidate, provider_model, degradation_status, audit_status) = match execution {
        AiExecutionResult::Completed { route, output } => {
            match gateway.decode_json::<ClueDraftCandidate>(&output) {
                Ok(candidate) => (
                    normalized_candidate(candidate, &text),
                    Some(route.model),
                    "manual_review_required",
                    AiTaskStatus::Completed,
                ),
                Err(_) => (
                    fallback_candidate(&text),
                    None,
                    "rule_based_fallback",
                    AiTaskStatus::Failed,
                ),
            }
        }
        AiExecutionResult::Degraded { .. } => (
            fallback_candidate(&text),
            None,
            "rule_based_fallback",
            AiTaskStatus::Degraded,
        ),
        AiExecutionResult::Failed { .. } => (
            fallback_candidate(&text),
            None,
            "rule_based_fallback",
            AiTaskStatus::Failed,
        ),
    };
    let candidate_json = serde_json::to_string(&candidate).map_err(|_| ApiError::Internal)?;
    let execution_audit = AiExecutionAudit::for_request(&ai_request, &decision, audit_status);
    let timestamp = now();
    let transaction = db.begin().await?;
    let draft = clue_drafts::ActiveModel {
        id: Set(case_service::new_id()),
        case_id: Set(case_id.to_owned()),
        status: Set("draft".to_owned()),
        content: Set(text),
        source_type: Set(source_type.clone()),
        raw_record_reference: Set(trim(request.raw_record_reference, 500)?),
        uncertainty_notice: Set(
            "This is an unreviewed extraction draft. Confirm time, location, and source before creating a clue."
                .to_owned(),
        ),
        template_version: Set("clue-draft-rule-v1".to_owned()),
        provider_model: Set(provider_model),
        degradation_status: Set(degradation_status.to_owned()),
        candidate_json: Set(candidate_json),
        review_status: Set("pending_review".to_owned()),
        reviewed_by_user_id: Set(None),
        reviewed_at: Set(None),
        review_reason: Set(None),
        version: Set(1),
        promoted_clue_id: Set(None),
        created_by_user_id: Set(auth.id.clone()),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp),
    }
    .insert(&transaction)
    .await?;
    crate::ai_gateway::persist_execution_audit(
        &transaction,
        &execution_audit,
        &auth.id,
        Some(case_id),
    )
    .await?;
    case_service::write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "clue.draft_created",
        "clue_draft",
        draft.id.clone(),
        Some(json!({ "source_type": source_type, "degradation_status": degradation_status })),
    )
    .await?;
    transaction.commit().await?;
    Ok(vec![clue_draft_response(draft)?])
}

/// Lists durable clue-extraction drafts so a commander can resume review after
/// a page refresh or a later hand-off. Raw source text remains protected by
/// the normal case-role check.
pub async fn list_clue_drafts(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<Vec<ClueDraftResponse>, ApiError> {
    case_service::require_case_role(db, &auth.id, case_id, &[CaseRole::Commander]).await?;
    clue_drafts::Entity::find()
        .filter(clue_drafts::Column::CaseId.eq(case_id))
        .order_by_desc(clue_drafts::Column::CreatedAt)
        .all(db)
        .await?
        .into_iter()
        .map(clue_draft_response)
        .collect()
}

pub async fn create_summary_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: CreateSummaryDraftRequest,
    gateway: &crate::ai_gateway::AiGateway,
) -> Result<SummaryDraftResponse, ApiError> {
    case_service::require_case_role(db, &auth.id, case_id, &[CaseRole::Commander]).await?;
    let summary = case_summary_service::get_case_summary(db, auth, case_id).await?;
    let (content, provider_model, execution_audit, initial_status, publication_eligible) =
        match request.content {
            Some(content) => (
                required_text("content", content, 12_000)?,
                None,
                None,
                "draft",
                false,
            ),
            None => {
                let input = serde_json::to_string(&summary).map_err(|_| ApiError::Internal)?;
                let ai_request = AiRequest {
                capability: AiCapability::CaseSummary,
                data_level: DataLevel::Internal,
                purpose: AiPurpose::CaseSummaryDraft,
                data_region: "CN".to_owned(),
                system_instruction: Some("Return a concise case-summary draft. Keep unverified information explicitly unverified and do not make location conclusions or task decisions.".to_owned()),
                output_schema: Some(summary_candidate_schema()),
                output_schema_name: Some("case_summary_candidate".to_owned()),
                input,
                requested_output_tokens: 800,
                template_version: "case-summary-ai-v1".to_owned(),
                input_scope_reference: "authorized_case_summary_inputs".to_owned(),
                redaction_policy_version: "case-summary-v1".to_owned(),
            };
                let execution = gateway.execute(&ai_request).await;
                let decision = execution.decision();
                let (content, provider_model, audit_status) = match execution {
                    AiExecutionResult::Completed { route, output } => {
                        match gateway.decode_json::<SummaryCandidate>(&output).and_then(
                            |candidate| {
                                required_text("content", candidate.content, 12_000).map_err(|_| {
                                    crate::ai_gateway::AiGatewayError::InvalidStructuredOutput
                                })
                            },
                        ) {
                            Ok(content) => (content, Some(route.model), AiTaskStatus::Completed),
                            Err(_) => (
                                deterministic_draft_content(&summary),
                                None,
                                AiTaskStatus::Failed,
                            ),
                        }
                    }
                    AiExecutionResult::Degraded { .. } => (
                        deterministic_draft_content(&summary),
                        None,
                        AiTaskStatus::Degraded,
                    ),
                    AiExecutionResult::Failed { .. } => (
                        deterministic_draft_content(&summary),
                        None,
                        AiTaskStatus::Failed,
                    ),
                };
                (
                    content,
                    provider_model,
                    Some(AiExecutionAudit::for_request(
                        &ai_request,
                        &decision,
                        audit_status,
                    )),
                    "pending_review",
                    true,
                )
            }
        };
    let timestamp = now();
    let scope = serde_json::to_string(&summary.source_scope).map_err(|_| ApiError::Internal)?;
    let transaction = db.begin().await?;
    let previous = summary_drafts::Entity::find()
        .filter(summary_drafts::Column::CaseId.eq(case_id))
        .order_by_desc(summary_drafts::Column::Version)
        .one(&transaction)
        .await?;
    let version = previous
        .as_ref()
        .and_then(|draft| draft.version.checked_add(1))
        .unwrap_or(1);
    let model = summary_drafts::ActiveModel {
        id: Set(case_service::new_id()),
        case_id: Set(case_id.to_owned()),
        parent_draft_id: Set(previous.as_ref().map(|draft| draft.id.clone())),
        version: Set(version),
        status: Set(initial_status.to_owned()),
        content: Set(content),
        source_scope_json: Set(scope),
        template_version: Set(if provider_model.is_some() {
            "case-summary-ai-v1".to_owned()
        } else {
            DRAFT_TEMPLATE_VERSION.to_owned()
        }),
        provider_model: Set(provider_model),
        publication_eligible: Set(publication_eligible),
        generated_by_user_id: Set(auth.id.clone()),
        reviewed_by_user_id: Set(None),
        reviewed_at: Set(None),
        review_reason: Set(None),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp),
    }
    .insert(&transaction)
    .await?;
    if let Some(execution_audit) = execution_audit {
        crate::ai_gateway::persist_execution_audit(
            &transaction,
            &execution_audit,
            &auth.id,
            Some(case_id),
        )
        .await?;
    }
    case_service::write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "summary_draft.created",
        "summary_draft",
        model.id.clone(),
        Some(json!({ "status": model.status, "version": model.version, "parent_draft_id": model.parent_draft_id, "template_version": model.template_version, "publication_eligible": model.publication_eligible })),
    )
    .await?;
    transaction.commit().await?;
    response(model)
}

pub async fn get_latest_summary_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<Option<SummaryDraftResponse>, ApiError> {
    case_service::require_case_role(db, &auth.id, case_id, &[CaseRole::Commander]).await?;
    summary_drafts::Entity::find()
        .filter(summary_drafts::Column::CaseId.eq(case_id))
        .filter(summary_drafts::Column::Status.eq("pending_review"))
        .order_by_desc(summary_drafts::Column::CreatedAt)
        .one(db)
        .await?
        .map(response)
        .transpose()
}

pub async fn list_summary_draft_versions(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<SummaryDraftVersionResponse, ApiError> {
    case_service::require_case_role(db, &auth.id, case_id, &[CaseRole::Commander]).await?;
    let items = summary_drafts::Entity::find()
        .filter(summary_drafts::Column::CaseId.eq(case_id))
        .order_by_desc(summary_drafts::Column::Version)
        .all(db)
        .await?
        .into_iter()
        .map(response)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SummaryDraftVersionResponse { items })
}

pub async fn diff_summary_draft_versions(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    from_id: &str,
    to_id: &str,
) -> Result<SummaryDraftDiffResponse, ApiError> {
    case_service::require_case_role(db, &auth.id, case_id, &[CaseRole::Commander]).await?;
    let drafts = summary_drafts::Entity::find()
        .filter(summary_drafts::Column::CaseId.eq(case_id))
        .filter(summary_drafts::Column::Id.is_in([from_id.to_owned(), to_id.to_owned()]))
        .all(db)
        .await?;
    let from = drafts
        .iter()
        .find(|draft| draft.id == from_id)
        .ok_or_else(|| ApiError::NotFound("source summary version was not found".to_owned()))?;
    let to = drafts
        .iter()
        .find(|draft| draft.id == to_id)
        .ok_or_else(|| ApiError::NotFound("target summary version was not found".to_owned()))?;
    let from_lines = from
        .content
        .lines()
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    let to_lines = to
        .content
        .lines()
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    Ok(SummaryDraftDiffResponse {
        from_version: from.version,
        to_version: to.version,
        added: to_lines.difference(&from_lines).cloned().collect(),
        removed: from_lines.difference(&to_lines).cloned().collect(),
    })
}

pub async fn review_summary_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    draft_id: &str,
    request: ReviewSummaryDraftRequest,
) -> Result<SummaryDraftResponse, ApiError> {
    case_service::require_case_role(db, &auth.id, case_id, &[CaseRole::Commander]).await?;
    let action = request.action.trim().to_lowercase();
    if !matches!(
        action.as_str(),
        "submit" | "publish" | "reject" | "withdraw"
    ) {
        return Err(ApiError::Validation(
            "action must be submit, publish, reject, or withdraw".to_owned(),
        ));
    }
    let reason = required_text("reason", request.reason, 1_000)?;
    let transaction = db.begin().await?;
    let existing = summary_drafts::Entity::find_by_id(draft_id)
        .one(&transaction)
        .await?
        .filter(|draft| draft.case_id == case_id)
        .ok_or_else(|| ApiError::NotFound("summary draft was not found".to_owned()))?;
    let transition_allowed = matches!(
        (existing.status.as_str(), action.as_str()),
        ("draft", "submit") | ("pending_review", "publish" | "reject") | ("published", "withdraw")
    );
    if !transition_allowed {
        return Err(ApiError::Conflict(
            "summary draft cannot transition from its current status".to_owned(),
        ));
    }
    if action == "publish" && !existing.publication_eligible {
        return Err(ApiError::Validation(
            "only a server-generated, scope-controlled summary draft can be published".to_owned(),
        ));
    }
    if action == "publish" {
        for published in summary_drafts::Entity::find()
            .filter(summary_drafts::Column::CaseId.eq(case_id))
            .filter(summary_drafts::Column::Status.eq("published"))
            .all(&transaction)
            .await?
        {
            let mut active = published.into_active_model();
            active.status = Set("superseded".to_owned());
            active.updated_at = Set(now());
            active.update(&transaction).await?;
        }
    }
    let next_status = match action.as_str() {
        "submit" => "pending_review",
        "publish" => "published",
        "reject" => "rejected",
        "withdraw" => "withdrawn",
        _ => return Err(ApiError::Internal),
    };
    let timestamp = now();
    let mut active = existing.into_active_model();
    active.status = Set(next_status.to_owned());
    active.reviewed_by_user_id = Set(Some(auth.id.clone()));
    active.reviewed_at = Set(Some(timestamp.clone()));
    active.review_reason = Set(Some(reason.clone()));
    active.updated_at = Set(timestamp);
    let updated = active.update(&transaction).await?;
    case_service::write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "summary_draft.reviewed",
        "summary_draft",
        updated.id.clone(),
        Some(json!({ "action": action, "reason_length": reason.chars().count() })),
    )
    .await?;
    transaction.commit().await?;
    response(updated)
}

pub async fn list_case_pois(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    query: CasePoiQuery,
    amap: &crate::amap_service::AmapService,
) -> Result<CasePoiResponse, ApiError> {
    let role = case_service::require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let category = query
        .category
        .unwrap_or_else(|| "hospital".to_owned())
        .trim()
        .to_lowercase();
    if !matches!(
        category.as_str(),
        "hospital" | "police" | "transit" | "market" | "community_service"
    ) {
        return Err(ApiError::Validation("category is unsupported".to_owned()));
    }
    let center = authorized_center(db, auth, case_id, role).await?;
    let (items, source, degradation_status, fallback_message) = match amap.search_nearby_pois(center, &category).await { PoiSearch::Available(pois) if !pois.is_empty() => (pois.into_iter().map(poi_item).collect(), "amap_webservice".to_owned(), "available".to_owned(), None), _ => (fallback_pois(&category), "fixed_demo_fallback".to_owned(), "degraded".to_owned(), Some("Nearby POI service is unavailable. Use the task area text and contact the commander for local confirmation.".to_owned())) };
    Ok(CasePoiResponse {
        items,
        source,
        degradation_status,
        fallback_message,
    })
}

async fn authorized_center(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    role: CaseRole,
) -> Result<Coordinate, ApiError> {
    let tasks = task_service::list_all_visible_tasks(db, auth, case_id).await?;
    if let Some(task) = tasks
        .into_iter()
        .find(|task| task.longitude.is_some() && task.latitude.is_some())
    {
        return Ok(Coordinate {
            longitude: task.longitude.ok_or(ApiError::Internal)?,
            latitude: task.latitude.ok_or(ApiError::Internal)?,
        });
    }
    let mut places = case_places::Entity::find()
        .filter(case_places::Column::CaseId.eq(case_id))
        .filter(case_places::Column::ReviewStatus.eq("confirmed"))
        .filter(case_places::Column::Longitude.is_not_null())
        .filter(case_places::Column::Latitude.is_not_null());
    if role == CaseRole::Volunteer {
        places = places.filter(case_places::Column::Visibility.eq("public"));
    }
    if let Some(place) = places
        .order_by_desc(case_places::Column::UpdatedAt)
        .one(db)
        .await?
    {
        return Ok(Coordinate {
            longitude: place.longitude.ok_or(ApiError::Internal)?,
            latitude: place.latitude.ok_or(ApiError::Internal)?,
        });
    }
    Err(ApiError::Conflict(
        "no authorized coordinate is available; use the task area text and contact the commander"
            .to_owned(),
    ))
}

fn poi_item(poi: crate::amap_service::Poi) -> CasePoiItem {
    CasePoiItem {
        id: poi.id,
        name: poi.name,
        category: poi.category,
        address: poi.address,
        longitude: poi.coordinate.map(|coordinate| coordinate.longitude),
        latitude: poi.coordinate.map(|coordinate| coordinate.latitude),
    }
}
fn fallback_pois(category: &str) -> Vec<CasePoiItem> {
    vec![CasePoiItem {
        id: format!("fallback-{category}"),
        name: format!("Fictional nearby {category}"),
        category: category.to_owned(),
        address: Some("Confirm locally with the commander".to_owned()),
        longitude: None,
        latitude: None,
    }]
}
fn deterministic_draft_content(summary: &crate::models::CaseSummaryResponse) -> String {
    format!(
        "Draft for commander review. Only confirmed information is factual. Confirmed clues: {}. Pending verification: {}. Safety reminders: {}",
        summary.confirmed_clues.len(),
        summary.pending_verification.len(),
        summary.safety_reminders.join(" ")
    )
}

fn archive_candidate_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "timeline": { "type": "array", "items": { "type": "string" } },
            "lessons": { "type": "array", "items": { "type": "string" } },
            "uncertainty": { "type": "string" }
        },
        "required": ["timeline", "lessons", "uncertainty"],
        "additionalProperties": false
    })
}

fn clue_candidate_schema() -> serde_json::Value {
    let nullable_string = json!({ "type": ["string", "null"] });
    json!({
        "type": "object",
        "properties": {
            "content_summary": nullable_string,
            "occurred_at": { "type": ["string", "null"] },
            "location_text": { "type": ["string", "null"] },
            "source_text": { "type": ["string", "null"] },
            "action_candidates": { "type": "array", "items": { "type": "string" } },
            "missing_fields": { "type": "array", "items": { "type": "string" } },
            "source_excerpt": { "type": "string" },
            "field_sources": {
                "type": "object",
                "properties": {
                    "content_summary": { "type": "object", "properties": { "reference": { "type": ["string", "null"] }, "excerpt": { "type": ["string", "null"] } }, "required": ["reference", "excerpt"], "additionalProperties": false },
                    "occurred_at": { "type": "object", "properties": { "reference": { "type": ["string", "null"] }, "excerpt": { "type": ["string", "null"] } }, "required": ["reference", "excerpt"], "additionalProperties": false },
                    "location_text": { "type": "object", "properties": { "reference": { "type": ["string", "null"] }, "excerpt": { "type": ["string", "null"] } }, "required": ["reference", "excerpt"], "additionalProperties": false },
                    "source_text": { "type": "object", "properties": { "reference": { "type": ["string", "null"] }, "excerpt": { "type": ["string", "null"] } }, "required": ["reference", "excerpt"], "additionalProperties": false },
                    "action_candidates": { "type": "object", "properties": { "reference": { "type": ["string", "null"] }, "excerpt": { "type": ["string", "null"] } }, "required": ["reference", "excerpt"], "additionalProperties": false }
                },
                "required": ["content_summary", "occurred_at", "location_text", "source_text", "action_candidates"],
                "additionalProperties": false
            }
        },
        "required": ["content_summary", "occurred_at", "location_text", "source_text", "action_candidates", "missing_fields", "source_excerpt", "field_sources"],
        "additionalProperties": false
    })
}

fn summary_candidate_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": { "content": { "type": "string" } },
        "required": ["content"],
        "additionalProperties": false
    })
}

fn deterministic_archive_draft_content(
    case_status: &str,
    confirmed_clue_count: u64,
    completed_task_count: u64,
) -> String {
    format!(
        "Internal archive draft. This record is not de-identified, publishable, indexable, exportable, or printable. Case status: {case_status}. Confirmed clue count: {confirmed_clue_count}. Completed task count: {completed_task_count}. Source scope is limited to counts and status metadata; it excludes raw clues, attachments, health notes, contacts, exact locations, routes, and task result text. Human de-identification and review are required before any separate reuse workflow."
    )
}

fn valid_archive_candidate(candidate: &ArchiveOrganizationCandidate) -> bool {
    candidate.timeline.len() <= 12
        && candidate.lessons.len() <= 12
        && candidate
            .timeline
            .iter()
            .chain(candidate.lessons.iter())
            .all(|item| !item.trim().is_empty() && item.chars().count() <= 500)
        && !candidate.uncertainty.trim().is_empty()
        && candidate.uncertainty.chars().count() <= 500
}

fn archive_candidate_content(candidate: ArchiveOrganizationCandidate) -> String {
    format!(
        "Internal, unreviewed and not-yet-deidentified organization draft. Timeline candidates:\n- {}\nExperience candidates:\n- {}\nUncertainty and correction required: {}\nThis draft is not indexable, exportable, printable, or publishable until an administrator confirms de-identification and review.",
        candidate.timeline.join("\n- "),
        candidate.lessons.join("\n- "),
        candidate.uncertainty.trim(),
    )
}
fn required_text(label: &str, value: String, maximum: usize) -> Result<String, ApiError> {
    let value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > maximum {
        return Err(ApiError::Validation(format!(
            "{label} must contain between 1 and {maximum} characters"
        )));
    }
    Ok(value)
}
fn trim(value: Option<String>, maximum: usize) -> Result<Option<String>, ApiError> {
    match value {
        Some(value) => Ok(Some(required_text("raw_record_reference", value, maximum)?)),
        None => Ok(None),
    }
}
fn response(model: summary_drafts::Model) -> Result<SummaryDraftResponse, ApiError> {
    Ok(SummaryDraftResponse {
        id: model.id,
        case_id: model.case_id,
        parent_draft_id: model.parent_draft_id,
        version: model.version,
        status: model.status,
        content: model.content,
        source_scope: serde_json::from_str(&model.source_scope_json)
            .map_err(|_| ApiError::Internal)?,
        template_version: model.template_version,
        provider_model: model.provider_model,
        generated_at: model.created_at.clone(),
        reviewed_at: model.reviewed_at,
        review_reason: model.review_reason,
        created_at: model.created_at,
        updated_at: model.updated_at,
        publication_eligible: model.publication_eligible,
    })
}

fn archive_draft_response(model: archive_drafts::Model) -> Result<ArchiveDraftResponse, ApiError> {
    Ok(ArchiveDraftResponse {
        id: model.id,
        case_id: model.case_id,
        status: model.status,
        content: model.content,
        source_scope: serde_json::from_str(&model.source_scope_json)
            .map_err(|_| ApiError::Internal)?,
        deidentification_status: model.deidentification_status,
        template_version: model.template_version,
        provider_model: model.provider_model,
        version: model.version,
        usage_scope: model.usage_scope,
        retention_status: model.retention_status,
        deidentified_at: model.deidentified_at,
        reviewed_at: model.reviewed_at,
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}

fn require_admin(auth: &AuthenticatedUser) -> Result<(), ApiError> {
    auth.global_capabilities
        .contains(&GlobalCapability::Admin)
        .then_some(())
        .ok_or_else(|| {
            ApiError::Forbidden("only administrators can perform this action".to_owned())
        })
}

fn clue_draft_response(model: clue_drafts::Model) -> Result<ClueDraftResponse, ApiError> {
    Ok(ClueDraftResponse {
        id: model.id,
        case_id: model.case_id,
        status: model.status,
        content: model.content,
        source_type: model.source_type,
        raw_record_reference: model.raw_record_reference,
        occurred_at: serde_json::from_str::<ClueDraftCandidate>(&model.candidate_json)
            .map_err(|_| ApiError::Internal)?
            .occurred_at,
        location_text: serde_json::from_str::<ClueDraftCandidate>(&model.candidate_json)
            .map_err(|_| ApiError::Internal)?
            .location_text,
        uncertainty_notice: model.uncertainty_notice,
        template_version: model.template_version,
        provider_model: model.provider_model,
        degradation_status: model.degradation_status,
        candidate: serde_json::from_str(&model.candidate_json).map_err(|_| ApiError::Internal)?,
        review_status: model.review_status,
        reviewed_at: model.reviewed_at,
        review_reason: model.review_reason,
        version: model.version,
        promoted_clue_id: model.promoted_clue_id,
    })
}

pub async fn review_clue_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    draft_id: &str,
    request: ReviewClueDraftRequest,
) -> Result<ClueDraftResponse, ApiError> {
    case_service::require_case_role(db, &auth.id, case_id, &[CaseRole::Commander]).await?;
    let action = request.action.trim().to_lowercase();
    if !matches!(action.as_str(), "accept" | "reject") {
        return Err(ApiError::Validation(
            "action must be accept or reject".to_owned(),
        ));
    }
    let reason = required_text("reason", request.reason, 1_000)?;
    let transaction = db.begin().await?;
    let existing = clue_drafts::Entity::find_by_id(draft_id)
        .one(&transaction)
        .await?
        .filter(|draft| draft.case_id == case_id)
        .ok_or_else(|| ApiError::NotFound("clue draft was not found".to_owned()))?;
    if existing.review_status != "pending_review" {
        return Err(ApiError::Conflict(
            "clue draft has already been reviewed".to_owned(),
        ));
    }
    let candidate = apply_field_decisions(
        normalized_candidate(request.candidate, &existing.content),
        &request.field_decisions,
    )?;
    let candidate_json = serde_json::to_string(&candidate).map_err(|_| ApiError::Internal)?;
    let next_status = if action == "accept" {
        "accepted"
    } else {
        "rejected"
    };
    let timestamp = now();
    let affected = clue_drafts::Entity::update_many()
        .col_expr(
            clue_drafts::Column::CandidateJson,
            Expr::value(candidate_json),
        )
        .col_expr(
            clue_drafts::Column::ReviewStatus,
            Expr::value(next_status.to_owned()),
        )
        .col_expr(
            clue_drafts::Column::ReviewedByUserId,
            Expr::value(Some(auth.id.clone())),
        )
        .col_expr(
            clue_drafts::Column::ReviewedAt,
            Expr::value(Some(timestamp.clone())),
        )
        .col_expr(
            clue_drafts::Column::ReviewReason,
            Expr::value(Some(reason.clone())),
        )
        .col_expr(
            clue_drafts::Column::Version,
            Expr::value(existing.version + 1),
        )
        .col_expr(clue_drafts::Column::UpdatedAt, Expr::value(timestamp))
        .filter(clue_drafts::Column::Id.eq(draft_id))
        .filter(clue_drafts::Column::Version.eq(existing.version))
        .filter(clue_drafts::Column::ReviewStatus.eq("pending_review"))
        .exec(&transaction)
        .await?;
    if affected.rows_affected != 1 {
        return Err(ApiError::Conflict(
            "clue draft changed during review; reload and try again".to_owned(),
        ));
    }
    let mut updated = clue_drafts::Entity::find_by_id(draft_id)
        .one(&transaction)
        .await?
        .ok_or(ApiError::Internal)?;
    case_service::write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "clue_draft.reviewed",
        "clue_draft",
        updated.id.clone(),
        Some(json!({
            "action": action,
            "version": updated.version,
            "reason_length": reason.chars().count(),
            "field_decisions": request.field_decisions.iter().map(|(field, decision)| json!({
                "field": field,
                "action": decision.action,
                "has_value": decision.value.as_ref().is_some_and(|value| !value.trim().is_empty()),
                "reason_length": decision.reason.as_ref().map(|value| value.chars().count()).unwrap_or(0),
            })).collect::<Vec<_>>(),
        })),
    )
    .await?;
    if action == "accept" {
        let promoted = case_service::create_clue_in_transaction(
            &transaction,
            auth,
            case_id,
            CreateClueRequest {
                source: candidate
                    .source_text
                    .clone()
                    .unwrap_or_else(|| "AI structured draft".to_owned())
                    .chars()
                    .take(64)
                    .collect(),
                content: candidate
                    .content_summary
                    .clone()
                    .unwrap_or_else(|| existing.content.clone()),
                source_type: Some(existing.source_type.clone()),
                raw_record_reference: existing.raw_record_reference.clone(),
                occurred_at: candidate.occurred_at.clone(),
                location_text: candidate.location_text.clone(),
                location_precision: candidate
                    .location_text
                    .as_ref()
                    .map(|_| "approximate".to_owned()),
                next_action: candidate.action_candidates.first().cloned(),
                linked_task_reference: None,
                attachment_ids: Vec::new(),
            },
        )
        .await?;
        let promoted_id = promoted.id.clone();
        let affected = clue_drafts::Entity::update_many()
            .col_expr(
                clue_drafts::Column::PromotedClueId,
                Expr::value(Some(promoted_id)),
            )
            .filter(clue_drafts::Column::Id.eq(draft_id))
            .filter(clue_drafts::Column::ReviewStatus.eq("accepted"))
            .exec(&transaction)
            .await?;
        if affected.rows_affected != 1 {
            return Err(ApiError::Conflict(
                "accepted clue draft changed before promotion could be recorded".to_owned(),
            ));
        }
        case_service::write_audit(
            &transaction,
            Some(case_id.to_owned()),
            auth,
            "clue_draft.promoted",
            "clue_draft",
            draft_id.to_owned(),
            Some(json!({ "clue_id": promoted.id })),
        )
        .await?;
        updated.promoted_clue_id = Some(promoted.id);
    }
    transaction.commit().await?;
    clue_draft_response(updated)
}

fn apply_field_decisions(
    mut candidate: ClueDraftCandidate,
    decisions: &std::collections::BTreeMap<String, ClueDraftFieldDecision>,
) -> Result<ClueDraftCandidate, ApiError> {
    for (field, decision) in decisions {
        let action = decision.action.trim().to_lowercase();
        if !matches!(action.as_str(), "accept" | "edit" | "clear") {
            return Err(ApiError::Validation(format!(
                "unsupported field decision for {field}"
            )));
        }
        let value = decision
            .value
            .clone()
            .and_then(|value| optional_excerpt(value, 1_000));
        if action == "edit" && value.is_none() {
            return Err(ApiError::Validation(format!(
                "edited field {field} requires a value"
            )));
        }
        match field.as_str() {
            "content_summary" if action == "clear" => candidate.content_summary = None,
            "content_summary" if action == "edit" => candidate.content_summary = value,
            "occurred_at" if action == "clear" => candidate.occurred_at = None,
            "occurred_at" if action == "edit" => candidate.occurred_at = value,
            "location_text" if action == "clear" => candidate.location_text = None,
            "location_text" if action == "edit" => candidate.location_text = value,
            "source_text" if action == "clear" => candidate.source_text = None,
            "source_text" if action == "edit" => candidate.source_text = value,
            "action_candidates" if action == "clear" => candidate.action_candidates = Vec::new(),
            "action_candidates" if action == "edit" => {
                candidate.action_candidates = value.into_iter().collect()
            }
            "content_summary" | "occurred_at" | "location_text" | "source_text"
            | "action_candidates" => {}
            _ => {
                return Err(ApiError::Validation(format!(
                    "unsupported candidate field {field}"
                )));
            }
        }
    }
    candidate.missing_fields = [
        ("occurred_at", candidate.occurred_at.is_none()),
        ("location_text", candidate.location_text.is_none()),
        ("source_text", candidate.source_text.is_none()),
        ("action_candidates", candidate.action_candidates.is_empty()),
    ]
    .into_iter()
    .filter_map(|(field, missing)| missing.then_some(field.to_owned()))
    .collect();
    Ok(candidate)
}

fn fallback_candidate(text: &str) -> ClueDraftCandidate {
    ClueDraftCandidate {
        missing_fields: vec![
            "occurred_at".to_owned(),
            "location_text".to_owned(),
            "source_text".to_owned(),
            "action_candidates".to_owned(),
        ],
        source_excerpt: excerpt(text, 500),
        ..Default::default()
    }
}

fn normalized_candidate(mut candidate: ClueDraftCandidate, text: &str) -> ClueDraftCandidate {
    candidate.content_summary = candidate
        .content_summary
        .and_then(|value| optional_excerpt(value, 1_000));
    candidate.occurred_at = candidate
        .occurred_at
        .and_then(|value| optional_excerpt(value, 80));
    candidate.location_text = candidate
        .location_text
        .and_then(|value| optional_excerpt(value, 500));
    candidate.source_text = candidate
        .source_text
        .and_then(|value| optional_excerpt(value, 300));
    candidate.action_candidates = candidate
        .action_candidates
        .into_iter()
        .filter_map(|value| optional_excerpt(value, 300))
        .take(8)
        .collect();
    candidate.missing_fields = [
        ("occurred_at", candidate.occurred_at.is_none()),
        ("location_text", candidate.location_text.is_none()),
        ("source_text", candidate.source_text.is_none()),
        ("action_candidates", candidate.action_candidates.is_empty()),
    ]
    .into_iter()
    .filter_map(|(field, missing)| missing.then_some(field.to_owned()))
    .collect();
    candidate.source_excerpt = excerpt(text, 500);
    candidate
}

fn optional_excerpt(value: String, maximum: usize) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| excerpt(value, maximum))
}

fn excerpt(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
