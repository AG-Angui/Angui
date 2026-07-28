use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::json;

use crate::{
    ai_gateway::{AiCapability, AiPurpose, AiRequest, DataLevel, GatewayDecision},
    amap_service::{Coordinate, PoiSearch},
    entities::{archive_drafts, case_places, cases, clue_drafts, clues, summary_drafts, tasks},
    error::ApiError,
    models::{
        ArchiveDraftResponse, AuthenticatedUser, CasePoiItem, CasePoiQuery, CasePoiResponse,
        CasePublicProgressItem, CasePublicProgressResponse, ClueDraftResponse,
        CreateClueDraftRequest, CreateSummaryDraftRequest, ReviewSummaryDraftRequest,
        SummaryDraftResponse,
    },
    roles::CaseRole,
    services::{case_service, case_summary_service, task_service},
};

const DRAFT_TEMPLATE_VERSION: &str = "case-summary-rule-v1";
const ARCHIVE_DRAFT_TEMPLATE_VERSION: &str = "case-archive-safe-metadata-v1";

pub async fn create_archive_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
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
    let timestamp = now();
    let model = archive_drafts::ActiveModel {
        id: Set(case_service::new_id()),
        case_id: Set(case_id.to_owned()),
        status: Set("draft".to_owned()),
        content: Set(deterministic_archive_draft_content(
            &case.status,
            confirmed_clue_count,
            completed_task_count,
        )),
        source_scope_json: Set(
            serde_json::to_string(&source_scope).map_err(|_| ApiError::Internal)?
        ),
        deidentification_status: Set("manual_review_required".to_owned()),
        template_version: Set(ARCHIVE_DRAFT_TEMPLATE_VERSION.to_owned()),
        provider_model: Set(None),
        created_by_user_id: Set(auth.id.clone()),
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
    let decision = gateway.route(&AiRequest {
        capability: AiCapability::StructuredExtraction,
        data_level: DataLevel::Collaborative,
        purpose: AiPurpose::ClueDraft,
        data_region: "CN".to_owned(),
        system_instruction: None,
        input: text.clone(),
        requested_output_tokens: 400,
        template_version: "clue-draft-rule-v1".to_owned(),
        input_scope_reference: "case_authorized_text".to_owned(),
        redaction_policy_version: "case-collaboration-v1".to_owned(),
    });
    let (provider_model, degradation_status) = match decision {
        GatewayDecision::Routed(route) => (Some(route.model), "manual_review_required"),
        GatewayDecision::Degraded { .. } => (None, "rule_based_fallback"),
    };
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
        created_by_user_id: Set(auth.id.clone()),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp),
    }
    .insert(&transaction)
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
    Ok(vec![clue_draft_response(draft)])
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
    let (content, publication_eligible, initial_status) = match request.content {
        Some(content) => (required_text("content", content, 12_000)?, false, "draft"),
        None => (
            deterministic_draft_content(&summary),
            true,
            "pending_review",
        ),
    };
    let ai_request = AiRequest {
        capability: AiCapability::CaseSummary,
        data_level: DataLevel::Internal,
        purpose: AiPurpose::CaseSummaryDraft,
        data_region: "CN".to_owned(),
        system_instruction: None,
        input: content.clone(),
        requested_output_tokens: 800,
        template_version: DRAFT_TEMPLATE_VERSION.to_owned(),
        input_scope_reference: "commander_case_summary".to_owned(),
        redaction_policy_version: "summary-draft-v1".to_owned(),
    };
    let provider_model = match gateway.route(&ai_request) {
        GatewayDecision::Routed(route) => Some(route.model),
        GatewayDecision::Degraded { .. } => None,
    };
    let timestamp = now();
    let scope = serde_json::to_string(&summary.source_scope).map_err(|_| ApiError::Internal)?;
    let transaction = db.begin().await?;
    let model = summary_drafts::ActiveModel {
        id: Set(case_service::new_id()),
        case_id: Set(case_id.to_owned()),
        status: Set(initial_status.to_owned()),
        content: Set(content),
        source_scope_json: Set(scope),
        template_version: Set(DRAFT_TEMPLATE_VERSION.to_owned()),
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
    case_service::write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "summary_draft.created",
        "summary_draft",
        model.id.clone(),
        Some(json!({ "status": initial_status, "template_version": DRAFT_TEMPLATE_VERSION, "publication_eligible": publication_eligible })),
    )
    .await?;
    transaction.commit().await?;
    response(model)
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
    if role == CaseRole::Commander
        && let Some(place) = case_places::Entity::find()
            .filter(case_places::Column::CaseId.eq(case_id))
            .filter(case_places::Column::ReviewStatus.eq("confirmed"))
            .filter(case_places::Column::Longitude.is_not_null())
            .filter(case_places::Column::Latitude.is_not_null())
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

fn deterministic_archive_draft_content(
    case_status: &str,
    confirmed_clue_count: u64,
    completed_task_count: u64,
) -> String {
    format!(
        "Internal archive draft. This record is not de-identified, publishable, indexable, exportable, or printable. Case status: {case_status}. Confirmed clue count: {confirmed_clue_count}. Completed task count: {completed_task_count}. Source scope is limited to counts and status metadata; it excludes raw clues, attachments, health notes, contacts, exact locations, routes, and task result text. Human de-identification and review are required before any separate reuse workflow."
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
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}

fn clue_draft_response(model: clue_drafts::Model) -> ClueDraftResponse {
    ClueDraftResponse {
        id: model.id,
        case_id: model.case_id,
        status: model.status,
        content: model.content,
        source_type: model.source_type,
        raw_record_reference: model.raw_record_reference,
        occurred_at: None,
        location_text: None,
        uncertainty_notice: model.uncertainty_notice,
        template_version: model.template_version,
        provider_model: model.provider_model,
        degradation_status: model.degradation_status,
    }
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
