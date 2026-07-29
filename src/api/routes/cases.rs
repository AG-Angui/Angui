use actix_web::{HttpResponse, http::header, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AddCaseMemberRequest, AuthenticatedUser, CaseMapItem, CaseMapViewResponse, CasePoiQuery,
        CaseResourceConfigurationResponse, CreateCasePlaceRequest, CreateCaseRequest,
        CreateClueDraftRequest, CreateClueRequest, CreateSummaryDraftRequest, CreateTaskRequest,
        ReviewSummaryDraftRequest, TaskListQuery, UpdateCaseStatusRequest,
        UpdateElderProfileRequest,
    },
    roles::CaseRole,
    services::case_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/cases")
            .route("", web::get().to(list_cases))
            .route("", web::post().to(create_case))
            .route("/command-intake", web::get().to(list_command_intake))
            .route("/{case_id}/accept-command", web::post().to(accept_command))
            .route("/{case_id}", web::get().to(get_case))
            .route("/{case_id}/status", web::patch().to(update_case_status))
            .route(
                "/{case_id}/elder-profile",
                web::patch().to(update_elder_profile),
            )
            .route("/{case_id}/clues", web::get().to(list_clues))
            .route("/{case_id}/clues", web::post().to(create_clue))
            .route("/{case_id}/tasks", web::get().to(list_tasks))
            .route("/{case_id}/tasks", web::post().to(create_task))
            .route("/{case_id}/map-view", web::get().to(get_map_view))
            .route("/{case_id}/summary", web::get().to(get_case_summary))
            .route(
                "/{case_id}/public-progress",
                web::get().to(get_public_progress),
            )
            .route("/{case_id}/clue-drafts", web::post().to(create_clue_drafts))
            .route("/{case_id}/pois", web::get().to(list_case_pois))
            .route(
                "/{case_id}/summary-drafts",
                web::post().to(create_summary_draft),
            )
            .route(
                "/{case_id}/summary-drafts/{draft_id}/review",
                web::patch().to(review_summary_draft),
            )
            .route(
                "/{case_id}/archive-drafts",
                web::post().to(create_archive_draft),
            )
            .route("/{case_id}/places", web::get().to(list_places))
            .route("/{case_id}/places", web::post().to(create_place))
            .route(
                "/{case_id}/resource-configuration",
                web::get().to(get_resource_configuration),
            )
            .route("/{case_id}/attachments", web::post().to(create_attachment))
            .route(
                "/{case_id}/attachments/{attachment_id}",
                web::get().to(download_attachment),
            )
            .route("/{case_id}/members", web::get().to(list_case_members))
            .route("/{case_id}/members", web::post().to(add_case_member)),
    );
}

async fn list_command_intake(auth: AuthenticatedUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(case_service::list_command_intake(&state.db, &auth).await?))
}

async fn accept_command(auth: AuthenticatedUser, state: web::Data<AppState>, case_id: web::Path<String>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(case_service::accept_command_case(&state.db, &auth, &case_id).await?))
}

async fn list_case_members(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(case_service::list_case_members(&state.db, &auth, &case_id).await?))
}

