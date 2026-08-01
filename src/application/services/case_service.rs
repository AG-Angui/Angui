use std::collections::HashMap;

use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, Order, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{
        audit_events, case_attachments, case_memberships, cases, clue_attachment_links,
        clue_attributions, clues, elder_profile_revisions, elder_profiles,
        user_global_capabilities, users,
    },
    error::ApiError,
    models::{
        AddCaseMemberRequest, AuthenticatedUser, CaseDetail, CaseListItem, CaseMemberResponse,
        ClueResponse, ClueTimelineQuery, ClueTimelineResponse, CommandIntakeCaseResponse,
        CreateCaseRequest, CreateClueRequest, ElderProfileResponse, ReviewClueRequest,
        UpdateCaseStatusRequest, UpdateElderProfileRequest,
    },
    roles::{AccountType, CaseRole, GlobalCapability},
};

const CASE_STATUSES: &[&str] = &["active", "resolved", "closed"];
const CLUE_REVIEW_STATUSES: &[&str] = &[
    "needs_verification",
    "confirmed",
    "rejected",
    "expired",
    "duplicate",
    "conflicting",
    "insufficient_information",
];
const CLUE_STATUSES: &[&str] = &[
    "pending_review",
    "needs_verification",
    "confirmed",
    "rejected",
    "expired",
    "duplicate",
    "conflicting",
    "insufficient_information",
];
const CLUE_SOURCE_TYPES: &[&str] = &["manual_report", "field_report", "chat_draft", "ai_draft"];
const PUBLIC_CLUE_SOURCE_TYPES: &[&str] = &["manual_report", "field_report"];
const CLUE_LOCATION_PRECISIONS: &[&str] = &["exact", "approximate", "unknown"];
const MAX_CLUE_TIMELINE_PAGE_SIZE: u64 = 100;

pub async fn list_cases(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
) -> Result<Vec<CaseListItem>, ApiError> {
    let memberships = case_memberships::Entity::find()
        .filter(case_memberships::Column::UserId.eq(&auth.id))
        .all(db)
        .await?;
    let roles: HashMap<_, _> = memberships
        .into_iter()
        .map(|membership| {
            Ok((
                membership.case_id,
                case_role_from_database(&membership.role)?,
            ))
        })
        .collect::<Result<_, ApiError>>()?;

    if roles.is_empty() {
        return Ok(Vec::new());
    }

    let rows = cases::Entity::find()
        .filter(cases::Column::Id.is_in(roles.keys().cloned()))
        .find_also_related(elder_profiles::Entity)
        .order_by_desc(cases::Column::CreatedAt)
        .all(db)
        .await?;

    rows.into_iter()
        .map(|(case_model, profile)| {
            let profile = profile.ok_or_else(|| {
                ApiError::Database(sea_orm::DbErr::Custom(format!(
                    "case {} is missing its elder profile",
                    case_model.id
                )))
            })?;
            let access_role = roles
                .get(&case_model.id)
                .cloned()
                .ok_or(ApiError::Internal)?;

            Ok(CaseListItem {
                id: case_model.id,
                case_code: case_model.case_code,
                status: case_model.status,
                access_role,
                display_name: profile.display_name,
                last_seen_at: profile.last_seen_at,
                last_seen_location: profile.last_seen_location,
                created_at: case_model.created_at,
                updated_at: case_model.updated_at,
            })
        })
        .collect()
}

pub async fn get_case(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<CaseDetail, ApiError> {
    let membership = membership_for_case(db, &auth.id, case_id).await?;
    load_case_detail(db, auth, membership).await
}

pub async fn list_command_intake(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
) -> Result<Vec<CommandIntakeCaseResponse>, ApiError> {
    if !auth
        .global_capabilities
        .contains(&GlobalCapability::Commander)
    {
        return Err(ApiError::Forbidden(
            "commander capability is required".to_owned(),
        ));
    }
    let rows = cases::Entity::find()
        .filter(cases::Column::Status.eq("active"))
        .find_also_related(elder_profiles::Entity)
        .order_by_desc(cases::Column::CreatedAt)
        .all(db)
        .await?;
    let accepted: std::collections::HashSet<String> = case_memberships::Entity::find()
        .filter(case_memberships::Column::Role.eq("commander"))
        .all(db)
        .await?
        .into_iter()
        .map(|membership| membership.case_id)
        .collect();
    Ok(rows
        .into_iter()
        .filter(|(case_model, _)| !accepted.contains(&case_model.id))
        .map(|(case_model, profile)| CommandIntakeCaseResponse {
            id: case_model.id,
            case_code: case_model.case_code,
            created_at: case_model.created_at,
            last_seen_at: profile.as_ref().and_then(|item| item.last_seen_at.clone()),
            area_hint: profile
                .as_ref()
                .and_then(|item| item.last_seen_location.clone()),
            elder_age: profile.and_then(|item| item.age),
        })
        .collect())
}

pub async fn accept_command_case(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<CaseDetail, ApiError> {
    if !auth
        .global_capabilities
        .contains(&GlobalCapability::Commander)
    {
        return Err(ApiError::Forbidden(
            "commander capability is required".to_owned(),
        ));
    }
    let transaction = db.begin().await?;
    let case_model = cases::Entity::find_by_id(case_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("case was not found".to_owned()))?;
    if case_model.status != "active" {
        return Err(ApiError::Conflict(
            "only active cases can be accepted".to_owned(),
        ));
    }
    if case_memberships::Entity::find()
        .filter(case_memberships::Column::CaseId.eq(case_id))
        .filter(case_memberships::Column::Role.eq("commander"))
        .one(&transaction)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict(
            "case has already been accepted by a commander".to_owned(),
        ));
    }
    insert_membership(
        &transaction,
        case_id,
        &auth.id,
        CaseRole::Commander,
        Some(&auth.id),
        &now(),
    )
    .await?;
    write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "case.commander_accepted",
        "case",
        case_id.to_owned(),
        None,
    )
    .await?;
    transaction.commit().await?;
    get_case(db, auth, case_id).await
}

