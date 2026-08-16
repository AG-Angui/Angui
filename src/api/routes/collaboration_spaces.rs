use actix_multipart::Multipart;
use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AuthenticatedUser, CreateCollaborationSpaceRequest, CreateSpaceMessageRequest,
        JoinCollaborationSpaceRequest, RecordSpaceLocationRequest, SpaceEventsQuery,
    },
    services::collaboration_space_service,
};

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
                .route("/{space_id}/locations", web::post().to(record_location))
                .route(
                    "/{space_id}/members/{user_id}/track",
                    web::get().to(list_member_locations),
                )
                .route("/{space_id}/messages", web::get().to(list_messages))
                .route("/{space_id}/messages", web::post().to(create_message))
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
    Ok(HttpResponse::NoContent().finish())
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
            &state.attachment_storage_directory,
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
