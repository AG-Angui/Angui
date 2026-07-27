use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{AuthenticatedUser, UpdateUserProfileRequest},
    services::auth_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/users/me")
            .route("/profile", web::get().to(get_profile))
            .route("/profile", web::patch().to(update_profile)),
    );
}

async fn get_profile(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(auth_service::get_profile(&state.db, &auth).await?))
}

async fn update_profile(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
    request: web::Json<UpdateUserProfileRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(auth_service::update_profile(&state.db, &auth, request.into_inner()).await?))
}
