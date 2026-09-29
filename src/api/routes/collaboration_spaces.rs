use actix_multipart::Multipart;
use actix_web::{HttpRequest, HttpResponse, web};
use actix_ws::Message;
use futures_util::StreamExt;
use tokio::time::{Duration, interval};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AcknowledgeSpaceMessageRequest, AuthenticatedUser, CreateCollaborationSpaceRequest,
        CreateSpaceMessageRequest, JoinCollaborationSpaceRequest, RecordSpaceLocationRequest,
        SpaceEventsQuery,
    },
    services::{collaboration_space_service, voice_review_service, voice_room_service},
};

#[derive(serde::Deserialize)]
struct RecordingTimes {
    started_at: String,
    ended_at: String,
}
#[derive(serde::Deserialize)]
struct FloorRequest {
    active: bool,
}
#[derive(serde::Deserialize)]
struct DiscovererRequest {
    discoverer_user_id: String,
}
#[derive(serde::Deserialize)]
struct ReasonRequest {
    reason: String,
}
#[derive(serde::Deserialize)]
struct MergeRequest {
    target_clue_id: String,
    reason: String,
}

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(
            web::scope("/cases/{case_id}/collaboration-spaces")
                .route("", web::get().to(list_case_spaces))
                .route("", web::post().to(create_space)),
        )
        .service(
            web::scope("/collaboration-spaces")
                .route("/{space_id}/snapshot", web::get().to(get_snapshot))
                .route("/{space_id}/events", web::get().to(list_events))
                .route("/{space_id}/events/ws", web::get().to(events_websocket))
                .route("/{space_id}/events/ticket", web::post().to(events_ticket))
                .route(
                    "/{space_id}/voice-room/ticket",
                    web::post().to(voice_room_ticket),
                )
                .route(
                    "/{space_id}/voice-room/floor",
                    web::post().to(set_voice_floor),
                )
                .route("/{space_id}/recordings", web::post().to(create_recording))
                .route(
                    "/{space_id}/recordings/{id}/audio",
                    web::get().to(read_recording),
                )
                .route(
                    "/{space_id}/voice-reports/{id}/audio",
                    web::get().to(read_voice_report),
                )
                .route(
                    "/{space_id}/voice-candidates",
                    web::get().to(list_voice_candidates),
                )
                .route(
                    "/{space_id}/voice-candidates/{id}/retry",
                    web::post().to(retry_voice_candidate),
                )
                .route(
                    "/{space_id}/voice-candidates/{id}/discoverer",
                    web::post().to(confirm_discoverer),
                )
                .route(
                    "/{space_id}/voice-candidates/{id}/return",
                    web::post().to(return_voice_candidate),
                )
                .route(
                    "/{space_id}/voice-candidates/{id}/resubmit",
                    web::post().to(resubmit_voice_candidate),
                )
                .route(
                    "/{space_id}/voice-candidates/{id}/merge",
                    web::post().to(merge_voice_candidate),
                )
                .route("/{space_id}/locations", web::post().to(record_location))
                .route(
                    "/{space_id}/locations/latest",
                    web::get().to(list_latest_locations),
                )
                .route(
                    "/{space_id}/members/{user_id}/track",
                    web::get().to(list_member_locations),
                )
                .route("/{space_id}/messages", web::get().to(list_messages))
                .route("/{space_id}/messages", web::post().to(create_message))
                .route(
                    "/{space_id}/messages/{message_id}/ack",
                    web::post().to(acknowledge_message),
                )
                .route(
                    "/{space_id}/voice-reports",
                    web::get().to(list_voice_reports),
                )
                .route(
                    "/{space_id}/voice-reports",
                    web::post().to(create_voice_report),
                )
                .route("/{space_id}/join", web::post().to(join_space))
                .route("/{space_id}/leave", web::post().to(leave_space))
                .route("/{space_id}/archive", web::post().to(archive_space))
                .route(
                    "/{space_id}/location-consents",
                    web::post().to(grant_location_consent),
                )
                .route(
                    "/{space_id}/location-consents/me",
                    web::delete().to(revoke_location_consent),
                ),
        );
}

