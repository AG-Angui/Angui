use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::{Duration, SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    entities::{audit_events, auth_sessions, user_global_capabilities, users},
    error::ApiError,
    models::{AuthenticatedUser, LoginRequest, LoginResponse, UserResponse},
    roles::{AccountType, GlobalCapability},
};

pub async fn login(
    db: &DatabaseConnection,
    request: LoginRequest,
    session_ttl_hours: i64,
) -> Result<LoginResponse, ApiError> {
    let email = normalize_email(&request.email)?;
    if request.password.is_empty() || request.password.len() > 256 {
        return Err(ApiError::Unauthorized(
            "invalid email or password".to_owned(),
        ));
    }
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(db)
        .await?;

    let password = request.password;
    let password_valid = match user.as_ref() {
        Some(user) => verify_password(password, user.password_hash.clone()).await?,
        None => {
            hash_password(password).await?;
            false
        }
    };

    let Some(user) = user.filter(|user| user.status == "active" && password_valid) else {
        write_auth_audit(db, None, "anonymous", "auth.login_failed", "unknown").await?;
        return Err(ApiError::Unauthorized(
            "invalid email or password".to_owned(),
        ));
    };
    let account_type = account_type_from_database(&user.account_type)?;
    let global_capabilities = global_capabilities_for_user(db, &user.id).await?;

    let transaction = db.begin().await?;
    let raw_token = format!(
        "angui_{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let session_id = Uuid::new_v4().to_string();
    let created_at = now();
    let expires_at = (Utc::now() + Duration::hours(session_ttl_hours))
        .to_rfc3339_opts(SecondsFormat::Millis, true);

    auth_sessions::ActiveModel {
        id: Set(session_id.clone()),
        user_id: Set(user.id.clone()),
        token_hash: Set(hash_token(&raw_token)),
        expires_at: Set(expires_at.clone()),
        revoked_at: Set(None),
        created_at: Set(created_at.clone()),
        last_used_at: Set(created_at),
    }
    .insert(&transaction)
    .await?;

    write_auth_audit(
        &transaction,
        None,
        &user.id,
        "auth.login_succeeded",
        &session_id,
    )
    .await?;
    transaction.commit().await?;

    Ok(LoginResponse {
        token: raw_token,
        expires_at,
        user: UserResponse {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            account_type,
            global_capabilities,
        },
    })
}

pub async fn authenticate(
    db: &DatabaseConnection,
    raw_token: &str,
) -> Result<AuthenticatedUser, ApiError> {
    let session = auth_sessions::Entity::find()
        .filter(auth_sessions::Column::TokenHash.eq(hash_token(raw_token)))
        .filter(auth_sessions::Column::RevokedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("invalid or expired session".to_owned()))?;

    if session.expires_at <= now() {
        return Err(ApiError::Unauthorized(
            "invalid or expired session".to_owned(),
        ));
    }

    let user = users::Entity::find_by_id(&session.user_id)
        .one(db)
        .await?
        .filter(|user| user.status == "active")
        .ok_or_else(|| ApiError::Unauthorized("invalid or expired session".to_owned()))?;

    let session_id = session.id.clone();
    let mut active = session.into_active_model();
    active.last_used_at = Set(now());
    active.update(db).await?;
    let account_type = account_type_from_database(&user.account_type)?;
    let global_capabilities = global_capabilities_for_user(db, &user.id).await?;

    Ok(AuthenticatedUser {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        account_type,
        global_capabilities,
        session_id,
    })
}

pub async fn logout(db: &DatabaseConnection, auth: &AuthenticatedUser) -> Result<(), ApiError> {
    let session = auth_sessions::Entity::find_by_id(&auth.session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("invalid or expired session".to_owned()))?;
    let transaction = db.begin().await?;
    let mut active = session.into_active_model();
    active.revoked_at = Set(Some(now()));
    active.update(&transaction).await?;
    write_auth_audit(
        &transaction,
        None,
        &auth.id,
        "auth.logout",
        &auth.session_id,
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn bootstrap_demo_users(
    db: &DatabaseConnection,
    password: &str,
) -> Result<Vec<UserResponse>, ApiError> {
    if !(12..=256).contains(&password.chars().count()) {
        return Err(ApiError::Validation(
            "ANGUI_DEMO_PASSWORD must contain between 12 and 256 characters".to_owned(),
        ));
    }

    let definitions = [
        (
            "family@demo.invalid",
            "模拟家属",
            AccountType::Member,
            &[][..],
        ),
        (
            "commander@demo.invalid",
            "模拟指挥",
            AccountType::Member,
            &[GlobalCapability::Commander][..],
        ),
        (
            "volunteer@demo.invalid",
            "模拟志愿者",
            AccountType::Member,
            &[GlobalCapability::Volunteer][..],
        ),
        (
            "learner@demo.invalid",
            "模拟新人",
            AccountType::Learner,
            &[][..],
        ),
        (
            "admin@demo.invalid",
            "模拟管理员",
            AccountType::Member,
            &[GlobalCapability::Admin][..],
        ),
    ];
    let mut created = Vec::with_capacity(definitions.len());
    for (email, display_name, account_type, global_capabilities) in definitions {
        created.push(
            upsert_user(
                db,
                email,
                display_name,
                account_type,
                global_capabilities,
                password,
            )
            .await?,
        );
    }
    Ok(created)
}

async fn upsert_user(
    db: &DatabaseConnection,
    email: &str,
    display_name: &str,
    account_type: AccountType,
    global_capabilities: &[GlobalCapability],
    password: &str,
) -> Result<UserResponse, ApiError> {
    let email = normalize_email(email)?;
    let password_hash = hash_password(password.to_owned()).await?;
    let timestamp = now();

    let transaction = db.begin().await?;
    let user = if let Some(existing) = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(&transaction)
        .await?
    {
        let mut active = existing.into_active_model();
        active.display_name = Set(display_name.to_owned());
        active.account_type = Set(account_type.to_string());
        active.password_hash = Set(password_hash);
        active.status = Set("active".to_owned());
        active.updated_at = Set(timestamp.clone());
        active.update(&transaction).await?
    } else {
        users::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            email: Set(email),
            display_name: Set(display_name.to_owned()),
            account_type: Set(account_type.to_string()),
            password_hash: Set(password_hash),
            status: Set("active".to_owned()),
            created_at: Set(timestamp.clone()),
            updated_at: Set(timestamp.clone()),
        }
        .insert(&transaction)
        .await?
    };

    user_global_capabilities::Entity::delete_many()
        .filter(user_global_capabilities::Column::UserId.eq(&user.id))
        .exec(&transaction)
        .await?;
    for capability in global_capabilities {
        user_global_capabilities::ActiveModel {
            user_id: Set(user.id.clone()),
            capability: Set(capability.to_string()),
            created_at: Set(timestamp.clone()),
        }
        .insert(&transaction)
        .await?;
    }

    let sessions = auth_sessions::Entity::find()
        .filter(auth_sessions::Column::UserId.eq(&user.id))
        .filter(auth_sessions::Column::RevokedAt.is_null())
        .all(&transaction)
        .await?;
    for session in sessions {
        let mut active = session.into_active_model();
        active.revoked_at = Set(Some(now()));
        active.update(&transaction).await?;
    }
    transaction.commit().await?;

    Ok(UserResponse {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        account_type,
        global_capabilities: global_capabilities.to_vec(),
    })
}

fn account_type_from_database(value: &str) -> Result<AccountType, ApiError> {
    AccountType::try_from(value).map_err(|error| {
        ApiError::Database(sea_orm::DbErr::Custom(format!(
            "users.account_type violates the account type constraint: {error}"
        )))
    })
}

async fn global_capabilities_for_user<C: sea_orm::ConnectionTrait>(
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

async fn hash_password(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| ApiError::Internal)
    })
    .await
    .map_err(|_| ApiError::Internal)?
}