pub async fn create_case(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    request: CreateCaseRequest,
) -> Result<CaseDetail, ApiError> {
    validate_case_request(&request)?;
    if !auth.account_type.can_join_cases() {
        return Err(ApiError::Forbidden(
            "this account cannot create cases".to_owned(),
        ));
    }
    let initial_case_role = CaseRole::Family;

    let transaction = db.begin().await?;
    let timestamp = now();
    let case_model = insert_case_records(&transaction, &request, &timestamp).await?;
    let case_id = case_model.id.clone();

    insert_membership(
        &transaction,
        &case_id,
        &auth.id,
        initial_case_role,
        Some(&auth.id),
        &timestamp,
    )
    .await?;

    write_audit(
        &transaction,
        Some(case_id.clone()),
        auth,
        "case.created",
        "case",
        case_id.clone(),
        Some(json!({ "status": "active" })),
    )
    .await?;

    transaction.commit().await?;
    get_case(db, auth, &case_id).await
}

pub async fn update_case_status(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: UpdateCaseStatusRequest,
) -> Result<CaseDetail, ApiError> {
    let next_status = request.status.trim().to_lowercase();
    if !CASE_STATUSES.contains(&next_status.as_str()) {
        return Err(ApiError::Validation(format!(
            "unsupported case status {next_status:?}"
        )));
    }

    let transaction = db.begin().await?;
    require_case_role(&transaction, &auth.id, case_id, &[CaseRole::Commander]).await?;
    let case_model = cases::Entity::find_by_id(case_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("case was not found".to_owned()))?;

    if !case_transition_allowed(&case_model.status, &next_status) {
        return Err(ApiError::Conflict(format!(
            "case status cannot change from {:?} to {:?}",
            case_model.status, next_status
        )));
    }

    let previous_status = case_model.status.clone();
    let mut active = case_model.into_active_model();
    active.status = Set(next_status.clone());
    active.updated_at = Set(now());
    active.update(&transaction).await?;

    write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "case.status_changed",
        "case",
        case_id.to_owned(),
        Some(json!({ "from": previous_status, "to": next_status })),
    )
    .await?;

    transaction.commit().await?;
    get_case(db, auth, case_id).await
}

pub async fn update_elder_profile(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: UpdateElderProfileRequest,
) -> Result<CaseDetail, ApiError> {
    let transaction = db.begin().await?;
    require_case_role(
        &transaction,
        &auth.id,
        case_id,
        &[CaseRole::Family, CaseRole::Commander],
    )
    .await?;
    validate_elder_profile_update(&request)?;
    let profile = elder_profiles::Entity::find()
        .filter(elder_profiles::Column::CaseId.eq(case_id))
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("case was not found".to_owned()))?;
    let previous = ElderProfileResponse::from(profile.clone());
    let mut active = profile.into_active_model();
    let mut changed_fields = Vec::new();
    if let Some(value) = request.display_name {
        let value = value.trim().to_owned();
        if previous.display_name != value {
            active.display_name = Set(value);
            changed_fields.push("display_name");
        }
    }
    if let Some(value) = request.age
        && previous.age != Some(value)
    {
        active.age = Set(Some(value));
        changed_fields.push("age");
    }
    if let Some(value) = request.gender {
        let value = trim_optional(Some(value));
        if previous.gender != value {
            active.gender = Set(value);
            changed_fields.push("gender");
        }
    }
    if let Some(value) = request.physical_description {
        let value = trim_optional(Some(value));
        if previous.physical_description != value {
            active.physical_description = Set(value);
            changed_fields.push("physical_description");
        }
    }
    if let Some(value) = request.clothing_description {
        let value = trim_optional(Some(value));
        if previous.clothing_description != value {
            active.clothing_description = Set(value);
            changed_fields.push("clothing_description");
        }
    }
    if let Some(value) = request.health_notes {
        let value = trim_optional(Some(value));
        if previous.health_notes != value {
            active.health_notes = Set(value);
            changed_fields.push("health_notes");
        }
    }
    if let Some(value) = request.last_seen_at {
        let value = trim_optional(Some(value));
        if previous.last_seen_at != value {
            active.last_seen_at = Set(value);
            changed_fields.push("last_seen_at");
        }
    }
    if let Some(value) = request.last_seen_location {
        let value = trim_optional(Some(value));
        if previous.last_seen_location != value {
            active.last_seen_location = Set(value);
            changed_fields.push("last_seen_location");
        }
    }
    if changed_fields.is_empty() {
        return Err(ApiError::Validation(
            "at least one changed elder profile field is required".to_owned(),
        ));
    }
    active.updated_at = Set(now());
    let updated = active.update(&transaction).await?;
    let timestamp = now();
    elder_profile_revisions::ActiveModel {
        id: Set(new_id()),
        elder_profile_id: Set(updated.id.clone()),
        case_id: Set(case_id.to_owned()),
        updated_by_user_id: Set(auth.id.clone()),
        previous_profile_json: Set(
            serde_json::to_string(&previous).map_err(|_| ApiError::Internal)?
        ),
        updated_profile_json: Set(serde_json::to_string(&ElderProfileResponse::from(
            updated.clone(),
        ))
        .map_err(|_| ApiError::Internal)?),
        created_at: Set(timestamp),
    }
    .insert(&transaction)
    .await?;
    write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "elder_profile.updated",
        "elder_profile",
        updated.id,
        Some(json!({ "changed_fields": changed_fields })),
    )
    .await?;
    transaction.commit().await?;
    get_case(db, auth, case_id).await
}

pub async fn create_clue(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: CreateClueRequest,
) -> Result<ClueResponse, ApiError> {
    let transaction = db.begin().await?;
    let response = create_clue_in_transaction(&transaction, auth, case_id, request).await?;
    transaction.commit().await?;
    Ok(response)
}

