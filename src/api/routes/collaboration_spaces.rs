use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AuthenticatedUser, CreateCollaborationSpaceRequest, JoinCollaborationSpaceRequest,
        SpaceEventsQuery,
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