async fn events_websocket(
    request: HttpRequest,
    body: web::Payload,
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    collaboration_space_service::authorize_space(&state.db, &auth, &space_id)
        .await
        .map_err(actix_web::error::ErrorForbidden)?;
    let (response, mut session, mut messages) = actix_ws::handle(&request, body)?;
    let db = state.db.clone();
    let user = auth.clone();
    let id = space_id.into_inner();
    let initial_version = request
        .query_string()
        .split('&')
        .find_map(|part| part.strip_prefix("after_version="))
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0)
        .max(0);
    actix_web::rt::spawn(async move {
        let mut version = initial_version;
        let mut tick = interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                Some(Ok(message)) = messages.next() => match message {
                    Message::Text(text) if text == "ping" => {
                        let _ = session.text("pong").await;
                    }
                    Message::Text(_) => {}
                    Message::Ping(bytes) => {
                        let _ = session.pong(&bytes).await;
                    }
                    Message::Close(reason) => { let _ = session.close(reason).await; break; }
                    _ => {}
                },
                _ = tick.tick() => {
                    match collaboration_space_service::list_events(&db, &user, &id, version).await {
                        Ok(events) if !events.is_empty() => {
                            version = events.iter().map(|event| event.version).max().unwrap_or(version);
                            if session.text(serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_owned())).await.is_err() { break; }
                        }
                        Ok(_) => {}
                        Err(_) => { let _ = session.close(None).await; break; }
                    }
                }
                else => break,
            }
        }
    });
    Ok(response)
}

async fn events_ticket(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    collaboration_space_service::authorize_space(&state.db, &auth, &space_id).await?;
    let ticket = uuid::Uuid::new_v4().to_string();
    let mut tickets = state.ws_tickets.lock().map_err(|_| ApiError::Internal)?;
    tickets.retain(|_, item| item.created.elapsed().as_secs() < 30);
    if tickets.len() >= 10_000 {
        return Err(ApiError::Conflict(
            "too many active socket tickets".to_owned(),
        ));
    }
    tickets.insert(
        ticket.clone(),
        crate::app_state::SocketTicket {
            auth,
            space_id: space_id.into_inner(),
            created: std::time::Instant::now(),
        },
    );
    Ok(HttpResponse::Ok().json(serde_json::json!({"ticket":ticket,"expires_in_seconds":30})))
}

async fn create_space(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<CreateCollaborationSpaceRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created().json(
        collaboration_space_service::create_space(&state.db, &auth, &case_id, request.into_inner())
            .await?,
    ))
}

async fn list_case_spaces(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(collaboration_space_service::list_case_spaces(&state.db, &auth, &case_id).await?))
}

async fn get_snapshot(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(collaboration_space_service::get_snapshot(&state.db, &auth, &space_id).await?))
}

async fn join_space(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
    request: web::Json<JoinCollaborationSpaceRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        collaboration_space_service::join_space(&state.db, &auth, &space_id, request.into_inner())
            .await?,
    ))
}

async fn leave_space(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    collaboration_space_service::leave_space(&state.db, &auth, &space_id).await?;
    voice_room_service::disconnect_user(&state, &space_id, &auth.id).await;
    Ok(HttpResponse::NoContent().finish())
}

async fn archive_space(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let archived = collaboration_space_service::archive_space(&state.db, &auth, &space_id).await?;
    voice_room_service::close_room(&state, &space_id).await;
    Ok(HttpResponse::Ok().json(archived))
}

async fn grant_location_consent(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
    request: web::Json<JoinCollaborationSpaceRequest>,
) -> Result<HttpResponse, ApiError> {
    if !request.location_consent {
        return Err(ApiError::Validation(
            "location_consent must be true when granting consent".to_owned(),
        ));
    }
    collaboration_space_service::grant_location_consent(
        &state.db,
        &auth,
        &space_id,
        request.consent_version.clone().unwrap_or_default(),
    )
    .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn revoke_location_consent(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    collaboration_space_service::revoke_location_consent(&state.db, &auth, &space_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn list_events(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
    query: web::Query<SpaceEventsQuery>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        collaboration_space_service::list_events(
            &state.db,
            &auth,
            &space_id,
            query.after_version.unwrap_or(0),
        )
        .await?,
    ))
}

async fn record_location(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
    request: web::Json<RecordSpaceLocationRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created().json(
        collaboration_space_service::record_location(
            &state.db,
            &auth,
            &space_id,
            request.into_inner(),
        )
        .await?,
    ))
}

async fn list_member_locations(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (space_id, user_id) = path.into_inner();
    Ok(HttpResponse::Ok().json(
        collaboration_space_service::list_member_locations(&state.db, &auth, &space_id, &user_id)
            .await?,
    ))
}

async fn list_latest_locations(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        collaboration_space_service::list_latest_locations(&state.db, &auth, &space_id).await?,
    ))
}

async fn create_message(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
    request: web::Json<CreateSpaceMessageRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created().json(
        collaboration_space_service::create_message(
            &state.db,
            &auth,
            &space_id,
            request.into_inner(),
        )
        .await?,
    ))
}