/// Creates a pending-review clue inside an existing transaction. AI-draft
/// promotion uses this to ensure accepting a draft, creating its formal clue,
/// recording the promotion link, and writing both audit events are atomic.
pub(crate) async fn create_clue_in_transaction<C: ConnectionTrait>(
    db: &C,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: CreateClueRequest,
) -> Result<ClueResponse, ApiError> {
    validate_clue_request(&request)?;
    require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let case_model = cases::Entity::find_by_id(case_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("case was not found".to_owned()))?;

    if case_model.status != "active" {
        return Err(ApiError::Conflict(
            "new clues can only be added to active cases".to_owned(),
        ));
    }

    let clue_id = new_id();
    let timestamp = now();
    let source_type = request
        .source_type
        .as_deref()
        .unwrap_or("manual_report")
        .trim()
        .to_lowercase();
    let location_precision =
        trim_optional(request.location_precision).map(|value| value.to_lowercase());
    let clue_model = clues::ActiveModel {
        id: Set(clue_id.clone()),
        case_id: Set(case_id.to_owned()),
        status: Set("pending_review".to_owned()),
        source: Set(request.source.trim().to_owned()),
        source_type: Set(source_type),
        content: Set(request.content.trim().to_owned()),
        raw_record_reference: Set(trim_optional(request.raw_record_reference)),
        occurred_at: Set(trim_optional(request.occurred_at)),
        reported_at: Set(timestamp.clone()),
        confirmed_at: Set(None),
        location_text: Set(trim_optional(request.location_text)),
        location_precision: Set(location_precision),
        next_action: Set(trim_optional(request.next_action)),
        linked_task_reference: Set(trim_optional(request.linked_task_reference)),
        related_clue_id: Set(None),
        relationship_type: Set(None),
        review_reason: Set(None),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp.clone()),
    }
    .insert(db)
    .await?;

    for attachment_id in &request.attachment_ids {
        let attachment = case_attachments::Entity::find_by_id(attachment_id)
            .one(db)
            .await?
            .filter(|attachment| attachment.case_id == case_id)
            .ok_or_else(|| {
                ApiError::Validation(
                    "attachment_ids must reference attachments in this case".to_owned(),
                )
            })?;
        if attachment.created_by_user_id != auth.id {
            return Err(ApiError::Forbidden(
                "an attachment can only be linked by its uploader".to_owned(),
            ));
        }
        clue_attachment_links::ActiveModel {
            clue_id: Set(clue_id.clone()),
            attachment_id: Set(attachment_id.clone()),
            created_at: Set(timestamp.clone()),
        }
        .insert(db)
        .await?;
    }

    let attribution = clue_attributions::ActiveModel {
        clue_id: Set(clue_id.clone()),
        submitted_by_user_id: Set(Some(auth.id.clone())),
        reviewed_by_user_id: Set(None),
        reviewed_at: Set(None),
    }
    .insert(db)
    .await?;

    write_audit(
        db,
        Some(case_id.to_owned()),
        auth,
        "clue.submitted",
        "clue",
        clue_id,
        Some(json!({ "status": "pending_review" })),
    )
    .await?;

    Ok(ClueResponse::new(
        clue_model,
        Some(attribution),
        &auth.id,
        request.attachment_ids,
    ))
}

pub async fn list_clues(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    query: ClueTimelineQuery,
) -> Result<ClueTimelineResponse, ApiError> {
    let query = ValidatedClueTimelineQuery::try_from(query)?;
    let membership = membership_for_case(db, &auth.id, case_id).await?;
    let case_role = case_role_from_database(&membership.role)?;
    let mut clue_query = clues::Entity::find().filter(clues::Column::CaseId.eq(case_id));

    if let Some(status) = &query.status {
        clue_query = clue_query.filter(clues::Column::Status.eq(status));
    }
    if let Some(source_type) = &query.source_type {
        clue_query = clue_query.filter(clues::Column::SourceType.eq(source_type));
    }

    clue_query = match query.sort.as_str() {
        "created_at" => clue_query.order_by(clues::Column::CreatedAt, query.order.clone()),
        "occurred_at" => clue_query
            .order_by(clues::Column::OccurredAt, query.order.clone())
            .order_by(clues::Column::CreatedAt, query.order.clone()),
        _ => return Err(ApiError::Internal),
    }
    .order_by(clues::Column::Id, query.order.clone());

    let clue_models = clue_query.all(db).await?;
    let clue_ids: Vec<_> = clue_models.iter().map(|clue| clue.id.clone()).collect();
    let attributions = clue_attributions_for_clues(db, clue_ids).await?;
    let attachment_links =
        clue_attachment_ids_for_clues(db, clue_models.iter().map(|clue| clue.id.clone()).collect())
            .await?;
    let visible_clues: Vec<_> = clue_models
        .into_iter()
        .filter_map(|clue| {
            let clue_id = clue.id.clone();
            visible_clue_response(
                clue,
                attributions.get(&clue_id).cloned(),
                attachment_links.get(&clue_id).cloned().unwrap_or_default(),
                auth,
                case_role,
            )
        })
        .collect();
    let visible_clues: Vec<_> = if let Some(query) = &query.q {
        visible_clues
            .into_iter()
            .filter(|clue| clue_matches_query(clue, query))
            .collect()
    } else {
        visible_clues
    };
    let total = u64::try_from(visible_clues.len()).map_err(|_| ApiError::Internal)?;
    let start = query.offset()?;
    let end = start
        .saturating_add(query.page_size_usize())
        .min(visible_clues.len());
    let items = visible_clues
        .into_iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect();

    Ok(ClueTimelineResponse {
        items,
        page: query.page,
        page_size: query.page_size,
        total,
    })
}