async fn verify_password(password: String, encoded_hash: String) -> Result<bool, ApiError> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&encoded_hash).map_err(|_| ApiError::Internal)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .map_err(|_| ApiError::Internal)?
}

fn hash_token(raw_token: &str) -> String {
    hex::encode(Sha256::digest(raw_token.as_bytes()))
}

fn normalize_email(email: &str) -> Result<String, ApiError> {
    let email = email.trim().to_lowercase();
    if email.is_empty() || email.len() > 320 || !email.contains('@') {
        return Err(ApiError::Validation("email is invalid".to_owned()));
    }
    Ok(email)
}

async fn write_auth_audit<C: sea_orm::ConnectionTrait>(
    db: &C,
    case_id: Option<String>,
    actor: &str,
    action: &str,
    entity_id: &str,
) -> Result<(), ApiError> {
    audit_events::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        case_id: Set(case_id),
        actor: Set(actor.to_owned()),
        action: Set(action.to_owned()),
        entity_type: Set("auth_session".to_owned()),
        entity_id: Set(entity_id.to_owned()),
        metadata_json: Set(Some(json!({ "result": action }).to_string())),
        created_at: Set(now()),
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
    use migration::{Migrator, MigratorTrait};
    use sea_orm::Database;

    use super::{authenticate, bootstrap_demo_users, login, logout};
    use crate::{error::ApiError, models::LoginRequest};

    #[actix_web::test]
    async fn logout_revokes_the_server_side_session() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should connect");
        Migrator::up(&database, None)
            .await
            .expect("migrations should run");
        bootstrap_demo_users(&database, "demo-password-123")
            .await
            .expect("users should bootstrap");
        let login = login(
            &database,
            LoginRequest {
                email: "family@demo.invalid".to_owned(),
                password: "demo-password-123".to_owned(),
            },
            8,
        )
        .await
        .expect("login should succeed");
        let auth = authenticate(&database, &login.token)
            .await
            .expect("session should authenticate");
        logout(&database, &auth)
            .await
            .expect("logout should succeed");

        assert!(matches!(
            authenticate(&database, &login.token).await,
            Err(ApiError::Unauthorized(_))
        ));
    }
}