async fn list_messages(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(collaboration_space_service::list_messages(&state.db, &auth, &space_id).await?))
}

async fn acknowledge_message(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    request: web::Json<AcknowledgeSpaceMessageRequest>,
) -> Result<HttpResponse, ApiError> {
    let (space_id, message_id) = path.into_inner();
    collaboration_space_service::acknowledge_message(
        &state.db,
        &auth,
        &space_id,
        &message_id,
        &request.status,
    )
    .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn create_voice_report(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
    multipart: Multipart,
) -> Result<HttpResponse, ApiError> {
    let (filename, content_type, bytes) =
        crate::services::case_resource_service::read_single_audio_upload(
            multipart,
            collaboration_space_service::MAX_VOICE_REPORT_BYTES,
        )
        .await?;
    Ok(HttpResponse::Created().json(
        collaboration_space_service::store_voice_report(
            &state.db,
            &auth,
            &space_id,
            &filename,
            &content_type,
            &bytes,
            state.audio_storage.as_ref(),
        )
        .await?,
    ))
}

async fn list_voice_reports(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(collaboration_space_service::list_voice_reports(&state.db, &auth, &space_id).await?))
}

async fn voice_room_ticket(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(voice_room_service::room_ticket(&state, &auth, &space_id).await?))
}

async fn set_voice_floor(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
    body: web::Json<FloorRequest>,
) -> Result<HttpResponse, ApiError> {
    voice_room_service::set_floor(&state, &auth, &space_id, body.active).await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn create_recording(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
    times: web::Query<RecordingTimes>,
    multipart: Multipart,
) -> Result<HttpResponse, ApiError> {
    let (_filename, mime, bytes) =
        crate::services::case_resource_service::read_single_audio_upload(
            multipart,
            collaboration_space_service::MAX_VOICE_REPORT_BYTES,
        )
        .await?;
    if mime != "audio/wav" {
        return Err(ApiError::Validation(
            "intercom recording must be audio/wav".to_owned(),
        ));
    }
    Ok(HttpResponse::Created().json(
        collaboration_space_service::store_intercom_recording(
            &state.db,
            &auth,
            &space_id,
            &bytes,
            &times.started_at,
            &times.ended_at,
            state.audio_storage.as_ref(),
        )
        .await?,
    ))
}

async fn read_recording(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (space_id, id) = path.into_inner();
    let (bytes, mime) = collaboration_space_service::read_audio(
        &state.db,
        &auth,
        &space_id,
        "recording",
        &id,
        state.audio_storage.as_ref(),
    )
    .await?;
    Ok(HttpResponse::Ok().content_type(mime).body(bytes))
}

async fn read_voice_report(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (space_id, id) = path.into_inner();
    let (bytes, mime) = collaboration_space_service::read_audio(
        &state.db,
        &auth,
        &space_id,
        "report",
        &id,
        state.audio_storage.as_ref(),
    )
    .await?;
    Ok(HttpResponse::Ok().content_type(mime).body(bytes))
}

async fn list_voice_candidates(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    space_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        collaboration_space_service::list_voice_candidates(&state.db, &auth, &space_id).await?,
    ))
}

async fn retry_voice_candidate(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (space_id, id) = path.into_inner();
    collaboration_space_service::retry_voice_candidate(&state.db, &auth, &space_id, &id).await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn confirm_discoverer(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<DiscovererRequest>,
) -> Result<HttpResponse, ApiError> {
    let (space_id, id) = path.into_inner();
    collaboration_space_service::confirm_voice_discoverer(
        &state.db,
        &auth,
        &space_id,
        &id,
        &body.discoverer_user_id,
    )
    .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn return_voice_candidate(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<ReasonRequest>,
) -> Result<HttpResponse, ApiError> {
    let (space_id, id) = path.into_inner();
    voice_review_service::return_candidate(&state.db, &auth, &space_id, &id, &body.reason).await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn resubmit_voice_candidate(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<crate::models::ClueDraftCandidate>,
) -> Result<HttpResponse, ApiError> {
    let (space_id, id) = path.into_inner();
    voice_review_service::resubmit_candidate(&state.db, &auth, &space_id, &id, body.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn merge_voice_candidate(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    body: web::Json<MergeRequest>,
) -> Result<HttpResponse, ApiError> {
    let (space_id, id) = path.into_inner();
    voice_review_service::merge_candidate(
        &state.db,
        &auth,
        &space_id,
        &id,
        &body.target_clue_id,
        &body.reason,
    )
    .await?;
    Ok(HttpResponse::NoContent().finish())
}
