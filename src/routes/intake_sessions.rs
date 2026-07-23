use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{AuthenticatedUser, CreateIntakeSessionRequest},
    services::intake_session_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(web::scope("/intake-sessions").route("", web::post().to(create_intake_session)));
}

async fn create_intake_session(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    request: web::Json<CreateIntakeSessionRequest>,
) -> Result<HttpResponse, ApiError> {
    let session = intake_session_service::create_intake_session(
        &state.db,
        &auth,
        request.into_inner(),
        state.intake_answer_hard_max,
    )
    .await?;
    Ok(HttpResponse::Created().json(session))
}
