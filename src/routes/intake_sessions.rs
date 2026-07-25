use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{AuthenticatedUser, CreateIntakeSessionRequest, SubmitIntakeAnswerRequest},
    services::intake_session_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/intake-sessions")
            .route("", web::post().to(create_intake_session))
            .route(
                "/{session_id}/answers",
                web::post().to(submit_intake_answer),
            ),
    );
}

async fn submit_intake_answer(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    session_id: web::Path<String>,
    request: web::Json<SubmitIntakeAnswerRequest>,
) -> Result<HttpResponse, ApiError> {
    let response = intake_session_service::submit_intake_answer(
        &state.db,
        &auth,
        &session_id,
        request.into_inner(),
        state.intake_answer_hard_max,
    )
    .await?;
    Ok(HttpResponse::Created().json(response))
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
