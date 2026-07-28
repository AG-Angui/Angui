use std::collections::HashMap;

use chrono::DateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::json;

use crate::{
    entities::{audit_events, auth_sessions, user_global_capabilities, users},
    error::ApiError,
    models::{
        AdminAuditEventPage, AdminAuditEventQuery, AdminAuditEventResponse, AdminUserPage,
        AdminUserQuery, AdminUserResponse, AuthenticatedUser, UpdateAdminUserStatusRequest,
    },
    roles::{AccountType, GlobalCapability},
    services::case_service,
};

const MAX_PAGE_SIZE: u64 = 100;
const MAX_FILTER_LENGTH: usize = 128;
const USER_STATUSES: &[&str] = &["active", "disabled", "locked"];

pub async fn list_audit_events(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    query: AdminAuditEventQuery,
) -> Result<AdminAuditEventPage, ApiError> {
    require_admin(auth)?;
    let query = ValidatedAuditQuery::try_from(query)?;
    let transaction = db.begin().await?;
    let mut records = audit_events::Entity::find();
    if let Some(case_id) = &query.case_id {
        records = records.filter(audit_events::Column::CaseId.eq(case_id));
    }
    if let Some(entity_type) = &query.entity_type {
        records = records.filter(audit_events::Column::EntityType.eq(entity_type));
    }
    if let Some(action) = &query.action {
        records = records.filter(audit_events::Column::Action.eq(action));
    }
    if let Some(from) = &query.from {
        records = records.filter(audit_events::Column::CreatedAt.gte(from));
    }
    if let Some(to) = &query.to {
        records = records.filter(audit_events::Column::CreatedAt.lte(to));
    }
    records = match query.sort.as_str() {
        "created_at" => records.order_by(audit_events::Column::CreatedAt, query.order.clone()),
        "action" => records.order_by(audit_events::Column::Action, query.order.clone()),
        "entity_type" => records.order_by(audit_events::Column::EntityType, query.order.clone()),
        _ => return Err(ApiError::Internal),
    }
    .order_by(audit_events::Column::Id, query.order.clone());
    let total = records.clone().count(&transaction).await?;
    let items = records
        .offset(query.offset()?)
        .limit(query.page_size)
        .all(&transaction)
        .await?
        .into_iter()
        .map(audit_event_response)
        .collect();
    write_admin_audit(
        &transaction,
        auth,
        "admin.audit_events_listed",
        "admin_audit_query",
        auth.session_id.clone(),
        json!({
            "case_filter_applied": query.case_id.is_some(),
            "entity_type_filter_applied": query.entity_type.is_some(),
            "action_filter_applied": query.action.is_some(),
            "time_range_filter_applied": query.from.is_some() || query.to.is_some(),
            "page": query.page,
            "page_size": query.page_size,
            "sort": query.sort,
            "order": order_name(&query.order),
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(AdminAuditEventPage {
        items,
        page: query.page,
        page_size: query.page_size,
        total,
    })
}

pub async fn list_users(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    query: AdminUserQuery,
) -> Result<AdminUserPage, ApiError> {
    require_admin(auth)?;
    let query = ValidatedUserQuery::try_from(query)?;
    let transaction = db.begin().await?;
    let mut records = users::Entity::find();
    if let Some(account_type) = &query.account_type {
        records = records.filter(users::Column::AccountType.eq(account_type));
    }
    if let Some(status) = &query.status {
        records = records.filter(users::Column::Status.eq(status));
    }
    let records = records.all(&transaction).await?;
    let user_ids = records
        .iter()
        .map(|user| user.id.clone())
        .collect::<Vec<_>>();
    let capabilities = capabilities_for_users(&transaction, &user_ids).await?;
    let last_sessions = last_sessions_for_users(&transaction, &user_ids).await?;
    let mut items = records
        .into_iter()
        .map(|user| {
            let user_id = user.id.clone();
            admin_user_response(
                user,
                capabilities.get(&user_id).cloned().unwrap_or_default(),
                last_sessions.get(&user_id).cloned(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    sort_users(&mut items, &query);
    let total = u64::try_from(items.len()).map_err(|_| ApiError::Internal)?;
    let offset = query.offset()?;
    let end = offset
        .saturating_add(query.page_size as usize)
        .min(items.len());
    let items = items
        .into_iter()
        .skip(offset)
        .take(end.saturating_sub(offset))
        .collect();
    write_admin_audit(
        &transaction,
        auth,
        "admin.users_listed",
        "admin_user_query",
        auth.session_id.clone(),
        json!({
            "account_type_filter_applied": query.account_type.is_some(),
            "status_filter_applied": query.status.is_some(),
            "page": query.page,
            "page_size": query.page_size,
            "sort": query.sort,
            "order": order_name(&query.order),
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(AdminUserPage {
        items,
        page: query.page,
        page_size: query.page_size,
        total,
    })
}

pub async fn update_user_status(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    user_id: &str,
    request: UpdateAdminUserStatusRequest,
) -> Result<AdminUserResponse, ApiError> {
    require_admin(auth)?;
    let next_status = validate_status_change(&request)?;
    if user_id == auth.id {
        return Err(ApiError::Validation(
            "an administrator cannot change their own account status".to_owned(),
        ));
    }
    let transaction = db.begin().await?;
    let existing = users::Entity::find_by_id(user_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("user was not found".to_owned()))?;
    let previous_status = existing.status.clone();
    let changed = previous_status != next_status;
    if changed {
        let mut active = existing.clone().into_active_model();
        active.status = Set(next_status.clone());
        active.updated_at = Set(now());
        active.update(&transaction).await?;
    }
    let revoked_sessions = if next_status == "active" {
        0
    } else {
        revoke_active_sessions(&transaction, user_id).await?
    };
    write_admin_audit(
        &transaction,
        auth,
        "admin.user_status_changed",
        "user",
        user_id.to_owned(),
        json!({
            "previous_status": previous_status,
            "next_status": next_status,
            "changed": changed,
            "revoked_session_count": revoked_sessions,
            "reason_length": request.reason.trim().chars().count(),
        }),
    )
    .await?;
    transaction.commit().await?;
    load_admin_user(db, user_id).await
}

fn require_admin(auth: &AuthenticatedUser) -> Result<(), ApiError> {
    auth.global_capabilities
        .contains(&GlobalCapability::Admin)
        .then_some(())
        .ok_or_else(|| {
            ApiError::Forbidden("only administrators can perform this action".to_owned())
        })
}

fn audit_event_response(model: audit_events::Model) -> AdminAuditEventResponse {
    AdminAuditEventResponse {
        id: model.id,
        case_id: model.case_id,
        actor_user_id: model.actor,
        action: model.action,
        entity_type: model.entity_type,
        entity_id: model.entity_id,
        created_at: model.created_at,
    }
}

async fn load_admin_user(
    db: &DatabaseConnection,
    user_id: &str,
) -> Result<AdminUserResponse, ApiError> {
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("user was not found".to_owned()))?;
    let capabilities = capabilities_for_users(db, std::slice::from_ref(&user.id)).await?;
    let last_sessions = last_sessions_for_users(db, std::slice::from_ref(&user.id)).await?;
    admin_user_response(
        user.clone(),
        capabilities.get(&user.id).cloned().unwrap_or_default(),
        last_sessions.get(&user.id).cloned(),
    )
}

async fn capabilities_for_users<C: sea_orm::ConnectionTrait>(
    db: &C,
    user_ids: &[String],
) -> Result<HashMap<String, Vec<GlobalCapability>>, ApiError> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut grouped = HashMap::<String, Vec<GlobalCapability>>::new();
    for capability in user_global_capabilities::Entity::find()
        .filter(user_global_capabilities::Column::UserId.is_in(user_ids.iter().cloned()))
        .order_by_asc(user_global_capabilities::Column::Capability)
        .all(db)
        .await?
    {
        let parsed = GlobalCapability::try_from(capability.capability.as_str()).map_err(|error| {
            ApiError::Database(sea_orm::DbErr::Custom(format!(
                "user_global_capabilities.capability violates the capability constraint: {error}"
            )))
        })?;
        grouped.entry(capability.user_id).or_default().push(parsed);
    }
    Ok(grouped)
}

async fn last_sessions_for_users<C: sea_orm::ConnectionTrait>(
    db: &C,
    user_ids: &[String],
) -> Result<HashMap<String, String>, ApiError> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(auth_sessions::Entity::find()
        .filter(auth_sessions::Column::UserId.is_in(user_ids.iter().cloned()))
        .order_by_desc(auth_sessions::Column::LastUsedAt)
        .all(db)
        .await?
        .into_iter()
        .fold(HashMap::new(), |mut latest, session| {
            latest
                .entry(session.user_id)
                .or_insert(session.last_used_at);
            latest
        }))
}

fn admin_user_response(
    user: users::Model,
    global_capabilities: Vec<GlobalCapability>,
    last_session_at: Option<String>,
) -> Result<AdminUserResponse, ApiError> {
    let account_type = AccountType::try_from(user.account_type.as_str()).map_err(|error| {
        ApiError::Database(sea_orm::DbErr::Custom(format!(
            "users.account_type violates the account type constraint: {error}"
        )))
    })?;
    Ok(AdminUserResponse {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        account_type,
        global_capabilities,
        status: user.status,
        created_at: user.created_at,
        last_session_at,
    })
}

async fn revoke_active_sessions<C: sea_orm::ConnectionTrait>(
    db: &C,
    user_id: &str,
) -> Result<u64, ApiError> {
    let result = auth_sessions::Entity::update_many()
        .col_expr(
            auth_sessions::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now()),
        )
        .filter(auth_sessions::Column::UserId.eq(user_id))
        .filter(auth_sessions::Column::RevokedAt.is_null())
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

async fn write_admin_audit<C: sea_orm::ConnectionTrait>(
    db: &C,
    auth: &AuthenticatedUser,
    action: &str,
    entity_type: &str,
    entity_id: String,
    metadata: serde_json::Value,
) -> Result<(), ApiError> {
    case_service::write_audit(
        db,
        None,
        auth,
        action,
        entity_type,
        entity_id,
        Some(metadata),
    )
    .await
}

fn validate_status_change(request: &UpdateAdminUserStatusRequest) -> Result<String, ApiError> {
    let status = request.status.trim().to_lowercase();
    if !USER_STATUSES.contains(&status.as_str()) {
        return Err(ApiError::Validation("status is unsupported".to_owned()));
    }
    let reason = request.reason.trim();
    if reason.is_empty() || reason.chars().count() > 1_000 {
        return Err(ApiError::Validation(
            "reason must contain between 1 and 1000 characters".to_owned(),
        ));
    }
    Ok(status)
}

fn validate_identifier_filter(
    label: &str,
    value: Option<String>,
) -> Result<Option<String>, ApiError> {
    value
        .map(|value| {
            let value = value.trim().to_owned();
            if value.is_empty()
                || value.chars().count() > MAX_FILTER_LENGTH
                || !value.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
                })
            {
                return Err(ApiError::Validation(format!("{label} is invalid")));
            }
            Ok(value)
        })
        .transpose()
}

fn validate_timestamp_filter(
    label: &str,
    value: Option<String>,
) -> Result<Option<String>, ApiError> {
    value
        .map(|value| {
            let value = value.trim().to_owned();
            DateTime::parse_from_rfc3339(&value).map_err(|_| {
                ApiError::Validation(format!("{label} must be an RFC 3339 timestamp"))
            })?;
            Ok(value)
        })
        .transpose()
}

#[derive(Clone)]
struct ValidatedAuditQuery {
    case_id: Option<String>,
    entity_type: Option<String>,
    action: Option<String>,
    from: Option<String>,
    to: Option<String>,
    page: u64,
    page_size: u64,
    sort: String,
    order: sea_orm::Order,
}

impl ValidatedAuditQuery {
    fn offset(&self) -> Result<u64, ApiError> {
        self.page
            .checked_sub(1)
            .and_then(|page| page.checked_mul(self.page_size))
            .ok_or_else(|| ApiError::Validation("page is too large".to_owned()))
    }
}

impl TryFrom<AdminAuditEventQuery> for ValidatedAuditQuery {
    type Error = ApiError;

    fn try_from(value: AdminAuditEventQuery) -> Result<Self, Self::Error> {
        let (page, page_size) = validate_pagination(value.page, value.page_size)?;
        let sort = validate_sort(value.sort, &["created_at", "action", "entity_type"])?;
        let order = validate_order(value.order)?;
        let from = validate_timestamp_filter("from", value.from)?;
        let to = validate_timestamp_filter("to", value.to)?;
        if from
            .as_ref()
            .zip(to.as_ref())
            .is_some_and(|(from, to)| from > to)
        {
            return Err(ApiError::Validation("from must not be after to".to_owned()));
        }
        Ok(Self {
            case_id: validate_identifier_filter("case_id", value.case_id)?,
            entity_type: validate_identifier_filter("entity_type", value.entity_type)?,
            action: validate_identifier_filter("action", value.action)?,
            from,
            to,
            page,
            page_size,
            sort,
            order,
        })
    }
}

#[derive(Clone)]
struct ValidatedUserQuery {
    account_type: Option<String>,
    status: Option<String>,
    page: u64,
    page_size: u64,
    sort: String,
    order: sea_orm::Order,
}

impl ValidatedUserQuery {
    fn offset(&self) -> Result<usize, ApiError> {
        let offset = self
            .page
            .checked_sub(1)
            .and_then(|page| page.checked_mul(self.page_size))
            .ok_or_else(|| ApiError::Validation("page is too large".to_owned()))?;
        usize::try_from(offset).map_err(|_| ApiError::Validation("page is too large".to_owned()))
    }
}

impl TryFrom<AdminUserQuery> for ValidatedUserQuery {
    type Error = ApiError;

    fn try_from(value: AdminUserQuery) -> Result<Self, Self::Error> {
        let (page, page_size) = validate_pagination(value.page, value.page_size)?;
        let account_type = value.account_type.map(|value| value.trim().to_lowercase());
        if account_type
            .as_deref()
            .is_some_and(|value| !matches!(value, "member" | "learner"))
        {
            return Err(ApiError::Validation(
                "account_type is unsupported".to_owned(),
            ));
        }
        let status = value.status.map(|value| value.trim().to_lowercase());
        if status
            .as_deref()
            .is_some_and(|value| !USER_STATUSES.contains(&value))
        {
            return Err(ApiError::Validation("status is unsupported".to_owned()));
        }
        Ok(Self {
            account_type,
            status,
            page,
            page_size,
            sort: validate_sort(value.sort, &["created_at", "email", "last_session_at"])?,
            order: validate_order(value.order)?,
        })
    }
}

fn validate_pagination(page: Option<u64>, page_size: Option<u64>) -> Result<(u64, u64), ApiError> {
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(25);
    if page == 0 {
        return Err(ApiError::Validation("page must be at least 1".to_owned()));
    }
    if page_size == 0 || page_size > MAX_PAGE_SIZE {
        return Err(ApiError::Validation(format!(
            "page_size must be between 1 and {MAX_PAGE_SIZE}"
        )));
    }
    Ok((page, page_size))
}

fn validate_sort(value: Option<String>, allowed: &[&str]) -> Result<String, ApiError> {
    let sort = value
        .unwrap_or_else(|| "created_at".to_owned())
        .trim()
        .to_lowercase();
    allowed
        .contains(&sort.as_str())
        .then_some(sort)
        .ok_or_else(|| ApiError::Validation("sort is unsupported".to_owned()))
}

fn validate_order(value: Option<String>) -> Result<sea_orm::Order, ApiError> {
    match value
        .unwrap_or_else(|| "desc".to_owned())
        .trim()
        .to_lowercase()
        .as_str()
    {
        "asc" => Ok(sea_orm::Order::Asc),
        "desc" => Ok(sea_orm::Order::Desc),
        _ => Err(ApiError::Validation("order is unsupported".to_owned())),
    }
}

fn order_name(order: &sea_orm::Order) -> &'static str {
    match order {
        sea_orm::Order::Asc => "asc",
        sea_orm::Order::Desc => "desc",
        sea_orm::Order::Field(_) => "field",
    }
}

fn sort_users(items: &mut [AdminUserResponse], query: &ValidatedUserQuery) {
    items.sort_by(|left, right| {
        let primary = match query.sort.as_str() {
            "created_at" => left.created_at.cmp(&right.created_at),
            "email" => left.email.cmp(&right.email),
            "last_session_at" => left.last_session_at.cmp(&right.last_session_at),
            _ => std::cmp::Ordering::Equal,
        };
        let primary = if query.order == sea_orm::Order::Desc {
            primary.reverse()
        } else {
            primary
        };
        primary.then_with(|| left.id.cmp(&right.id))
    });
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