pub async fn review_clue(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    clue_id: &str,
    request: ReviewClueRequest,
) -> Result<ClueResponse, ApiError> {
    let next_status = request.status.trim().to_lowercase();
    if !CLUE_REVIEW_STATUSES.contains(&next_status.as_str()) {
        return Err(ApiError::Validation(format!(
            "unsupported reviewed clue status {next_status:?}"
        )));
    }
    let transaction = db.begin().await?;
    let clue_model = clues::Entity::find_by_id(clue_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("clue was not found".to_owned()))?;
    require_case_role(
        &transaction,
        &auth.id,
        &clue_model.case_id,
        &[CaseRole::Commander],
    )
    .await?;
    validate_review_request(&request, &next_status)?;
    let previous_status = clue_model.status.clone();
    let case_id = clue_model.case_id.clone();
    let related_clue_id = trim_optional(request.related_clue_id);
    if let Some(related_clue_id) = &related_clue_id {
        clues::Entity::find_by_id(related_clue_id)
            .one(&transaction)
            .await?
            .filter(|related| related.case_id == case_id && related.id != clue_id)
            .ok_or_else(|| {
                ApiError::Validation(
                    "related_clue_id must reference another clue in the same case".to_owned(),
                )
            })?;
    }
    if matches!(next_status.as_str(), "duplicate" | "conflicting") && related_clue_id.is_none() {
        return Err(ApiError::Validation(
            "duplicate and conflicting reviews require related_clue_id".to_owned(),
        ));
    }
    if matches!(next_status.as_str(), "duplicate" | "conflicting")
        && trim_optional(request.relationship_type.clone()).is_none()
    {
        return Err(ApiError::Validation(
            "duplicate and conflicting reviews require relationship_type".to_owned(),
        ));
    }
    let mut active = clue_model.into_active_model();
    active.status = Set(next_status.clone());
    if next_status == "confirmed" {
        active.confirmed_at = Set(Some(now()));
    } else {
        active.confirmed_at = Set(None);
    }
    active.related_clue_id = Set(related_clue_id.clone());
    let relationship_type = trim_optional(request.relationship_type);
    active.relationship_type = Set(relationship_type.clone());
    active.review_reason = Set(Some(request.reason.trim().to_owned()));
    if let Some(next_action) = request.next_action {
        active.next_action = Set(trim_optional(Some(next_action)));
    }
    if let Some(linked_task_reference) = request.linked_task_reference {
        active.linked_task_reference = Set(trim_optional(Some(linked_task_reference)));
    }
    active.updated_at = Set(now());
    let updated = active.update(&transaction).await?;

    let reviewed_at = now();
    let attribution = if let Some(existing) = clue_attributions::Entity::find_by_id(clue_id)
        .one(&transaction)
        .await?
    {
        let mut active = existing.into_active_model();
        active.reviewed_by_user_id = Set(Some(auth.id.clone()));
        active.reviewed_at = Set(Some(reviewed_at));
        active.update(&transaction).await?
    } else {
        clue_attributions::ActiveModel {
            clue_id: Set(clue_id.to_owned()),
            submitted_by_user_id: Set(None),
            reviewed_by_user_id: Set(Some(auth.id.clone())),
            reviewed_at: Set(Some(reviewed_at)),
        }
        .insert(&transaction)
        .await?
    };

    write_audit(
        &transaction,
        Some(case_id),
        auth,
        "clue.reviewed",
        "clue",
        clue_id.to_owned(),
        Some(json!({
            "from": previous_status,
            "to": next_status,
            "reason": request.reason.trim(),
            "related_clue_id": related_clue_id,
            "relationship_type": relationship_type,
        })),
    )
    .await?;

    transaction.commit().await?;
    let mut attachment_links = clue_attachment_ids_for_clues(db, vec![clue_id.to_owned()]).await?;
    let attachment_ids = attachment_links.remove(clue_id).unwrap_or_default();
    Ok(ClueResponse::new(
        updated,
        Some(attribution),
        &auth.id,
        attachment_ids,
    ))
}

struct ValidatedClueTimelineQuery {
    page: u64,
    page_size: u64,
    status: Option<String>,
    source_type: Option<String>,
    q: Option<String>,
    sort: String,
    order: Order,
}

fn clue_matches_query(clue: &ClueResponse, query: &str) -> bool {
    let normalized_query = query.to_lowercase();
    [
        clue.content.as_str(),
        clue.source.as_str(),
        clue.source_type.as_str(),
        clue.status.as_str(),
        clue.location_text.as_deref().unwrap_or_default(),
        clue.reported_at.as_str(),
        clue.occurred_at.as_deref().unwrap_or_default(),
    ]
    .into_iter()
    .any(|value| value.to_lowercase().contains(&normalized_query))
}

impl ValidatedClueTimelineQuery {
    fn offset(&self) -> Result<usize, ApiError> {
        let page_offset = self
            .page
            .checked_sub(1)
            .and_then(|page| page.checked_mul(self.page_size))
            .ok_or_else(|| ApiError::Validation("page is too large".to_owned()))?;
        usize::try_from(page_offset)
            .map_err(|_| ApiError::Validation("page is too large".to_owned()))
    }

    fn page_size_usize(&self) -> usize {
        self.page_size as usize
    }
}

impl TryFrom<ClueTimelineQuery> for ValidatedClueTimelineQuery {
    type Error = ApiError;

    fn try_from(value: ClueTimelineQuery) -> Result<Self, Self::Error> {
        let page = value.page.unwrap_or(1);
        if page == 0 {
            return Err(ApiError::Validation("page must be at least 1".to_owned()));
        }
        let page_size = value.page_size.unwrap_or(25);
        if page_size == 0 || page_size > MAX_CLUE_TIMELINE_PAGE_SIZE {
            return Err(ApiError::Validation(format!(
                "page_size must be between 1 and {MAX_CLUE_TIMELINE_PAGE_SIZE}"
            )));
        }
        let status = value.status.map(|status| status.trim().to_lowercase());
        if status
            .as_deref()
            .is_some_and(|status| !CLUE_STATUSES.contains(&status))
        {
            return Err(ApiError::Validation("status is unsupported".to_owned()));
        }
        let source_type = value
            .source_type
            .map(|source_type| source_type.trim().to_lowercase());
        if source_type
            .as_deref()
            .is_some_and(|source_type| !CLUE_SOURCE_TYPES.contains(&source_type))
        {
            return Err(ApiError::Validation(
                "source_type is unsupported".to_owned(),
            ));
        }
        let q = value.q.map(|query| query.trim().to_owned());
        if q.as_deref()
            .is_some_and(|query| query.is_empty() || query.chars().count() > 200)
        {
            return Err(ApiError::Validation(
                "q must be between 1 and 200 characters".to_owned(),
            ));
        }
        let sort = value
            .sort
            .unwrap_or_else(|| "created_at".to_owned())
            .trim()
            .to_lowercase();
        if !matches!(sort.as_str(), "created_at" | "occurred_at") {
            return Err(ApiError::Validation("sort is unsupported".to_owned()));
        }
        let order = match value
            .order
            .unwrap_or_else(|| "desc".to_owned())
            .trim()
            .to_lowercase()
            .as_str()
        {
            "asc" => Order::Asc,
            "desc" => Order::Desc,
            _ => return Err(ApiError::Validation("order is unsupported".to_owned())),
        };
        Ok(Self {
            page,
            page_size,
            status,
            source_type,
            q,
            sort,
            order,
        })
    }
}

