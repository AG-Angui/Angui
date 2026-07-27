use actix_web::{HttpRequest, HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{AuthenticatedUser, LoginRequest, UserResponse},
    services::auth_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/auth")
            .route("/login", web::post().to(login))
            .route("/logout", web::post().to(logout))
            .route("/me", web::get().to(current_user)),
    );
}

async fn login(
    request_meta: HttpRequest,
    state: web::Data<AppState>,
    request: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    let request = request.into_inner();
    let client = request_meta
        .peer_addr()
        .map(|address| address.ip().to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    let client_key = format!("client:{client}");
    let account_key = format!("account:{client}:{}", request.email.trim().to_lowercase());
    state.login_limiter.check(&client_key)?;
    state.login_limiter.check(&account_key)?;

    match auth_service::login(&state.db, request, state.session_ttl_hours).await {
        Ok(response) => {
            state.login_limiter.clear(&client_key)?;
            state.login_limiter.clear(&account_key)?;
            Ok(HttpResponse::Ok().json(response))
        }
        Err(ApiError::Unauthorized(message)) => {
            state.login_limiter.record_failure(client_key)?;
            state.login_limiter.record_failure(account_key)?;
            Err(ApiError::Unauthorized(message))
        }
        Err(error) => Err(error),
    }
}

async fn logout(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    auth_service::logout(&state.db, &auth).await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn current_user(auth: AuthenticatedUser) -> HttpResponse {
    HttpResponse::Ok().json(UserResponse {
        id: auth.id,
        email: auth.email,
        display_name: auth.display_name,
        account_type: auth.account_type,
        global_capabilities: auth.global_capabilities,
    })
}
