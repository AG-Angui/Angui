use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AcknowledgeIntakeAiInitialReviewRequest, AuthenticatedUser, ConfirmIntakeSessionRequest,
        CreateIntakeSessionRequest, RestoreIntakeAnswerRequest, StartIntakeAiInitialReviewRequest,
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
                "/{session_id}/ai-follow-up",
                web::get().to(get_ai_follow_up),
            )
            .route(
                "/{session_id}/ai-initial-review",
                web::get().to(get_ai_initial_review),
            )
            .route(
                "/{session_id}/ai-initial-review",
                web::post().to(start_ai_initial_review),
            )
            .route(
                "/{session_id}/ai-initial-review/acknowledge",
                web::post().to(acknowledge_ai_initial_review),
            )
            .route(
                "/{session_id}/answer-revisions",
                web::get().to(list_answer_revisions),
            )
            .route(
                "/{session_id}/answers/{field}/restore",
                web::post().to(restore_answer_revision),
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

async fn get_ai_follow_up(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    session_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        intake_session_service::get_ai_follow_up(&state.db, &auth, &session_id, &state.ai_gateway)
            .await?,
    ))
}

async fn get_ai_initial_review(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    session_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(intake_session_service::get_ai_initial_review(&state.db, &auth, &session_id).await?))
}

async fn start_ai_initial_review(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    session_id: web::Path<String>,
    request: web::Json<StartIntakeAiInitialReviewRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created().json(
        intake_session_service::start_ai_initial_review(
            &state.db,
            &auth,
            &session_id,
            request.into_inner(),
            &state.ai_gateway,
        )
        .await?,
    ))
}

async fn acknowledge_ai_initial_review(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    session_id: web::Path<String>,
    request: web::Json<AcknowledgeIntakeAiInitialReviewRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        intake_session_service::acknowledge_ai_initial_review(
            &state.db,
            &auth,
            &session_id,
            request.into_inner(),
        )
        .await?,
    ))
}

async fn list_answer_revisions(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    session_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(intake_session_service::list_answer_revisions(&state.db, &auth, &session_id).await?))
}

async fn restore_answer_revision(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    request: web::Json<RestoreIntakeAnswerRequest>,
) -> Result<HttpResponse, ApiError> {
    let (session_id, field) = path.into_inner();
    Ok(HttpResponse::Created().json(
        intake_session_service::restore_answer_revision(
            &state.db,
            &auth,
            &session_id,
            &field,
            request.into_inner(),
            state.intake_answer_hard_max,
        )
        .await?,
    ))
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
    let response = intake_session_service::submit_intake_answer_with_map(
        &state.db,
        &auth,
        &session_id,
        request.into_inner(),
        state.intake_answer_hard_max,
        &state.amap_service,
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
