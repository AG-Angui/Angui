use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AuthenticatedUser, ConfirmIntakeSessionRequest, CreateIntakeSessionRequest,
        SubmitIntakeAnswerRequest,
    },
    services::intake_session_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/intake-sessions")
            .route("", web::post().to(create_intake_session))
            .route(
                "/{session_id}/answers",
                web::post().to(submit_intake_answer),
            )
            .route(
                "/{session_id}/profile-draft",
                web::get().to(get_intake_profile_draft),
            )
            .route(
                "/{session_id}/confirm",
                web::post().to(confirm_intake_session),
            ),
    );
}

async fn get_intake_profile_draft(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    session_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let draft =
        intake_session_service::get_intake_profile_draft(&state.db, &auth, &session_id).await?;
    Ok(HttpResponse::Ok().json(draft))
}

async fn confirm_intake_session(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    session_id: web::Path<String>,
    request: web::Json<ConfirmIntakeSessionRequest>,
) -> Result<HttpResponse, ApiError> {
    let confirmation = intake_session_service::confirm_intake_session(
        &state.db,
        &auth,
        &session_id,
        request.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Created().json(confirmation))
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
