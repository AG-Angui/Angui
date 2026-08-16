use chrono::{SecondsFormat, Utc};
use hmac::{Hmac, Mac};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Sha256;

use crate::{
    ai_gateway::{AiCapability, AiExecutionResult, AiPurpose, AiRequest, AiTaskStatus, DataLevel},
    amap_service::{Coordinate, PoiSearch, RouteEstimate, RouteMode},
    entities::{
        archive_drafts, archive_review_materials, case_places, case_source_records, cases,
        clue_drafts, clues, summary_drafts, tasks,
    },
    error::ApiError,
    models::{
        ArchiveDraftResponse, ArchiveReviewMaterialDiffResponse, ArchiveReviewMaterialResponse,
        AuthenticatedUser, CasePoiItem, CasePoiQuery, CasePoiResponse, CasePoiRouteQuery,
        CasePoiRouteResponse, CasePublicProgressItem, CasePublicProgressResponse,
        CaseSourceRecordResponse, ClueDraftCandidate, ClueDraftFieldDecision, ClueDraftResponse,
        CreateCaseSourceRecordRequest, CreateClueDraftRequest, CreateClueRequest,
        CreateSummaryDraftRequest, DeidentifyArchiveDraftRequest, PublishedSummaryVersion,
        PublishedSummaryVersionResponse, RestoreArchiveReviewMaterialRequest,
        ReviewArchiveDraftRequest, ReviewClueDraftRequest, ReviewSummaryDraftRequest,
        SummaryDraftDiffResponse, SummaryDraftResponse, SummaryDraftVersionResponse,
    },
    roles::{CaseRole, GlobalCapability},
    services::{case_service, case_summary_service, task_service},
};

const DRAFT_TEMPLATE_VERSION: &str = "case-summary-rule-v1";
const ARCHIVE_DRAFT_TEMPLATE_VERSION: &str = "case-archive-safe-metadata-v1";
const POI_SELECTION_TOKEN_TTL_SECONDS: i64 = 5 * 60;

#[derive(Deserialize, Serialize)]
struct PoiSelectionTokenPayload {
    case_id: String,
    user_id: String,
    destination_longitude: f64,
    destination_latitude: f64,
    expires_at: i64,
}

pub async fn create_case_source_record(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: CreateCaseSourceRecordRequest,
) -> Result<CaseSourceRecordResponse, ApiError> {
    case_service::require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let record_type = request.record_type.trim().to_lowercase();
    if !matches!(
        record_type.as_str(),
        "message" | "phone_record" | "field_feedback"
    ) {
        return Err(ApiError::Validation(
            "record_type must be message, phone_record, or field_feedback".to_owned(),
        ));
    }
    let content = required_text("content", request.content, 4_000)?;
    let timestamp = now();
    let model = case_source_records::ActiveModel {
        id: Set(case_service::new_id()),
        case_id: Set(case_id.to_owned()),
        record_type: Set(record_type),
        content: Set(content),
        occurred_at: Set(trim(request.occurred_at, 40)?),
        source_reference: Set(trim(request.source_reference, 500)?),
        created_by_user_id: Set(auth.id.clone()),
        created_at: Set(timestamp),
    }
    .insert(db)
    .await?;
    case_service::write_audit(
        db,
        Some(case_id.to_owned()),
        auth,
        "case_source_record.created",
        "case_source_record",
        model.id.clone(),
        Some(json!({"record_type": model.record_type})),
    )
    .await?;
    Ok(case_source_record_response(model))
}

pub async fn list_case_source_records(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<Vec<CaseSourceRecordResponse>, ApiError> {
    case_service::require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    Ok(case_source_records::Entity::find()
        .filter(case_source_records::Column::CaseId.eq(case_id))
        .order_by_desc(case_source_records::Column::CreatedAt)
        .all(db)
        .await?
        .into_iter()
        .map(case_source_record_response)
        .collect())
}

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
    confirmed_information: Vec<String>,
    pending_verification: Vec<String>,
    excluded_directions: Vec<String>,
    safety_reminders: Vec<String>,
    uncertainty_notice: String,
}

