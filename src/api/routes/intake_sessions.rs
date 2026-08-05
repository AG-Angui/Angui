use std::time::Duration;

use actix_web::{Error, HttpResponse, http::header, web};
use futures_util::stream;
use serde_json::json;
use tokio::sync::mpsc;

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AcknowledgeIntakeAiInitialReviewRequest, AuthenticatedUser, ConfirmIntakeSessionRequest,
        CreateIntakeSessionRequest, RestoreIntakeAnswerRequest, RestoreIntakeProfileDraftRequest,
        ReviewIntakeProfileDraftRequest, StartIntakeAiInitialReviewRequest,
        SubmitIntakeAnswerRequest,
    },
    services::ai_execution_service,
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
                "/{session_id}/profile-draft/generate",
                web::post().to(generate_intake_profile_draft),
            )
            .route(
                "/{session_id}/profile-draft/versions",
                web::get().to(list_intake_profile_draft_versions),
            )
            .route(
                "/{session_id}/profile-draft/{draft_id}/diff/{to_id}",
                web::get().to(diff_intake_profile_drafts),
            )
            .route(
                "/{session_id}/profile-draft/{draft_id}/review",
                web::patch().to(review_intake_profile_draft),
            )
            .route(
                "/{session_id}/profile-draft/restore",
                web::post().to(restore_intake_profile_draft),
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

async fn generate_intake_profile_draft(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    session_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let session_id = session_id.into_inner();
    let (execution, started) = ai_execution_service::start_intake_execution(
        &state.db,
        &auth,
        &session_id,
        "intake_profile_draft",
    )
    .await?;
    let execution_id = execution.execution_id.clone();
    let (sender, receiver) = mpsc::channel(8);
    let _ = sender.try_send(sse_event(
        "started",
        json!({
            "session_id": session_id.clone(),
            "execution_id": execution_id.clone(),
            "event_id": started.event_id,
            "workflow": execution.workflow,
        }),
    ));
    let _ = sender.try_send(sse_event(
        "progress",
        json!({
            "execution_id": execution_id.clone(),
            "event_id": started.event_id,
            "stage": "queued",
        }),
    ));

    tokio::spawn(async move {
        if let Ok(event) =
            ai_execution_service::advance_execution(&state.db, &auth, &execution_id, "preparing")
                .await
        {
            let _ = sender.send(sse_event("progress", json!(event))).await;
        }
        if let Ok(event) =
            ai_execution_service::advance_execution(&state.db, &auth, &execution_id, "generating")
                .await
        {
            let _ = sender.send(sse_event("progress", json!(event))).await;
        }
        let event = match intake_session_service::generate_intake_profile_draft(
            &state.db,
            &auth,
            &session_id,
            &state.ai_gateway,
        )
        .await
        {
            Ok(draft) => {
                let stage = if draft.degradation_status.contains("fallback") {
                    "fallback"
                } else {
                    "validating"
                };
                if let Ok(event) =
                    ai_execution_service::advance_execution(&state.db, &auth, &execution_id, stage)
                        .await
                {
                    let _ = sender.send(sse_event("progress", json!(event))).await;
                }
                let _ = ai_execution_service::complete_execution(
                    &state.db,
                    &auth,
                    &execution_id,
                    "draft_ready",
                    stage == "fallback",
                )
                .await;
                sse_event("completed", json!(draft))
            }
            Err(_) => {
                let _ = ai_execution_service::fail_execution(
                    &state.db,
                    &auth,
                    &execution_id,
                    "processing_failed",
                )
                .await;
                sse_event(
                    "error",
                    json!({ "message": "AI 审核未能完成，请重试或改用人工流程。" }),
                )
            }
        };
        let _ = sender.send(event).await;
    });

    Ok(intake_sse_response(receiver))
}

fn intake_sse_response(receiver: mpsc::Receiver<web::Bytes>) -> HttpResponse {
    let events = stream::unfold(
        (receiver, tokio::time::interval(Duration::from_secs(10))),
        |(mut receiver, mut heartbeat)| async move {
            tokio::select! {
                event = receiver.recv() => event.map(|event| {
                    (Ok::<_, Error>(event), (receiver, heartbeat))
                }),
                _ = heartbeat.tick() => Some((
                    Ok(web::Bytes::from_static(b": keep-alive\n\n")),
                    (receiver, heartbeat),
                )),
            }
        },
    );

    HttpResponse::Created()
        .insert_header((header::CONTENT_TYPE, "text/event-stream"))
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(events)
}

fn sse_event(event: &str, payload: serde_json::Value) -> web::Bytes {
    let payload = serde_json::to_string(&payload)
        .unwrap_or_else(|_| "{\"message\":\"internal service error\"}".to_owned());
    web::Bytes::from(format!("event: {event}\ndata: {payload}\n\n"))
}

async fn list_intake_profile_draft_versions(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    session_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        intake_session_service::list_intake_profile_draft_versions(&state.db, &auth, &session_id)
            .await?,
    ))
}
async fn diff_intake_profile_drafts(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (session_id, from_id, to_id) = path.into_inner();
    Ok(HttpResponse::Ok().json(
        intake_session_service::diff_intake_profile_drafts(
            &state.db,
            &auth,
            &session_id,
            &from_id,
            &to_id,
        )
        .await?,
    ))
}
async fn review_intake_profile_draft(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    request: web::Json<ReviewIntakeProfileDraftRequest>,
) -> Result<HttpResponse, ApiError> {
    let (session_id, draft_id) = path.into_inner();
    Ok(HttpResponse::Ok().json(
        intake_session_service::review_intake_profile_draft(
            &state.db,
            &auth,
            &session_id,
            &draft_id,
            request.into_inner(),
        )
        .await?,
    ))
}
async fn restore_intake_profile_draft(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    session_id: web::Path<String>,
    request: web::Json<RestoreIntakeProfileDraftRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created().json(
        intake_session_service::restore_intake_profile_draft(
            &state.db,
            &auth,
            &session_id,
            request.into_inner(),
        )
        .await?,
    ))
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
    let session_id = session_id.into_inner();
    let request = request.into_inner();
    let (execution, started) = ai_execution_service::start_intake_execution(
        &state.db,
        &auth,
        &session_id,
        "intake_initial_review",
    )
    .await?;
    let execution_id = execution.execution_id.clone();
    let (sender, receiver) = mpsc::channel(8);
    let _ = sender.try_send(sse_event(
        "started",
        json!({
            "session_id": session_id.clone(),
            "execution_id": execution_id.clone(),
            "event_id": started.event_id,
            "workflow": execution.workflow,
        }),
    ));
    let _ = sender.try_send(sse_event(
        "progress",
        json!({
            "execution_id": execution_id.clone(),
            "event_id": started.event_id,
            "stage": "queued",
        }),
    ));

    tokio::spawn(async move {
        if let Ok(event) =
            ai_execution_service::advance_execution(&state.db, &auth, &execution_id, "preparing")
                .await
        {
            let _ = sender.send(sse_event("progress", json!(event))).await;
        }
        if let Ok(event) =
            ai_execution_service::advance_execution(&state.db, &auth, &execution_id, "generating")
                .await
        {
            let _ = sender.send(sse_event("progress", json!(event))).await;
        }
        let event = match intake_session_service::start_ai_initial_review(
            &state.db,
            &auth,
            &session_id,
            request,
            &state.ai_gateway,
        )
        .await
        {
            Ok(review) => {
                let stage = if review.degradation_status == "rule_based_fallback" {
                    "fallback"
                } else {
                    "validating"
                };
                if let Ok(event) =
                    ai_execution_service::advance_execution(&state.db, &auth, &execution_id, stage)
                        .await
                {
                    let _ = sender.send(sse_event("progress", json!(event))).await;
                }
                let _ = ai_execution_service::complete_execution(
                    &state.db,
                    &auth,
                    &execution_id,
                    "review_ready",
                    stage == "fallback",
                )
                .await;
                sse_event("completed", json!(review))
            }
            Err(_) => {
                let _ = ai_execution_service::fail_execution(
                    &state.db,
                    &auth,
                    &execution_id,
                    "processing_failed",
                )
                .await;
                sse_event(
                    "error",
                    json!({ "message": "AI 审核未能完成，请重试或改用人工流程。" }),
                )
            }
        };
        let _ = sender.send(event).await;
    });

    Ok(intake_sse_response(receiver))
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
