use crate::{
    entities::{
        access_requests, auth_email_tokens, message_deliveries, user_global_capabilities, users,
    },
    error::ApiError,
    integrations::message_delivery::MessageDelivery,
    models::AuthenticatedUser,
    models::{
        AccessRequestResponse, AdminAccessRequestResponse, CreateAccessRequest,
        PasswordSetupRequest, ReviewAccessRequest,
    },
    roles::{AccountType, GlobalCapability},
};
use chrono::{Duration, SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const GENERIC_MESSAGE: &str = "如果邮箱可以申请访问，我们会发送一封验证邮件，请查收后继续。";
const VERIFY_TTL_HOURS: i64 = 24;
const SETUP_TTL_HOURS: i64 = 24;

pub async fn create(
    db: &DatabaseConnection,
    delivery: &MessageDelivery,
    request: CreateAccessRequest,
    frontend_origin: &str,
) -> Result<AccessRequestResponse, ApiError> {
    let email = normalize_email(&request.email)?;
    validate_name(&request.display_name)?;
    validate_role(&request.requested_role)?;
    let now = now();
    let transaction = db.begin().await?;
    let existing = access_requests::Entity::find()
        .filter(access_requests::Column::Email.eq(&email))
        .one(&transaction)
        .await?;
    let id = existing
        .as_ref()
        .map(|v| v.id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    if let Some(existing) = existing {
        let mut model = existing.into_active_model();
        model.display_name = Set(request.display_name.trim().to_owned());
        model.requested_role = Set(request.requested_role.trim().to_lowercase());
        model.status = Set("pending_verification".to_owned());
        model.email_verified_at = Set(None);
        model.updated_at = Set(now.clone());
        model.update(&transaction).await?;
    } else {
        access_requests::ActiveModel {
            id: Set(id.clone()),
            email: Set(email.clone()),
            display_name: Set(request.display_name.trim().to_owned()),
            requested_role: Set(request.requested_role.trim().to_lowercase()),
            status: Set("pending_verification".to_owned()),
            email_verified_at: Set(None),
            reviewed_by_user_id: Set(None),
            reviewed_at: Set(None),
            review_reason: Set(None),
            created_user_id: Set(None),
            created_at: Set(now.clone()),
            updated_at: Set(now.clone()),
        }
        .insert(&transaction)
        .await?;
    }
    auth_email_tokens::Entity::delete_many()
        .filter(auth_email_tokens::Column::AccessRequestId.eq(&id))
        .exec(&transaction)
        .await?;
    let raw = token();
    auth_email_tokens::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        access_request_id: Set(Some(id.clone())),
        user_id: Set(None),
        purpose: Set("access_request_verify".to_owned()),
        token_hash: Set(hash_token(&raw)),
        expires_at: Set((Utc::now() + Duration::hours(VERIFY_TTL_HOURS))
            .to_rfc3339_opts(SecondsFormat::Millis, true)),
        consumed_at: Set(None),
        created_at: Set(now.clone()),
    }
    .insert(&transaction)
    .await?;
    transaction.commit().await?;
    let receipt = delivery
        .send(
            &email,
            "安归访问申请验证",
            &format!(
                "请打开链接验证邮箱：{}/#access-verify={}",
                frontend_origin.trim_end_matches('/'),
                raw
            ),
        )
        .await;
    let _ = record_delivery(
        db,
        "access_request",
        &id,
        "access_request_verify",
        receipt.status.as_str(),
        receipt.reason,
    )
    .await;
    Ok(AccessRequestResponse {
        id,
        status: "pending_verification".to_owned(),
        message: GENERIC_MESSAGE.to_owned(),
    })
}

pub async fn verify(db: &DatabaseConnection, raw: &str) -> Result<AccessRequestResponse, ApiError> {
    let token = auth_email_tokens::Entity::find()
        .filter(auth_email_tokens::Column::TokenHash.eq(hash_token(raw)))
        .filter(auth_email_tokens::Column::Purpose.eq("access_request_verify"))
        .filter(auth_email_tokens::Column::ConsumedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(|| {
            ApiError::Unauthorized("verification link is invalid or expired".to_owned())
        })?;
    if token.expires_at <= now() {
        return Err(ApiError::Unauthorized(
            "verification link is invalid or expired".to_owned(),
        ));
    }
    let request_id = token.access_request_id.clone().ok_or(ApiError::Internal)?;
    let transaction = db.begin().await?;
    let request = access_requests::Entity::find_by_id(&request_id)
        .one(&transaction)
        .await?
        .ok_or(ApiError::Internal)?;
    let mut token_model = token.into_active_model();
    token_model.consumed_at = Set(Some(now()));
    token_model.update(&transaction).await?;
    let mut request_model = request.into_active_model();
    request_model.status = Set("pending_review".to_owned());
    request_model.email_verified_at = Set(Some(now()));
    request_model.updated_at = Set(now());
    request_model.update(&transaction).await?;
    transaction.commit().await?;
    Ok(AccessRequestResponse {
        id: request_id,
        status: "pending_review".to_owned(),
        message: "邮箱已验证，申请进入人工审核。".to_owned(),
    })
}

pub async fn list(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
) -> Result<Vec<AdminAccessRequestResponse>, ApiError> {
    require_admin(auth)?;
    Ok(access_requests::Entity::find()
        .order_by_desc(access_requests::Column::CreatedAt)
        .all(db)
        .await?
        .into_iter()
        .map(|r| AdminAccessRequestResponse {
            id: r.id,
            email: r.email,
            display_name: r.display_name,
            requested_role: r.requested_role,
            status: r.status,
            email_verified_at: r.email_verified_at,
            created_at: r.created_at,
        })
        .collect())
}

pub async fn review(
    db: &DatabaseConnection,
    delivery: &MessageDelivery,
    auth: &AuthenticatedUser,
    id: &str,
    input: ReviewAccessRequest,
    frontend_origin: &str,
) -> Result<AdminAccessRequestResponse, ApiError> {
    require_admin(auth)?;
    let action = input.action.trim().to_lowercase();
    if !["approve", "reject"].contains(&action.as_str()) {
        return Err(ApiError::Validation(
            "action must be approve or reject".to_owned(),
        ));
    }
    if action == "reject" && input.reason.as_deref().unwrap_or("").trim().is_empty() {
        return Err(ApiError::Validation(
            "rejection reason is required".to_owned(),
        ));
    }
    let request = access_requests::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("access request was not found".to_owned()))?;
    if request.status != "pending_review" {
        return Err(ApiError::Conflict(
            "access request is not awaiting review".to_owned(),
        ));
    }
    let transaction = db.begin().await?;
    let mut model = request.clone().into_active_model();
    let status = if action == "approve" {
        "approved"
    } else {
        "rejected"
    };
    model.status = Set(status.to_owned());
    model.reviewed_by_user_id = Set(Some(auth.id.clone()));
    model.reviewed_at = Set(Some(now()));
    model.review_reason = Set(input.reason.clone());
    model.updated_at = Set(now());
    if action == "approve" {
        let role = input.role.unwrap_or(request.requested_role.clone());
        let (account_type, capabilities) = role_mapping(&role)?;
        let user_id = Uuid::new_v4().to_string();
        let password_hash = empty_password_hash();
        users::ActiveModel {
            id: Set(user_id.clone()),
            email: Set(request.email.clone()),
            display_name: Set(request.display_name.clone()),
            account_type: Set(account_type.to_string()),
            password_hash: Set(password_hash),
            status: Set("disabled".to_owned()),
            created_at: Set(now()),
            updated_at: Set(now()),
        }
        .insert(&transaction)
        .await?;
        for capability in capabilities {
            user_global_capabilities::ActiveModel {
                user_id: Set(user_id.clone()),
                capability: Set(capability.to_string()),
                created_at: Set(now()),
            }
            .insert(&transaction)
            .await?;
        }
        model.created_user_id = Set(Some(user_id.clone()));
        let raw = token();
        auth_email_tokens::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            access_request_id: Set(None),
            user_id: Set(Some(user_id.clone())),
            purpose: Set("password_setup".to_owned()),
            token_hash: Set(hash_token(&raw)),
            expires_at: Set((Utc::now() + Duration::hours(SETUP_TTL_HOURS))
                .to_rfc3339_opts(SecondsFormat::Millis, true)),
            consumed_at: Set(None),
            created_at: Set(now()),
        }
        .insert(&transaction)
        .await?;
        transaction.commit().await?;
        let receipt = delivery
            .send(
                &request.email,
                "安归账号设置密码",
                &format!(
                    "请设置密码：{}/#password-setup={}",
                    frontend_origin.trim_end_matches('/'),
                    raw
                ),
            )
            .await;
        let _ = record_delivery(
            db,
            "user",
            &user_id,
            "password_setup",
            receipt.status.as_str(),
            receipt.reason,
        )
        .await;
    } else {
        model.update(&transaction).await?;
        transaction.commit().await?;
        let _ = delivery
            .send(
                &request.email,
                "安归访问申请结果",
                "你的访问申请未获批准，请联系管理员了解详情。",
            )
            .await;
    }
    Ok(AdminAccessRequestResponse {
        id: request.id,
        email: request.email,
        display_name: request.display_name,
        requested_role: request.requested_role,
        status: status.to_owned(),
        email_verified_at: request.email_verified_at,
        created_at: request.created_at,
    })
}