async fn get_resource_configuration(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    case_service::require_case_role(
        &state.db,
        &auth.id,
        &case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    Ok(HttpResponse::Ok().json(CaseResourceConfigurationResponse {
        attachment_max_image_bytes: state.attachment_max_image_bytes,
        attachment_max_per_case: state.attachment_max_per_case,
        case_place_types: state.case_place_types.clone(),
    }))
}

async fn create_place(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<CreateCasePlaceRequest>,
) -> Result<HttpResponse, ApiError> {
    let place = crate::services::case_resource_service::create_place(
        &state.db,
        &auth,
        &case_id,
        request.into_inner(),
        &state.case_place_types,
    )
    .await?;
    Ok(HttpResponse::Created().json(place))
}

async fn list_places(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let role = case_service::require_case_role(
        &state.db,
        &auth.id,
        &case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let places =
        crate::services::case_resource_service::visible_places(&state.db, &case_id, &auth.id, role)
            .await?;
    Ok(HttpResponse::Ok().json(places))
}

async fn create_attachment(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    multipart: actix_multipart::Multipart,
) -> Result<HttpResponse, ApiError> {
    let (filename, content_type, bytes) =
        crate::services::case_resource_service::read_single_image_upload(
            multipart,
            state.attachment_max_image_bytes,
        )
        .await?;
    let attachment = crate::services::case_resource_service::store_image_attachment(
        &state.db,
        &auth,
        &case_id,
        crate::services::case_resource_service::AttachmentUpload {
            filename: &filename,
            declared_content_type: &content_type,
            bytes: &bytes,
        },
        crate::services::case_resource_service::AttachmentStorage {
            directory: &state.attachment_storage_directory,
            max_image_bytes: state.attachment_max_image_bytes,
            max_attachments_per_case: state.attachment_max_per_case,
        },
    )
    .await?;
    Ok(HttpResponse::Created().json(attachment))
}

async fn create_archive_draft(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let draft = crate::services::case_collaboration_service::create_archive_draft(
        &state.db, &auth, &case_id,
    )
    .await?;
    Ok(HttpResponse::Created().json(draft))
}

async fn download_attachment(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (case_id, attachment_id) = path.into_inner();
    let attachment = crate::services::case_resource_service::load_attachment_for_download(
        &state.db,
        &auth,
        &case_id,
        &attachment_id,
        &state.attachment_storage_directory,
    )
    .await?;
    Ok(HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, attachment.content_type))
        .insert_header((header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .insert_header((header::CACHE_CONTROL, "no-store, private"))
        .insert_header((
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", attachment.filename),
        ))
        .body(attachment.bytes))
}

async fn list_cases(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let cases = case_service::list_cases(&state.db, &auth).await?;
    Ok(HttpResponse::Ok().json(cases))
}

async fn create_case(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    request: web::Json<CreateCaseRequest>,
) -> Result<HttpResponse, ApiError> {
    let case = case_service::create_case(&state.db, &auth, request.into_inner()).await?;
    Ok(HttpResponse::Created().json(case))
}

async fn get_case(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
    case_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let case = case_service::get_case(&state.db, &auth, &case_id).await?;
    Ok(HttpResponse::Ok().json(case))
}

async fn get_map_view(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
    case_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let detail = case_service::get_case(&state.db, &auth, &case_id).await?;
    let task_items =
        crate::services::task_service::list_all_visible_tasks(&state.db, &auth, &case_id).await?;
    let mut items = Vec::new();
    if detail.access_role != CaseRole::Volunteer
        && let Some(location_text) = detail.elder_profile.last_seen_location
    {
        items.push(CaseMapItem {
            id: format!("case:{}:last-seen", detail.id),
            object_type: "last_seen".to_owned(),
            display_name: None,
            longitude: None,
            latitude: None,
            location_text: Some(location_text),
            location_precision: "unknown".to_owned(),
            source: "case_profile".to_owned(),
            occurred_at: detail.elder_profile.last_seen_at,
            reported_at: None,
            review_status: "pending_review".to_owned(),
            related_task_id: None,
            updated_at: detail.updated_at.clone(),
        });
    }
    items.extend(detail.places.into_iter().map(|place| {
        CaseMapItem {
            id: place.id,
            object_type: "place".to_owned(),
            display_name: Some(place.name),
            longitude: place.longitude,
            latitude: place.latitude,
            location_text: Some(place.address),
            location_precision: if place.longitude.is_some() {
                "exact"
            } else {
                "unknown"
            }
            .to_owned(),
            source: place.source,
            occurred_at: None,
            reported_at: Some(place.created_at),
            review_status: place.review_status,
            related_task_id: None,
            updated_at: place.updated_at,
        }
    }));
    if detail.access_role == CaseRole::Commander {
        items.extend(
            detail
                .clues
                .into_iter()
                .filter(|clue| clue.status == "confirmed" && clue.location_text.is_some())
                .map(|clue| CaseMapItem {
                    id: clue.id,
                    object_type: "clue".to_owned(),
                    display_name: Some(clue.content),
                    longitude: None,
                    latitude: None,
                    location_text: clue.location_text,
                    location_precision: clue
                        .location_precision
                        .unwrap_or_else(|| "unknown".to_owned()),
                    source: clue.source_type,
                    occurred_at: clue.occurred_at,
                    reported_at: Some(clue.reported_at),
                    review_status: clue.status,
                    related_task_id: clue.linked_task_reference,
                    updated_at: clue.updated_at,
                }),
        );
    }
    items.extend(task_items.into_iter().map(|task| {
        CaseMapItem {
            id: task.id.clone(),
            object_type: "task".to_owned(),
            display_name: Some(task.title),
            longitude: task.longitude,
            latitude: task.latitude,
            location_text: Some(task.area_text),
            location_precision: if task.longitude.is_some() {
                "exact"
            } else {
                "unknown"
            }
            .to_owned(),
            source: "task".to_owned(),
            occurred_at: Some(task.due_at),
            reported_at: Some(task.created_at),
            review_status: task.status,
            related_task_id: Some(task.id),
            updated_at: task.updated_at,
        }
    }));
    Ok(HttpResponse::Ok().json(CaseMapViewResponse { items }))
}

async fn get_case_summary(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
    case_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        crate::services::case_summary_service::get_case_summary(&state.db, &auth, &case_id).await?,
    ))
}