pub async fn create_archive_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    _gateway: &crate::ai_gateway::AiGateway,
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
        "confirmed_clue_review_material".to_owned(),
        "completed_task_review_material".to_owned(),
    ];
    let confirmed_clue_material = clues::Entity::find()
        .filter(clues::Column::CaseId.eq(case_id))
        .filter(clues::Column::Status.eq("confirmed"))
        .all(&transaction)
        .await?
        .into_iter()
        .map(|clue| format!("confirmed clue: {}", clue.content))
        .collect::<Vec<_>>();
    let task_material = tasks::Entity::find()
        .filter(tasks::Column::CaseId.eq(case_id))
        .filter(tasks::Column::Status.eq("completed"))
        .all(&transaction)
        .await?
        .into_iter()
        .map(|task| format!("completed task: {}", task.title))
        .collect::<Vec<_>>();
    let raw_material = confirmed_clue_material
        .into_iter()
        .chain(task_material)
        .collect::<Vec<_>>()
        .join("\n");
    let timestamp = now();
    let material_version = archive_review_materials::Entity::find()
        .filter(archive_review_materials::Column::CaseId.eq(case_id))
        .order_by_desc(archive_review_materials::Column::Version)
        .one(&transaction)
        .await?
        .map_or(1, |existing| existing.version + 1);
    let material = archive_review_materials::ActiveModel {
        id: Set(case_service::new_id()),
        case_id: Set(case_id.to_owned()),
        version: Set(material_version),
        parent_material_id: Set(None),
        content: Set(raw_material),
        source_scope_json: Set(
            serde_json::to_string(&source_scope).map_err(|_| ApiError::Internal)?
        ),
        status: Set("draft".to_owned()),
        created_by_user_id: Set(auth.id.clone()),
        reviewed_by_user_id: Set(None),
        reviewed_at: Set(None),
        review_reason: Set(None),
        created_at: Set(timestamp.clone()),
    }
    .insert(&transaction)
    .await?;
    let model = archive_drafts::ActiveModel {
        id: Set(case_service::new_id()),
        case_id: Set(case_id.to_owned()),
        status: Set("draft".to_owned()),
        content: Set("Awaiting administrator de-identification of the selected review material before AI organization.".to_owned()),
        source_scope_json: Set(
            serde_json::to_string(&source_scope).map_err(|_| ApiError::Internal)?
        ),
        review_material_id: Set(Some(material.id.clone())),
        deidentification_status: Set("manual_review_required".to_owned()),
        template_version: Set(ARCHIVE_DRAFT_TEMPLATE_VERSION.to_owned()),
        provider_model: Set(None),
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
    gateway: &crate::ai_gateway::AiGateway,
) -> Result<ArchiveDraftResponse, ApiError> {
    require_admin(auth)?;
    let outcome = request.outcome.trim().to_lowercase();
    if !matches!(outcome.as_str(), "confirm" | "reject") {
        return Err(ApiError::Validation(
            "outcome must be confirm or reject".to_owned(),
        ));
    }
    let reason = required_text("reason", request.reason, 1_000)?;
    let deidentified_material = if outcome == "confirm" {
        Some(request.deidentified_material.unwrap_or_default())
    } else {
        None
    };
    let timestamp = now();
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
    let material_id = existing.review_material_id.clone().ok_or_else(|| {
        ApiError::Conflict("archive draft is missing its review material".to_owned())
    })?;
    let original_material = archive_review_materials::Entity::find_by_id(&material_id)
        .one(&transaction)
        .await?
        .ok_or(ApiError::Internal)?;
    let deidentified_material = deidentified_material
        .map(|material| required_text("deidentified_material", material, 12_000))
        .transpose()?;
    let approved_material = if let Some(material_text) = deidentified_material.as_ref() {
        Some(
            archive_review_materials::ActiveModel {
                id: Set(case_service::new_id()),
                case_id: Set(original_material.case_id.clone()),
                version: Set(original_material.version + 1),
                parent_material_id: Set(Some(original_material.id.clone())),
                content: Set(material_text.clone()),
                source_scope_json: Set(original_material.source_scope_json.clone()),
                status: Set("deidentified".to_owned()),
                created_by_user_id: Set(auth.id.clone()),
                reviewed_by_user_id: Set(Some(auth.id.clone())),
                reviewed_at: Set(Some(timestamp.clone())),
                review_reason: Set(Some(reason.clone())),
                created_at: Set(timestamp.clone()),
            }
            .insert(&transaction)
            .await?,
        )
    } else {
        None
    };
    transaction.commit().await?;

    let (content, provider_model, template_version, next_material_id, execution_audits) =
        if let Some(material) = approved_material.as_ref() {
            let ai_request = AiRequest { capability: AiCapability::CaseOrganization, data_level: DataLevel::Internal, purpose: AiPurpose::CaseArchiveDraft, data_region: "CN".to_owned(), system_instruction: Some("Return JSON only: {timeline:string[],lessons:string[],uncertainty:string}. Use only this administrator-approved de-identified material. Do not infer identities, exact locations, health details, causes, or operational outcomes.".to_owned()), output_schema: Some(archive_candidate_schema()), output_schema_name: Some("case_archive_candidate".to_owned()), input: serde_json::to_string(&json!({"deidentified_material": material.content})).map_err(|_| ApiError::Internal)?, requested_output_tokens: 500, template_version: "case-archive-ai-v2".to_owned(), input_scope_reference: format!("approved-deidentified-material:{}:v{}", material.id, material.version), redaction_policy_version: "archive-approved-material-v1".to_owned() };
            let execution = gateway.execute(&ai_request).await;
            let execution_audits =
                crate::ai_gateway::execution_attempt_audits(&ai_request, &execution);
            let (content, provider_model) = match execution {
                AiExecutionResult::Completed { route, output, .. } => {
                    match gateway.decode_json::<ArchiveOrganizationCandidate>(&output) {
                        Ok(candidate) if valid_archive_candidate(&candidate) => {
                            (archive_candidate_content(candidate), Some(route.model))
                        }
                        _ => (
                            deterministic_archive_material_content(&material.content),
                            None,
                        ),
                    }
                }
                AiExecutionResult::Degraded { .. } => (
                    deterministic_archive_material_content(&material.content),
                    None,
                ),
                AiExecutionResult::Failed { .. } => (
                    deterministic_archive_material_content(&material.content),
                    None,
                ),
            };
            (
                content,
                provider_model,
                "case-archive-ai-v2".to_owned(),
                Some(material.id.clone()),
                Some(execution_audits),
            )
        } else {
            (
                existing.content.clone(),
                None,
                existing.template_version.clone(),
                Some(original_material.id.clone()),
                None,
            )
        };
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
    let next_version = existing.version.checked_add(1).ok_or(ApiError::Internal)?;
    let transaction = db.begin().await?;
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
        .col_expr(archive_drafts::Column::Content, Expr::value(content))
        .col_expr(
            archive_drafts::Column::ProviderModel,
            Expr::value(provider_model),
        )
        .col_expr(
            archive_drafts::Column::TemplateVersion,
            Expr::value(template_version),
        )
        .col_expr(
            archive_drafts::Column::ReviewMaterialId,
            Expr::value(next_material_id),
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
    if let Some(execution_audits) = execution_audits {
        crate::ai_gateway::persist_execution_audits(
            &transaction,
            &execution_audits,
            &auth.id,
            Some(&model.case_id),
        )
        .await?;
    }
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

/// Lists the immutable review-material chain for one archive draft, including
/// the administrator-approved material currently selected as AI input.
pub async fn list_archive_review_materials(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    draft_id: &str,
) -> Result<Vec<ArchiveReviewMaterialResponse>, ApiError> {
    require_admin(auth)?;
    let draft = archive_drafts::Entity::find_by_id(draft_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("archive draft was not found".to_owned()))?;
    archive_review_materials::Entity::find()
        .filter(archive_review_materials::Column::CaseId.eq(&draft.case_id))
        .order_by_desc(archive_review_materials::Column::Version)
        .all(db)
        .await?
        .into_iter()
        .map(|material| {
            archive_review_material_response(material, draft.review_material_id.as_deref())
        })
        .collect()
}

/// Compares two immutable material versions without collapsing duplicate lines
/// or modifying either version.
pub async fn diff_archive_review_materials(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    draft_id: &str,
    from_version: i32,
    to_version: i32,
) -> Result<ArchiveReviewMaterialDiffResponse, ApiError> {
    require_admin(auth)?;
    if from_version < 1 || to_version < 1 {
        return Err(ApiError::Validation(
            "material versions must be positive".to_owned(),
        ));
    }
    let draft = archive_drafts::Entity::find_by_id(draft_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("archive draft was not found".to_owned()))?;
    let materials = archive_review_materials::Entity::find()
        .filter(archive_review_materials::Column::CaseId.eq(&draft.case_id))
        .filter(archive_review_materials::Column::Version.is_in([from_version, to_version]))
        .all(db)
        .await?;
    let from = materials.iter().find(|item| item.version == from_version);
    let to = materials.iter().find(|item| item.version == to_version);
    let (Some(from), Some(to)) = (from, to) else {
        return Err(ApiError::NotFound(
            "archive review material version was not found".to_owned(),
        ));
    };
    let (added, removed) = ordered_line_diff(&from.content, &to.content);
    Ok(ArchiveReviewMaterialDiffResponse {
        from_version,
        to_version,
        added,
        removed,
    })
}

/// Copies a historical approved material into a new version and regenerates an
/// archive draft only while that draft is awaiting final administrator review.
pub async fn restore_archive_review_material(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    draft_id: &str,
    version: i32,
    request: RestoreArchiveReviewMaterialRequest,
    gateway: &crate::ai_gateway::AiGateway,
) -> Result<ArchiveDraftResponse, ApiError> {
    require_admin(auth)?;
    if version < 1 {
        return Err(ApiError::Validation(
            "material version must be positive".to_owned(),
        ));
    }
    let reason = required_text("reason", request.reason, 1_000)?;
    let transaction = db.begin().await?;
    let existing = archive_drafts::Entity::find_by_id(draft_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("archive draft was not found".to_owned()))?;
    if existing.status != "pending_review" {
        return Err(ApiError::Conflict(
            "archive draft is not awaiting final review and cannot restore material".to_owned(),
        ));
    }
    let source = archive_review_materials::Entity::find()
        .filter(archive_review_materials::Column::CaseId.eq(&existing.case_id))
        .filter(archive_review_materials::Column::Version.eq(version))
        .one(&transaction)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound("archive review material version was not found".to_owned())
        })?;
    if source.status != "deidentified" {
        return Err(ApiError::Conflict(
            "only an administrator-approved deidentified material can be restored".to_owned(),
        ));
    }
    let current_material_id = existing.review_material_id.clone().ok_or_else(|| {
        ApiError::Conflict("archive draft is missing its selected review material".to_owned())
    })?;
    let mut cursor = Some(current_material_id);
    let mut visited = std::collections::HashSet::new();
    let mut source_is_in_chain = false;
    while let Some(material_id) = cursor {
        if !visited.insert(material_id.clone()) {
            break;
        }
        if material_id == source.id {
            source_is_in_chain = true;
            break;
        }
        cursor = archive_review_materials::Entity::find_by_id(material_id)
            .one(&transaction)
            .await?
            .and_then(|material| material.parent_material_id);
    }
    if !source_is_in_chain {
        return Err(ApiError::Conflict(
            "material version is not part of this archive draft's immutable review chain"
                .to_owned(),
        ));
    }
    let next_material_version = archive_review_materials::Entity::find()
        .filter(archive_review_materials::Column::CaseId.eq(&existing.case_id))
        .order_by_desc(archive_review_materials::Column::Version)
        .one(&transaction)
        .await?
        .map_or(Ok(1), |item| {
            item.version.checked_add(1).ok_or(ApiError::Internal)
        })?;
    let timestamp = now();
    let restored = archive_review_materials::ActiveModel {
        id: Set(case_service::new_id()),
        case_id: Set(existing.case_id.clone()),
        version: Set(next_material_version),
        parent_material_id: Set(Some(source.id.clone())),
        content: Set(source.content.clone()),
        source_scope_json: Set(source.source_scope_json.clone()),
        status: Set("deidentified".to_owned()),
        created_by_user_id: Set(auth.id.clone()),
        reviewed_by_user_id: Set(Some(auth.id.clone())),
        reviewed_at: Set(Some(timestamp.clone())),
        review_reason: Set(Some(reason.clone())),
        created_at: Set(timestamp.clone()),
    }
    .insert(&transaction)
    .await?;
    transaction.commit().await?;

    let ai_request = AiRequest {
        capability: AiCapability::CaseOrganization,
        data_level: DataLevel::Internal,
        purpose: AiPurpose::CaseArchiveDraft,
        data_region: "CN".to_owned(),
        system_instruction: Some("Return JSON only: {timeline:string[],lessons:string[],uncertainty:string}. Use only this administrator-approved de-identified material. Do not infer identities, exact locations, health details, causes, or operational outcomes.".to_owned()),
        output_schema: Some(archive_candidate_schema()),
        output_schema_name: Some("case_archive_candidate".to_owned()),
        input: serde_json::to_string(&json!({"deidentified_material": restored.content}))
            .map_err(|_| ApiError::Internal)?,
        requested_output_tokens: 500,
        template_version: "case-archive-ai-v2".to_owned(),
        input_scope_reference: format!("approved-deidentified-material:{}:v{}", restored.id, restored.version),
        redaction_policy_version: "archive-approved-material-v1".to_owned(),
    };
    let execution = gateway.execute(&ai_request).await;
    let execution_audits = crate::ai_gateway::execution_attempt_audits(&ai_request, &execution);
    let (content, provider_model) = match execution {
        AiExecutionResult::Completed { route, output, .. } => {
            match gateway.decode_json::<ArchiveOrganizationCandidate>(&output) {
                Ok(candidate) if valid_archive_candidate(&candidate) => {
                    (archive_candidate_content(candidate), Some(route.model))
                }
                _ => (
                    deterministic_archive_material_content(&restored.content),
                    None,
                ),
            }
        }
        _ => (
            deterministic_archive_material_content(&restored.content),
            None,
        ),
    };
    let next_draft_version = existing.version.checked_add(1).ok_or(ApiError::Internal)?;
    let transaction = db.begin().await?;
    let updated = archive_drafts::Entity::update_many()
        .col_expr(
            archive_drafts::Column::Status,
            Expr::value("pending_review"),
        )
        .col_expr(
            archive_drafts::Column::DeidentificationStatus,
            Expr::value("deidentified"),
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
        .col_expr(archive_drafts::Column::Content, Expr::value(content))
        .col_expr(
            archive_drafts::Column::ProviderModel,
            Expr::value(provider_model),
        )
        .col_expr(
            archive_drafts::Column::TemplateVersion,
            Expr::value("case-archive-ai-v2"),
        )
        .col_expr(
            archive_drafts::Column::ReviewMaterialId,
            Expr::value(Some(restored.id.clone())),
        )
        .col_expr(
            archive_drafts::Column::Version,
            Expr::value(next_draft_version),
        )
        .col_expr(
            archive_drafts::Column::UsageScope,
            Expr::value("internal_archive"),
        )
        .col_expr(
            archive_drafts::Column::RetentionStatus,
            Expr::value("retained"),
        )
        .col_expr(
            archive_drafts::Column::ReviewedByUserId,
            Expr::value(None::<String>),
        )
        .col_expr(
            archive_drafts::Column::ReviewedAt,
            Expr::value(None::<String>),
        )
        .col_expr(
            archive_drafts::Column::ReviewReason,
            Expr::value(None::<String>),
        )
        .col_expr(
            archive_drafts::Column::UpdatedAt,
            Expr::value(timestamp.clone()),
        )
        .filter(archive_drafts::Column::Id.eq(draft_id))
        .filter(archive_drafts::Column::Version.eq(existing.version))
        .filter(archive_drafts::Column::Status.eq(&existing.status))
        .filter(
            archive_drafts::Column::DeidentificationStatus.eq(&existing.deidentification_status),
        )
        .exec(&transaction)
        .await?;
    if updated.rows_affected != 1 {
        return Err(ApiError::Conflict(
            "archive draft changed before material restore could be recorded".to_owned(),
        ));
    }
    let model = archive_drafts::Entity::find_by_id(draft_id)
        .one(&transaction)
        .await?
        .ok_or(ApiError::Internal)?;
    crate::ai_gateway::persist_execution_audits(
        &transaction,
        &execution_audits,
        &auth.id,
        Some(&model.case_id),
    )
    .await?;
    case_service::write_audit(
        &transaction,
        Some(model.case_id.clone()),
        auth,
        "archive_review_material.restored",
        "archive_review_material",
        restored.id,
        Some(json!({
            "draft_id": draft_id,
            "source_version": version,
            "restored_version": next_material_version,
            "draft_version": model.version,
            "reason_length": reason.chars().count(),
        })),
    )
    .await?;
    transaction.commit().await?;
    archive_draft_response(model)
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
        publication_status: "reviewed_public".to_owned(),
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
    let source_record = case_source_records::Entity::find_by_id(request.source_record_id.trim())
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("controlled source record was not found".to_owned()))?;
    if source_record.case_id != case_id {
        return Err(ApiError::NotFound(
            "controlled source record was not found".to_owned(),
        ));
    }
    let text = source_record.content.clone();
    // Existing formal-clue lifecycle exposes only report categories. Preserve
    // the precise controlled record type on its linked source object.
    let source_type = if source_record.record_type == "field_feedback" {
        "field_report".to_owned()
    } else {
        "manual_report".to_owned()
    };
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
    let execution_audits = crate::ai_gateway::execution_attempt_audits(&ai_request, &execution);
    let (candidate, provider_model, degradation_status, _audit_status) = match execution {
        AiExecutionResult::Completed { route, output, .. } => {
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
    let timestamp = now();
    let transaction = db.begin().await?;
    let draft = clue_drafts::ActiveModel {
        id: Set(case_service::new_id()),
        case_id: Set(case_id.to_owned()),
        status: Set("draft".to_owned()),
        content: Set(text),
        source_type: Set(source_type.clone()),
        raw_record_reference: Set(source_record.source_reference.clone()),
        source_record_id: Set(Some(source_record.id.clone())),
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
    crate::ai_gateway::persist_execution_audits(
        &transaction,
        &execution_audits,
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
                let execution_audits =
                    crate::ai_gateway::execution_attempt_audits(&ai_request, &execution);
                let (content, provider_model, _audit_status) = match execution {
                    AiExecutionResult::Completed { route, output, .. } => {
                        match gateway.decode_json::<SummaryCandidate>(&output).and_then(
                            |candidate| {
                                validate_summary_candidate(candidate, &summary).map_err(|_| {
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
                    Some(execution_audits),
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
    if let Some(execution_audits) = execution_audit {
        crate::ai_gateway::persist_execution_audits(
            &transaction,
            &execution_audits,
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

/// Lists only human-published, publication-eligible summary versions for a
/// volunteer. Pending drafts and commander review notes stay restricted.
pub async fn list_published_summary_versions_for_volunteer(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<PublishedSummaryVersionResponse, ApiError> {
    case_service::require_case_role(db, &auth.id, case_id, &[CaseRole::Volunteer]).await?;
    let items = summary_drafts::Entity::find()
        .filter(summary_drafts::Column::CaseId.eq(case_id))
        .filter(summary_drafts::Column::PublicationEligible.eq(true))
        .filter(summary_drafts::Column::Status.is_in(["published", "superseded"]))
        .order_by_desc(summary_drafts::Column::Version)
        .all(db)
        .await?
        .into_iter()
        .map(|draft| PublishedSummaryVersion {
            version: draft.version,
            content: draft.content,
            // A published version keeps its original human-review timestamp
            // when a later version supersedes it.
            published_at: draft.reviewed_at.unwrap_or(draft.updated_at),
        })
        .collect();
    Ok(PublishedSummaryVersionResponse { items })
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
    poi_selection_token_secret: &str,
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
        .as_deref()
        .unwrap_or("hospital")
        .trim()
        .to_lowercase();
    if !matches!(
        category.as_str(),
        "hospital" | "police" | "transit" | "market" | "community_service"
    ) {
        return Err(ApiError::Validation("category is unsupported".to_owned()));
    }
    let (center, center_source) = poi_search_center(db, auth, case_id, role, &query, amap).await?;
    let (items, source, degradation_status, fallback_message) =
        match amap.search_nearby_pois(center, &category).await {
            PoiSearch::Available(pois) if !pois.is_empty() => (
                pois.into_iter()
                    .map(|poi| poi_item(poi, case_id, &auth.id, poi_selection_token_secret))
                    .collect(),
                "amap_webservice".to_owned(),
                "available".to_owned(),
                None,
            ),
            _ => (
                fallback_pois(&category),
                "fixed_demo_fallback".to_owned(),
                "degraded".to_owned(),
                Some(
                    "Nearby POI service is unavailable. Use the task area text and contact the commander for local confirmation."
                        .to_owned(),
                ),
            ),
        };
    Ok(CasePoiResponse {
        items,
        center_source,
        source,
        degradation_status,
        fallback_message,
    })
}

pub async fn get_case_poi_route(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    query: CasePoiRouteQuery,
    amap: &crate::amap_service::AmapService,
    poi_selection_token_secret: &str,
) -> Result<CasePoiRouteResponse, ApiError> {
    case_service::require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let browser_origin = Coordinate {
        longitude: query.browser_longitude,
        latitude: query.browser_latitude,
    };
    if !browser_origin.is_valid() {
        return Err(ApiError::Validation(
            "route coordinates are invalid".to_owned(),
        ));
    }
    let destination = selected_poi_destination(
        &query.selection_token,
        case_id,
        &auth.id,
        poi_selection_token_secret,
    )?;
    let origin = amap.convert_gps_coordinate(browser_origin).await.ok_or_else(|| {
        ApiError::Conflict(
            "browser location could not be converted for route planning; use the authorized case center instead"
                .to_owned(),
        )
    })?;
    let straight_line_meters = haversine_meters(origin, destination);
    match amap
        .estimate_route(origin, destination, RouteMode::Walking)
        .await
    {
        RouteEstimate::Available {
            distance_meters,
            duration_seconds,
            ..
        } => Ok(CasePoiRouteResponse {
            straight_line_meters,
            walking_distance_meters: Some(distance_meters),
            walking_duration_seconds: Some(duration_seconds),
            source: "amap_webservice".to_owned(),
            degradation_status: "available".to_owned(),
        }),
        RouteEstimate::Unavailable { .. } => Ok(CasePoiRouteResponse {
            straight_line_meters,
            walking_distance_meters: None,
            walking_duration_seconds: None,
            source: "straight_line_fallback".to_owned(),
            degradation_status: "degraded".to_owned(),
        }),
    }
}

async fn poi_search_center(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    role: CaseRole,
    query: &CasePoiQuery,
    amap: &crate::amap_service::AmapService,
) -> Result<(Coordinate, String), ApiError> {
    match (query.browser_longitude, query.browser_latitude) {
        (Some(longitude), Some(latitude)) => {
            let browser_coordinate = Coordinate {
                longitude,
                latitude,
            };
            if !browser_coordinate.is_valid() {
                return Err(ApiError::Validation(
                    "browser location is invalid".to_owned(),
                ));
            }
            let coordinate = amap.convert_gps_coordinate(browser_coordinate).await.ok_or_else(|| {
                ApiError::Conflict(
                    "browser location could not be converted for nearby search; use the authorized case center instead"
                        .to_owned(),
                )
            })?;
            Ok((coordinate, "browser_location".to_owned()))
        }
        (None, None) => Ok((
            authorized_center(db, auth, case_id, role).await?,
            "authorized_case_location".to_owned(),
        )),
        _ => Err(ApiError::Validation(
            "browser_longitude and browser_latitude must be provided together".to_owned(),
        )),
    }
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
        places = places.filter(case_places::Column::Visibility.is_in(["public", "confirmed"]));
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

fn poi_item(
    poi: crate::amap_service::Poi,
    case_id: &str,
    user_id: &str,
    poi_selection_token_secret: &str,
) -> CasePoiItem {
    let selection_token = poi.coordinate.and_then(|coordinate| {
        coordinate.is_valid().then(|| {
            issue_poi_selection_token(case_id, user_id, coordinate, poi_selection_token_secret)
        })
    });
    CasePoiItem {
        id: poi.id,
        name: poi.name,
        category: poi.category,
        address: poi.address,
        longitude: poi.coordinate.map(|coordinate| coordinate.longitude),
        latitude: poi.coordinate.map(|coordinate| coordinate.latitude),
        distance_meters: poi.distance_meters,
        selection_token,
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
        distance_meters: None,
        selection_token: None,
    }]
}

fn issue_poi_selection_token(
    case_id: &str,
    user_id: &str,
    destination: Coordinate,
    secret: &str,
) -> String {
    let payload = PoiSelectionTokenPayload {
        case_id: case_id.to_owned(),
        user_id: user_id.to_owned(),
        destination_longitude: destination.longitude,
        destination_latitude: destination.latitude,
        expires_at: Utc::now().timestamp() + POI_SELECTION_TOKEN_TTL_SECONDS,
    };
    let encoded_payload =
        hex::encode(serde_json::to_vec(&payload).expect("POI token payload serializes"));
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts token-secret key lengths");
    mac.update(encoded_payload.as_bytes());
    format!(
        "{encoded_payload}.{}",
        hex::encode(mac.finalize().into_bytes())
    )
}

fn selected_poi_destination(
    token: &str,
    case_id: &str,
    user_id: &str,
    secret: &str,
) -> Result<Coordinate, ApiError> {
    let invalid_selection = || {
        ApiError::Conflict(
            "selected POI is no longer valid; refresh nearby resources and try again".to_owned(),
        )
    };
    if token.len() > 2_048 {
        return Err(invalid_selection());
    }
    let (encoded_payload, signature) = token.split_once('.').ok_or_else(invalid_selection)?;
    let signature = hex::decode(signature).map_err(|_| invalid_selection())?;
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| ApiError::Internal)?;
    mac.update(encoded_payload.as_bytes());
    mac.verify_slice(&signature)
        .map_err(|_| invalid_selection())?;
    let payload = hex::decode(encoded_payload)
        .ok()
        .and_then(|value| serde_json::from_slice::<PoiSelectionTokenPayload>(&value).ok())
        .ok_or_else(invalid_selection)?;
    if payload.case_id != case_id
        || payload.user_id != user_id
        || payload.expires_at < Utc::now().timestamp()
    {
        return Err(invalid_selection());
    }
    let destination = Coordinate {
        longitude: payload.destination_longitude,
        latitude: payload.destination_latitude,
    };
    destination
        .is_valid()
        .then_some(destination)
        .ok_or_else(invalid_selection)
}

fn haversine_meters(origin: Coordinate, destination: Coordinate) -> u64 {
    let latitude_delta = (destination.latitude - origin.latitude).to_radians();
    let longitude_delta = (destination.longitude - origin.longitude).to_radians();
    let a = (latitude_delta / 2.0).sin().powi(2)
        + origin.latitude.to_radians().cos()
            * destination.latitude.to_radians().cos()
            * (longitude_delta / 2.0).sin().powi(2);
    (6_371_000.0 * 2.0 * a.sqrt().atan2((1.0 - a).sqrt())).round() as u64
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
        "properties": {
            "confirmed_information": { "type": "array", "items": { "type": "string" } },
            "pending_verification": { "type": "array", "items": { "type": "string" } },
            "excluded_directions": { "type": "array", "items": { "type": "string" } },
            "safety_reminders": { "type": "array", "items": { "type": "string" } },
            "uncertainty_notice": { "type": "string" }
        },
        "required": ["confirmed_information", "pending_verification", "excluded_directions", "safety_reminders", "uncertainty_notice"],
        "additionalProperties": false
    })
}

fn validate_summary_candidate(
    candidate: SummaryCandidate,
    summary: &crate::models::CaseSummaryResponse,
) -> Result<String, ()> {
    let confirmed = summary
        .confirmed_clues
        .iter()
        .map(|item| item.content.as_str())
        .collect::<std::collections::HashSet<_>>();
    let pending = summary
        .pending_verification
        .iter()
        .map(|item| item.content.as_str())
        .collect::<std::collections::HashSet<_>>();
    let bounded = |items: &[String]| {
        items.len() <= 20
            && items
                .iter()
                .all(|item| !item.trim().is_empty() && item.chars().count() <= 500)
    };
    if !bounded(&candidate.confirmed_information)
        || !bounded(&candidate.pending_verification)
        || !bounded(&candidate.excluded_directions)
        || !bounded(&candidate.safety_reminders)
        || candidate.uncertainty_notice.trim().is_empty()
        || candidate.uncertainty_notice.chars().count() > 500
    {
        return Err(());
    }
    if !candidate
        .confirmed_information
        .iter()
        .all(|item| confirmed.contains(item.trim()))
        || !candidate
            .pending_verification
            .iter()
            .all(|item| pending.contains(item.trim()))
    {
        return Err(());
    }
    Ok(format!(
        "Confirmed information:\n- {}\n\nPending verification:\n- {}\n\nExcluded directions:\n- {}\n\nSafety reminders:\n- {}\n\nUncertainty notice: {}",
        candidate.confirmed_information.join("\n- "),
        candidate.pending_verification.join("\n- "),
        candidate.excluded_directions.join("\n- "),
        candidate.safety_reminders.join("\n- "),
        candidate.uncertainty_notice.trim()
    ))
}

fn deterministic_archive_material_content(material: &str) -> String {
    let lines = material
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(12)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        "Timeline and retrospective draft require administrator-approved de-identified material."
            .to_owned()
    } else {
        format!(
            "Timeline candidates (manual review required):\n{}\n\nLessons candidate: preserve uncertainty and do not treat this draft as a knowledge-base entry.",
            lines.join("\n")
        )
    }
}

// Retain every occurrence and ordering signal while avoiding an unbounded
// quadratic diff for administrator-supplied material up to the input limit.
fn ordered_line_diff(from: &str, to: &str) -> (Vec<String>, Vec<String>) {
    let from_lines = from.lines().collect::<Vec<_>>();
    let to_lines = to.lines().collect::<Vec<_>>();
    let mut prefix = 0;
    while prefix < from_lines.len()
        && prefix < to_lines.len()
        && from_lines[prefix] == to_lines[prefix]
    {
        prefix += 1;
    }

    let mut from_end = from_lines.len();
    let mut to_end = to_lines.len();
    while from_end > prefix && to_end > prefix && from_lines[from_end - 1] == to_lines[to_end - 1] {
        from_end -= 1;
        to_end -= 1;
    }

    (
        to_lines[prefix..to_end]
            .iter()
            .map(|line| (*line).to_owned())
            .collect(),
        from_lines[prefix..from_end]
            .iter()
            .map(|line| (*line).to_owned())
            .collect(),
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

fn case_source_record_response(model: case_source_records::Model) -> CaseSourceRecordResponse {
    CaseSourceRecordResponse {
        id: model.id,
        case_id: model.case_id,
        record_type: model.record_type,
        content: model.content,
        occurred_at: model.occurred_at,
        source_reference: model.source_reference,
        created_at: model.created_at,
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
        review_material_id: model.review_material_id,
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

fn archive_review_material_response(
    model: archive_review_materials::Model,
    selected_id: Option<&str>,
) -> Result<ArchiveReviewMaterialResponse, ApiError> {
    let selected_for_ai = selected_id == Some(model.id.as_str());
    Ok(ArchiveReviewMaterialResponse {
        id: model.id,
        case_id: model.case_id,
        version: model.version,
        parent_material_id: model.parent_material_id,
        content: model.content,
        source_scope: serde_json::from_str(&model.source_scope_json)
            .map_err(|_| ApiError::Internal)?,
        status: model.status,
        created_by_user_id: model.created_by_user_id,
        reviewed_by_user_id: model.reviewed_by_user_id,
        reviewed_at: model.reviewed_at,
        review_reason: model.review_reason,
        created_at: model.created_at,
        selected_for_ai,
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
        source_record_id: model.source_record_id,
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

#[cfg(test)]
mod tests {
    use super::{Coordinate, issue_poi_selection_token, selected_poi_destination};

    const SECRET: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn poi_selection_token_is_bound_to_its_case_user_and_destination() {
        let destination = Coordinate {
            longitude: 116.404,
            latitude: 39.915,
        };
        let token = issue_poi_selection_token("case-a", "user-a", destination, SECRET);

        assert_eq!(
            selected_poi_destination(&token, "case-a", "user-a", SECRET)
                .expect("issued token should be accepted")
                .as_query_value(),
            destination.as_query_value()
        );
        assert!(selected_poi_destination(&token, "case-b", "user-a", SECRET).is_err());
        assert!(selected_poi_destination(&token, "case-a", "user-b", SECRET).is_err());
    }

    #[test]
    fn poi_selection_token_rejects_tampering() {
        let token = issue_poi_selection_token(
            "case-a",
            "user-a",
            Coordinate {
                longitude: 116.404,
                latitude: 39.915,
            },
            SECRET,
        );
        let (payload, signature) = token.split_once('.').expect("signed token format");
        let tampered = format!("{payload}.{}", signature.replacen('a', "b", 1));

        assert!(selected_poi_destination(&tampered, "case-a", "user-a", SECRET).is_err());
    }
}
