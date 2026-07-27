use actix_multipart::Multipart;
use actix_web::{HttpResponse, http::header, web};
use futures_util::StreamExt;

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AddCaseMemberRequest, AuthenticatedUser, CaseResourceConfigurationResponse,
        CreateCasePlaceRequest, CreateCaseRequest, CreateClueRequest, CreateTaskRequest,
        TaskListQuery, UpdateCaseStatusRequest, UpdateElderProfileRequest,
    },
    roles::CaseRole,
    services::case_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/cases")
            .route("", web::get().to(list_cases))
            .route("", web::post().to(create_case))
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
            .route("/{case_id}/members", web::post().to(add_case_member)),
    );
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
    mut multipart: Multipart,
) -> Result<HttpResponse, ApiError> {
    let mut file: Option<(String, String, Vec<u8>)> = None;
    while let Some(item) = multipart.next().await {
        let mut field =
            item.map_err(|_| ApiError::Validation("multipart upload is malformed".to_owned()))?;
        if field.name() != Some("file") || file.is_some() {
            return Err(ApiError::Validation(
                "submit exactly one file field".to_owned(),
            ));
        }
        let filename = field
            .content_disposition()
            .and_then(|value| value.get_filename())
            .ok_or_else(|| ApiError::Validation("file name is required".to_owned()))?
            .to_owned();
        let content_type = field
            .content_type()
            .map(|value| value.essence_str().to_owned())
            .ok_or_else(|| ApiError::Validation("file content type is required".to_owned()))?;
        let mut bytes = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk = chunk
                .map_err(|_| ApiError::Validation("file upload could not be read".to_owned()))?;
            if bytes.len().saturating_add(chunk.len()) > state.attachment_max_image_bytes {
                return Err(ApiError::Validation(format!(
                    "image must not exceed {} bytes",
                    state.attachment_max_image_bytes
                )));
            }
            bytes.extend_from_slice(&chunk);
        }
        file = Some((filename, content_type, bytes));
    }
    let (filename, content_type, bytes) =
        file.ok_or_else(|| ApiError::Validation("file field is required".to_owned()))?;
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