pub async fn set_password(
    db: &DatabaseConnection,
    input: PasswordSetupRequest,
) -> Result<(), ApiError> {
    if !(12..=256).contains(&input.password.chars().count()) {
        return Err(ApiError::Validation(
            "password must contain between 12 and 256 characters".to_owned(),
        ));
    }
    let token = auth_email_tokens::Entity::find()
        .filter(auth_email_tokens::Column::TokenHash.eq(hash_token(&input.token)))
        .filter(auth_email_tokens::Column::Purpose.eq("password_setup"))
        .filter(auth_email_tokens::Column::ConsumedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(|| {
            ApiError::Unauthorized("password setup link is invalid or expired".to_owned())
        })?;
    if token.expires_at <= now() {
        return Err(ApiError::Unauthorized(
            "password setup link is invalid or expired".to_owned(),
        ));
    }
    let user_id = token.user_id.clone().ok_or(ApiError::Internal)?;
    let password_hash = hash_password(input.password).await?;
    let transaction = db.begin().await?;
    let user = users::Entity::find_by_id(&user_id)
        .one(&transaction)
        .await?
        .ok_or(ApiError::Internal)?;
    let mut user_model = user.into_active_model();
    user_model.password_hash = Set(password_hash);
    user_model.status = Set("active".to_owned());
    user_model.updated_at = Set(now());
    user_model.update(&transaction).await?;
    let mut token_model = token.into_active_model();
    token_model.consumed_at = Set(Some(now()));
    token_model.update(&transaction).await?;
    transaction.commit().await?;
    Ok(())
}

fn role_mapping(role: &str) -> Result<(AccountType, Vec<GlobalCapability>), ApiError> {
    match role {
        "family" => Ok((AccountType::Member, vec![])),
        "volunteer" => Ok((AccountType::Member, vec![GlobalCapability::Volunteer])),
        "commander" => Ok((AccountType::Member, vec![GlobalCapability::Commander])),
        _ => Err(ApiError::Validation("role is unsupported".to_owned())),
    }
}
fn require_admin(auth: &AuthenticatedUser) -> Result<(), ApiError> {
    if auth.global_capabilities.contains(&GlobalCapability::Admin) {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "administrator capability is required".to_owned(),
        ))
    }
}
fn normalize_email(v: &str) -> Result<String, ApiError> {
    let v = v.trim().to_lowercase();
    if v.is_empty() || v.len() > 320 || !v.contains('@') {
        Err(ApiError::Validation("email is invalid".to_owned()))
    } else {
        Ok(v)
    }
}
fn validate_name(v: &str) -> Result<(), ApiError> {
    if v.trim().is_empty() || v.chars().count() > 120 {
        Err(ApiError::Validation("display_name is invalid".to_owned()))
    } else {
        Ok(())
    }
}
fn validate_role(v: &str) -> Result<(), ApiError> {
    if ["family", "volunteer", "commander"].contains(&v.trim().to_lowercase().as_str()) {
        Ok(())
    } else {
        Err(ApiError::Validation("role is unsupported".to_owned()))
    }
}
fn token() -> String {
    format!(
        "angui_mail_{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    )
}
fn hash_token(v: &str) -> String {
    hex::encode(Sha256::digest(v.as_bytes()))
}
fn empty_password_hash() -> String {
    "!pending-password".to_owned()
}
async fn hash_password(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || {
        use argon2::{
            Argon2, PasswordHasher,
            password_hash::{SaltString, rand_core::OsRng},
        };
        Argon2::default()
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map(|h| h.to_string())
            .map_err(|_| ApiError::Internal)
    })
    .await
    .map_err(|_| ApiError::Internal)?
}
async fn record_delivery(
    db: &DatabaseConnection,
    subject_type: &str,
    subject_id: &str,
    template: &str,
    status: &str,
    reason: Option<String>,
) -> Result<(), ApiError> {
    message_deliveries::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        channel: Set("email".to_owned()),
        template: Set(template.to_owned()),
        subject_type: Set(subject_type.to_owned()),
        subject_id: Set(subject_id.to_owned()),
        status: Set(status.to_owned()),
        attempt_count: Set(1),
        failure_reason: Set(reason),
        created_at: Set(now()),
        delivered_at: Set((status == "delivered").then(now)),
    }
    .insert(db)
    .await?;
    Ok(())
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_email_without_changing_the_local_part_contents() {
        assert_eq!(
            normalize_email("  Applicant+Tag@Example.INVALID ").unwrap(),
            "applicant+tag@example.invalid"
        );
    }

    #[test]
    fn rejects_empty_or_malformed_email_addresses() {
        assert!(normalize_email("").is_err());
        assert!(normalize_email("not-an-email").is_err());
    }

    #[test]
    fn requested_roles_are_a_closed_set() {
        for role in ["family", "volunteer", "commander"] {
            assert!(validate_role(role).is_ok(), "{role} should be valid");
        }
        assert!(validate_role("admin").is_err());
        assert!(validate_role("member").is_err());
    }

    #[test]
    fn only_supported_roles_map_to_server_capabilities() {
        let (family_type, family_capabilities) = role_mapping("family").unwrap();
        assert_eq!(family_type, AccountType::Member);
        assert!(family_capabilities.is_empty());

        let (_, volunteer_capabilities) = role_mapping("volunteer").unwrap();
        assert_eq!(volunteer_capabilities, vec![GlobalCapability::Volunteer]);
        assert!(role_mapping("admin").is_err());
    }

    #[test]
    fn raw_email_tokens_are_never_the_database_value() {
        let raw = token();
        let stored = hash_token(&raw);
        assert_ne!(stored, raw);
        assert_eq!(stored.len(), 64);
        assert_eq!(stored, hash_token(&raw));
    }
}