pub async fn add_case_member(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: AddCaseMemberRequest,
) -> Result<CaseMemberResponse, ApiError> {
    let requested_case_role = request.case_role;
    let email = request.email.trim().to_lowercase();
    if email.is_empty() || email.len() > 320 || !email.contains('@') {
        return Err(ApiError::Validation("email is invalid".to_owned()));
    }

    let transaction = db.begin().await?;
    let acting_case_role = require_case_role(
        &transaction,
        &auth.id,
        case_id,
        &[CaseRole::Family, CaseRole::Commander],
    )
    .await?;
    if acting_case_role == CaseRole::Family
        && !matches!(requested_case_role, CaseRole::Family | CaseRole::Commander)
    {
        return Err(ApiError::Forbidden(
            "family members can only add a family member or commander".to_owned(),
        ));
    }
    let target = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .filter(users::Column::Status.eq("active"))
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("active user was not found".to_owned()))?;
    let target_account_type = account_type_from_database(&target.account_type)?;
    if !target_account_type.can_join_cases() {
        return Err(ApiError::Forbidden(
            "learning-only accounts cannot be assigned to cases".to_owned(),
        ));
    }
    let target_global_capabilities = global_capabilities_for_user(&transaction, &target.id).await?;
    if matches!(
        requested_case_role,
        CaseRole::Commander | CaseRole::Volunteer
    ) && !target_global_capabilities
        .iter()
        .copied()
        .any(|capability| capability.authorizes_case_role(requested_case_role))
    {
        return Err(ApiError::Forbidden(
            "the target account lacks the required global capability for this case role".to_owned(),
        ));
    }
    if case_memberships::Entity::find()
        .filter(case_memberships::Column::CaseId.eq(case_id))
        .filter(case_memberships::Column::UserId.eq(&target.id))
        .one(&transaction)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict(
            "user is already a member of this case".to_owned(),
        ));
    }

    insert_membership(
        &transaction,
        case_id,
        &target.id,
        requested_case_role,
        Some(&auth.id),
        &now(),
    )
    .await?;
    write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "case.member_added",
        "user",
        target.id.clone(),
        Some(json!({ "case_role": requested_case_role })),
    )
    .await?;
    transaction.commit().await?;

    Ok(CaseMemberResponse {
        user_id: target.id,
        email: target.email,
        display_name: target.display_name,
        account_type: target_account_type,
        global_capabilities: target_global_capabilities,
        case_role: requested_case_role,
    })
}