async fn get_public_progress(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
    case_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        crate::services::case_collaboration_service::get_public_progress(
            &state.db, &auth, &case_id,
        )
        .await?,
    ))
}

async fn create_clue_drafts(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
    case_id: web::Path<String>,
    request: web::Json<CreateClueDraftRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created().json(
        crate::services::case_collaboration_service::create_clue_drafts(
            &state.db,
            &auth,
            &case_id,
            request.into_inner(),
            &state.ai_gateway,
        )
        .await?,
    ))
}

async fn list_case_pois(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
    case_id: web::Path<String>,
    query: web::Query<CasePoiQuery>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        crate::services::case_collaboration_service::list_case_pois(
            &state.db,
            &auth,
            &case_id,
            query.into_inner(),
            &state.amap_service,
        )
        .await?,
    ))
}

async fn create_summary_draft(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
    case_id: web::Path<String>,
    request: web::Json<CreateSummaryDraftRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created().json(
        crate::services::case_collaboration_service::create_summary_draft(
            &state.db,
            &auth,
            &case_id,
            request.into_inner(),
            &state.ai_gateway,
        )
        .await?,
    ))
}

async fn review_summary_draft(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
    request: web::Json<ReviewSummaryDraftRequest>,
) -> Result<HttpResponse, ApiError> {
    let (case_id, draft_id) = path.into_inner();
    Ok(HttpResponse::Ok().json(
        crate::services::case_collaboration_service::review_summary_draft(
            &state.db,
            &auth,
            &case_id,
            &draft_id,
            request.into_inner(),
        )
        .await?,
    ))
}

async fn update_case_status(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<UpdateCaseStatusRequest>,
) -> Result<HttpResponse, ApiError> {
    let case =
        case_service::update_case_status(&state.db, &auth, &case_id, request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(case))
}

async fn update_elder_profile(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<UpdateElderProfileRequest>,
) -> Result<HttpResponse, ApiError> {
    let case = case_service::update_elder_profile(&state.db, &auth, &case_id, request.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(case))
}

async fn create_clue(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<CreateClueRequest>,
) -> Result<HttpResponse, ApiError> {
    let clue = case_service::create_clue(&state.db, &auth, &case_id, request.into_inner()).await?;
    Ok(HttpResponse::Created().json(clue))
}

async fn list_clues(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    query: web::Query<crate::models::ClueTimelineQuery>,
) -> Result<HttpResponse, ApiError> {
    let clues = case_service::list_clues(&state.db, &auth, &case_id, query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(clues))
}

async fn create_task(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<CreateTaskRequest>,
) -> Result<HttpResponse, ApiError> {
    let task = crate::services::task_service::create_task(
        &state.db,
        &auth,
        &case_id,
        request.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Created().json(task))
}

async fn list_tasks(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    query: web::Query<TaskListQuery>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        crate::services::task_service::list_tasks(&state.db, &auth, &case_id, query.into_inner())
            .await?,
    ))
}

async fn add_case_member(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<AddCaseMemberRequest>,
) -> Result<HttpResponse, ApiError> {
    let member =
        case_service::add_case_member(&state.db, &auth, &case_id, request.into_inner()).await?;
    Ok(HttpResponse::Created().json(member))
}
