use actix_web::{HttpRequest, HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AuthenticatedUser, CreateAccessRequest, LoginRequest, PasswordSetupRequest, UserResponse,
        VerifyAccessRequest,
    },
    services::access_request_service,
    services::auth_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/auth")
            .route("/login", web::post().to(login))
            .route("/logout", web::post().to(logout))
            .route("/me", web::get().to(current_user))
            .route("/access-requests", web::post().to(create_access_request))
            .route(
                "/access-requests/verify",
                web::post().to(verify_access_request),
            )
            .route("/password-setup", web::post().to(password_setup)),
    );
}

async fn create_access_request(
    state: web::Data<AppState>,
    request: web::Json<CreateAccessRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        access_request_service::create(
            &state.db,
            &state.message_delivery,
            request.into_inner(),
            &state.frontend_origin,
        )
        .await?,
    ))
}
async fn verify_access_request(
    state: web::Data<AppState>,
    request: web::Json<VerifyAccessRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(access_request_service::verify(&state.db, &request.token).await?))
}
async fn password_setup(
    state: web::Data<AppState>,
    request: web::Json<PasswordSetupRequest>,
) -> Result<HttpResponse, ApiError> {
    access_request_service::set_password(&state.db, request.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
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