pub async fn list_case_members(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<Vec<CaseMemberResponse>, ApiError> {
    require_case_role(db, &auth.id, case_id, &[CaseRole::Commander]).await?;
    let memberships = case_memberships::Entity::find()
        .filter(case_memberships::Column::CaseId.eq(case_id))
        .order_by_asc(case_memberships::Column::Role)
        .order_by_asc(case_memberships::Column::UserId)
        .all(db)
        .await?;
    let mut members = Vec::with_capacity(memberships.len());
    for membership in memberships {
        let user = users::Entity::find_by_id(&membership.user_id)
            .one(db)
            .await?
            .ok_or(ApiError::Internal)?;
        members.push(CaseMemberResponse {
            user_id: user.id.clone(),
            email: user.email,
            display_name: user.display_name,
            account_type: account_type_from_database(&user.account_type)?,
            global_capabilities: global_capabilities_for_user(db, &user.id).await?,
            case_role: case_role_from_database(&membership.role)?,
        });
    }
    Ok(members)
}

async fn load_case_detail(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    membership: case_memberships::Model,
) -> Result<CaseDetail, ApiError> {
    let case_role = case_role_from_database(&membership.role)?;
    let case_model = cases::Entity::find_by_id(&membership.case_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("case was not found".to_owned()))?;
    let profile = elder_profiles::Entity::find()
        .filter(elder_profiles::Column::CaseId.eq(&membership.case_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            ApiError::Database(sea_orm::DbErr::Custom("elder profile missing".into()))
        })?;
    let clue_models = clues::Entity::find()
        .filter(clues::Column::CaseId.eq(&membership.case_id))
        .order_by_desc(clues::Column::CreatedAt)
        .all(db)
        .await?;
    let clue_ids: Vec<_> = clue_models.iter().map(|clue| clue.id.clone()).collect();
    let attributions = clue_attributions_for_clues(db, clue_ids).await?;
    let attachment_links =
        clue_attachment_ids_for_clues(db, clue_models.iter().map(|clue| clue.id.clone()).collect())
            .await?;
    let visible_clues = clue_models
        .into_iter()
        .filter_map(|clue| {
            let clue_id = clue.id.clone();
            visible_clue_response(
                clue,
                attributions.get(&clue_id).cloned(),
                attachment_links.get(&clue_id).cloned().unwrap_or_default(),
                auth,
                case_role,
            )
        })
        .collect();

    let profile_response: ElderProfileResponse = profile.into();
    let family_members = case_memberships::Entity::find()
        .filter(case_memberships::Column::CaseId.eq(&membership.case_id))
        .filter(case_memberships::Column::Role.eq(CaseRole::Family.to_string()))
        .all(db)
        .await?;
    let mut family_contact_emails = Vec::with_capacity(family_members.len());
    for member in family_members {
        if let Some(user) = users::Entity::find_by_id(member.user_id).one(db).await? {
            family_contact_emails.push(user.email);
        }
    }

    let (places, attachments) = futures_util::try_join!(
        crate::services::case_resource_service::visible_places(
            db,
            &membership.case_id,
            &auth.id,
            case_role,
        ),
        crate::services::case_resource_service::visible_attachments(
            db,
            &membership.case_id,
            &auth.id,
            case_role,
        ),
    )?;

    Ok(CaseDetail::new(
        case_model,
        profile_response,
        visible_clues,
        places,
        attachments,
        case_role,
        family_contact_emails,
    ))
}

async fn clue_attributions_for_clues<C: ConnectionTrait>(
    db: &C,
    clue_ids: Vec<String>,
) -> Result<HashMap<String, clue_attributions::Model>, ApiError> {
    if clue_ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(clue_attributions::Entity::find()
        .filter(clue_attributions::Column::ClueId.is_in(clue_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|attribution| (attribution.clue_id.clone(), attribution))
        .collect())
}

async fn clue_attachment_ids_for_clues<C: ConnectionTrait>(
    db: &C,
    clue_ids: Vec<String>,
) -> Result<HashMap<String, Vec<String>>, ApiError> {
    if clue_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let links = clue_attachment_links::Entity::find()
        .filter(clue_attachment_links::Column::ClueId.is_in(clue_ids))
        .all(db)
        .await?;
    Ok(links.into_iter().fold(HashMap::new(), |mut links, link| {
        links
            .entry(link.clue_id)
            .or_insert_with(Vec::new)
            .push(link.attachment_id);
        links
    }))
}

fn visible_clue_response(
    clue: clues::Model,
    attribution: Option<clue_attributions::Model>,
    attachment_ids: Vec<String>,
    auth: &AuthenticatedUser,
    case_role: CaseRole,
) -> Option<ClueResponse> {
    let own = attribution
        .as_ref()
        .and_then(|value| value.submitted_by_user_id.as_deref())
        == Some(auth.id.as_str());
    let visible = match case_role {
        CaseRole::Commander => true,
        CaseRole::Family => clue.status == "confirmed" || own,
        CaseRole::Volunteer => clue.status == "confirmed" || own,
    };
    let can_see_attachment_references = case_role == CaseRole::Commander || own;
    visible.then(|| {
        let mut response = ClueResponse::new(
            clue,
            attribution,
            &auth.id,
            if can_see_attachment_references {
                attachment_ids
            } else {
                Vec::new()
            },
        );
        if case_role != CaseRole::Commander && !own {
            // Confirmed facts can be shared for coordination, while controlled
            // source references, audit rationale, and internal task routing
            // remain available only to commanders and the submitter.
            response.raw_record_reference = None;
            response.review_reason = None;
            response.next_action = None;
            response.linked_task_reference = None;
            response.related_clue_id = None;
            response.relationship_type = None;
        }
        response
    })
}

async fn membership_for_case<C: ConnectionTrait>(
    db: &C,
    user_id: &str,
    case_id: &str,
) -> Result<case_memberships::Model, ApiError> {
    case_memberships::Entity::find()
        .filter(case_memberships::Column::CaseId.eq(case_id))
        .filter(case_memberships::Column::UserId.eq(user_id))
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("case was not found".to_owned()))
}

pub(crate) async fn require_case_role<C: ConnectionTrait>(
    db: &C,
    user_id: &str,
    case_id: &str,
    allowed_roles: &[CaseRole],
) -> Result<CaseRole, ApiError> {
    let membership = membership_for_case(db, user_id, case_id).await?;
    let case_role = case_role_from_database(&membership.role)?;
    if !allowed_roles.contains(&case_role) {
        return Err(ApiError::Forbidden(
            "this account cannot perform the requested case action".to_owned(),
        ));
    }
    Ok(case_role)
}

pub(crate) async fn insert_membership<C: ConnectionTrait>(
    db: &C,
    case_id: &str,
    user_id: &str,
    case_role: CaseRole,
    created_by_user_id: Option<&str>,
    created_at: &str,
) -> Result<(), ApiError> {
    case_memberships::ActiveModel {
        id: Set(new_id()),
        case_id: Set(case_id.to_owned()),
        user_id: Set(user_id.to_owned()),
        role: Set(case_role.to_string()),
        created_by_user_id: Set(created_by_user_id.map(str::to_owned)),
        created_at: Set(created_at.to_owned()),
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Creates the formal case and elder profile within a caller-owned transaction.
/// Intake confirmation uses the same implementation so a case cannot be
/// committed without its profile or its source-session link.
pub(crate) async fn insert_case_records<C: ConnectionTrait>(
    db: &C,
    request: &CreateCaseRequest,
    timestamp: &str,
) -> Result<cases::Model, ApiError> {
    validate_case_request(request)?;
    let case_id = new_id();
    let case_model = cases::ActiveModel {
        id: Set(case_id.clone()),
        case_code: Set(format!("AG-{}", case_id[..8].to_uppercase())),
        status: Set("active".to_owned()),
        created_at: Set(timestamp.to_owned()),
        updated_at: Set(timestamp.to_owned()),
    }
    .insert(db)
    .await?;

    elder_profiles::ActiveModel {
        id: Set(new_id()),
        case_id: Set(case_id),
        display_name: Set(request.display_name.trim().to_owned()),
        age: Set(request.age),
        gender: Set(trim_optional(request.gender.clone())),
        physical_description: Set(trim_optional(request.physical_description.clone())),
        clothing_description: Set(trim_optional(request.clothing_description.clone())),
        health_notes: Set(trim_optional(request.health_notes.clone())),
        last_seen_at: Set(trim_optional(request.last_seen_at.clone())),
        last_seen_location: Set(trim_optional(request.last_seen_location.clone())),
        created_at: Set(timestamp.to_owned()),
        updated_at: Set(timestamp.to_owned()),
    }
    .insert(db)
    .await?;

    Ok(case_model)
}

fn account_type_from_database(value: &str) -> Result<AccountType, ApiError> {
    AccountType::try_from(value).map_err(|error| {
        ApiError::Database(sea_orm::DbErr::Custom(format!(
            "users.account_type violates the account type constraint: {error}"
        )))
    })
}

async fn global_capabilities_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: &str,
) -> Result<Vec<GlobalCapability>, ApiError> {
    user_global_capabilities::Entity::find()
        .filter(user_global_capabilities::Column::UserId.eq(user_id))
        .order_by_asc(user_global_capabilities::Column::Capability)
        .all(db)
        .await?
        .into_iter()
        .map(|capability| {
            GlobalCapability::try_from(capability.capability.as_str()).map_err(|error| {
                ApiError::Database(sea_orm::DbErr::Custom(format!(
                    "user_global_capabilities.capability violates the capability constraint: {error}"
                )))
            })
        })
        .collect()
}

fn case_role_from_database(value: &str) -> Result<CaseRole, ApiError> {
    CaseRole::try_from(value).map_err(|error| {
        ApiError::Database(sea_orm::DbErr::Custom(format!(
            "case_memberships.role violates the case role constraint: {error}"
        )))
    })
}

pub(crate) async fn write_audit<C: ConnectionTrait>(
    db: &C,
    case_id: Option<String>,
    auth: &AuthenticatedUser,
    action: &str,
    entity_type: &str,
    entity_id: String,
    metadata: Option<serde_json::Value>,
) -> Result<(), ApiError> {
    let metadata = metadata.map(|mut value| {
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
        id: Set(new_id()),
        case_id: Set(case_id),
        actor: Set(auth.id.clone()),
        action: Set(action.to_owned()),
        entity_type: Set(entity_type.to_owned()),
        entity_id: Set(entity_id),
        metadata_json: Set(metadata),
        created_at: Set(now()),
    }
    .insert(db)
    .await?;
    Ok(())
}

fn validate_case_request(request: &CreateCaseRequest) -> Result<(), ApiError> {
    let name = request.display_name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(ApiError::Validation(
            "display_name must contain between 1 and 120 characters".to_owned(),
        ));
    }
    if request.age.is_some_and(|age| !(0..=130).contains(&age)) {
        return Err(ApiError::Validation(
            "age must be between 0 and 130".to_owned(),
        ));
    }
    if trim_optional(request.last_seen_location.clone()).is_none() {
        return Err(ApiError::Validation(
            "last_seen_location is required".to_owned(),
        ));
    }
    Ok(())
}

fn validate_elder_profile_update(request: &UpdateElderProfileRequest) -> Result<(), ApiError> {
    if request.display_name.is_none()
        && request.age.is_none()
        && request.gender.is_none()
        && request.physical_description.is_none()
        && request.clothing_description.is_none()
        && request.health_notes.is_none()
        && request.last_seen_at.is_none()
        && request.last_seen_location.is_none()
    {
        return Err(ApiError::Validation(
            "at least one elder profile field is required".to_owned(),
        ));
    }
    if let Some(display_name) = &request.display_name {
        let value = display_name.trim();
        if value.is_empty() || value.chars().count() > 120 {
            return Err(ApiError::Validation(
                "display_name must contain between 1 and 120 characters".to_owned(),
            ));
        }
    }
    if request.age.is_some_and(|age| !(0..=130).contains(&age)) {
        return Err(ApiError::Validation(
            "age must be between 0 and 130".to_owned(),
        ));
    }
    validate_optional_length("gender", &request.gender, 64)?;
    validate_optional_length("physical_description", &request.physical_description, 2000)?;
    validate_optional_length("clothing_description", &request.clothing_description, 2000)?;
    validate_optional_length("health_notes", &request.health_notes, 2000)?;
    validate_optional_length("last_seen_at", &request.last_seen_at, 40)?;
    validate_optional_length("last_seen_location", &request.last_seen_location, 500)?;
    if let Some(value) = request
        .last_seen_at
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        chrono::DateTime::parse_from_rfc3339(value).map_err(|_| {
            ApiError::Validation("last_seen_at must be an RFC 3339 timestamp".to_owned())
        })?;
    }
    Ok(())
}

fn validate_clue_request(request: &CreateClueRequest) -> Result<(), ApiError> {
    if request.source.trim().is_empty() || request.source.chars().count() > 64 {
        return Err(ApiError::Validation(
            "source must contain between 1 and 64 characters".to_owned(),
        ));
    }
    if request.content.trim().is_empty() || request.content.chars().count() > 4000 {
        return Err(ApiError::Validation(
            "content must contain between 1 and 4000 characters".to_owned(),
        ));
    }
    let source_type = request
        .source_type
        .as_deref()
        .unwrap_or("manual_report")
        .trim()
        .to_lowercase();
    if !PUBLIC_CLUE_SOURCE_TYPES.contains(&source_type.as_str()) {
        return Err(ApiError::Validation(
            "source_type is unsupported".to_owned(),
        ));
    }
    validate_optional_length("raw_record_reference", &request.raw_record_reference, 500)?;
    validate_optional_length("next_action", &request.next_action, 500)?;
    validate_optional_length("linked_task_reference", &request.linked_task_reference, 120)?;
    if let Some(precision) = request.location_precision.as_deref()
        && !CLUE_LOCATION_PRECISIONS.contains(&precision.trim().to_lowercase().as_str())
    {
        return Err(ApiError::Validation(
            "location_precision is unsupported".to_owned(),
        ));
    }
    if request
        .location_precision
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && trim_optional(request.location_text.clone()).is_none()
    {
        return Err(ApiError::Validation(
            "location_precision requires location_text".to_owned(),
        ));
    }
    if request.attachment_ids.len() > 10 {
        return Err(ApiError::Validation(
            "attachment_ids cannot contain more than 10 items".to_owned(),
        ));
    }
    let mut unique_ids = std::collections::HashSet::new();
    if request
        .attachment_ids
        .iter()
        .any(|attachment_id| attachment_id.trim().is_empty() || !unique_ids.insert(attachment_id))
    {
        return Err(ApiError::Validation(
            "attachment_ids must contain unique non-empty IDs".to_owned(),
        ));
    }
    Ok(())
}

fn validate_review_request(request: &ReviewClueRequest, next_status: &str) -> Result<(), ApiError> {
    if request.reason.trim().is_empty() || request.reason.chars().count() > 1000 {
        return Err(ApiError::Validation(
            "reason must contain between 1 and 1000 characters".to_owned(),
        ));
    }
    validate_optional_length("relationship_type", &request.relationship_type, 32)?;
    validate_optional_length("next_action", &request.next_action, 500)?;
    validate_optional_length("linked_task_reference", &request.linked_task_reference, 120)?;
    if let Some(relationship_type) = request.relationship_type.as_deref() {
        let relationship_type = relationship_type.trim().to_lowercase();
        if !matches!(
            relationship_type.as_str(),
            "duplicate_of" | "conflicts_with"
        ) {
            return Err(ApiError::Validation(
                "relationship_type is unsupported".to_owned(),
            ));
        }
        let expected = match next_status {
            "duplicate" => "duplicate_of",
            "conflicting" => "conflicts_with",
            _ => {
                return Err(ApiError::Validation(
                    "relationship_type requires duplicate or conflicting status".to_owned(),
                ));
            }
        };
        if relationship_type != expected {
            return Err(ApiError::Validation(format!(
                "relationship_type must be {expected} for {next_status}"
            )));
        }
    }
    Ok(())
}

fn validate_optional_length(
    label: &str,
    value: &Option<String>,
    maximum: usize,
) -> Result<(), ApiError> {
    if value
        .as_deref()
        .is_some_and(|value| value.trim().chars().count() > maximum)
    {
        return Err(ApiError::Validation(format!(
            "{label} cannot exceed {maximum} characters"
        )));
    }
    Ok(())
}

fn case_transition_allowed(current: &str, next: &str) -> bool {
    current == next
        || matches!(
            (current, next),
            ("active", "resolved")
                | ("active", "closed")
                | ("resolved", "active")
                | ("resolved", "closed")
        )
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

pub(crate) fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use migration::{Migrator, MigratorTrait};
    use sea_orm::{ColumnTrait, Database, EntityTrait, PaginatorTrait, QueryFilter};

    use super::{
        add_case_member, create_case, create_clue, get_case, list_cases, review_clue,
        update_case_status,
    };
    use crate::{
        entities::audit_events,
        error::ApiError,
        models::{
            AddCaseMemberRequest, CreateCaseRequest, CreateClueRequest, LoginRequest,
            ReviewClueRequest, UpdateCaseStatusRequest,
        },
        roles::CaseRole,
        services::auth_service,
    };

    async fn database() -> sea_orm::DatabaseConnection {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&database, None)
            .await
            .expect("migrations should succeed");
        auth_service::bootstrap_demo_users(&database, "demo-password-123")
            .await
            .expect("demo users should bootstrap");
        database
    }

    async fn sign_in(
        database: &sea_orm::DatabaseConnection,
        email: &str,
    ) -> crate::models::AuthenticatedUser {
        let login = auth_service::login(
            database,
            LoginRequest {
                email: email.to_owned(),
                password: "demo-password-123".to_owned(),
            },
            8,
        )
        .await
        .expect("login should succeed");
        auth_service::authenticate(database, &login.token)
            .await
            .expect("session should authenticate")
    }

    #[actix_web::test]
    async fn case_access_is_filtered_by_membership_and_role() {
        let database = database().await;
        let family = sign_in(&database, "family@demo.invalid").await;
        let commander = sign_in(&database, "commander@demo.invalid").await;
        let volunteer = sign_in(&database, "volunteer@demo.invalid").await;

        let case = create_case(
            &database,
            &family,
            CreateCaseRequest {
                display_name: "模拟老人 A".to_owned(),
                age: Some(76),
                gender: Some("female".to_owned()),
                physical_description: Some("短发，行动较慢".to_owned()),
                clothing_description: Some("蓝色外套".to_owned()),
                health_notes: Some("模拟认知障碍信息".to_owned()),
                last_seen_at: Some("2026-07-13T09:00:00Z".to_owned()),
                last_seen_location: Some("模拟公园北门".to_owned()),
            },
        )
        .await
        .expect("family should create a case");

        assert!(list_cases(&database, &commander).await.unwrap().is_empty());
        assert!(list_cases(&database, &volunteer).await.unwrap().is_empty());

        add_case_member(
            &database,
            &family,
            &case.id,
            AddCaseMemberRequest {
                email: "commander@demo.invalid".to_owned(),
                case_role: CaseRole::Commander,
            },
        )
        .await
        .expect("family should invite a commander");

        add_case_member(
            &database,
            &commander,
            &case.id,
            AddCaseMemberRequest {
                email: "volunteer@demo.invalid".to_owned(),
                case_role: CaseRole::Volunteer,
            },
        )
        .await
        .expect("commander should add a volunteer");

        let clue = create_clue(
            &database,
            &family,
            &case.id,
            CreateClueRequest {
                source: "family".to_owned(),
                content: "模拟线索：曾向市场方向步行".to_owned(),
                source_type: None,
                raw_record_reference: None,
                occurred_at: Some("2026-07-13T09:10:00Z".to_owned()),
                location_text: Some("模拟公园北门".to_owned()),
                location_precision: None,
                next_action: None,
                linked_task_reference: None,
                attachment_ids: Vec::new(),
            },
        )
        .await
        .expect("family should submit a clue");
        assert!(clue.is_own_submission);

        let family_view = get_case(&database, &family, &case.id).await.unwrap();
        assert_eq!(family_view.clues.len(), 1);
        let volunteer_view = get_case(&database, &volunteer, &case.id).await.unwrap();
        assert!(volunteer_view.clues.is_empty());
        assert!(volunteer_view.elder_profile.health_notes.is_some());

        let family_review = review_clue(
            &database,
            &family,
            &clue.id,
            ReviewClueRequest {
                status: "confirmed".to_owned(),
                reason: "family user cannot review".to_owned(),
                related_clue_id: None,
                relationship_type: None,
                next_action: None,
                linked_task_reference: None,
            },
        )
        .await;
        assert!(matches!(family_review, Err(ApiError::Forbidden(_))));

        review_clue(
            &database,
            &commander,
            &clue.id,
            ReviewClueRequest {
                status: "confirmed".to_owned(),
                reason: "commander reviewed source record".to_owned(),
                related_clue_id: None,
                relationship_type: None,
                next_action: None,
                linked_task_reference: None,
            },
        )
        .await
        .expect("commander should review clue");
        let volunteer_view = get_case(&database, &volunteer, &case.id).await.unwrap();
        assert_eq!(volunteer_view.clues.len(), 1);

        update_case_status(
            &database,
            &commander,
            &case.id,
            UpdateCaseStatusRequest {
                status: "resolved".to_owned(),
            },
        )
        .await
        .expect("commander should resolve case");

        let case_audits = audit_events::Entity::find()
            .filter(audit_events::Column::CaseId.eq(&case.id))
            .count(&database)
            .await
            .expect("audit count should succeed");
        assert_eq!(case_audits, 6);
    }
}
