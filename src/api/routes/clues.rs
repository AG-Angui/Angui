use actix_multipart::Multipart;
use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{AuthenticatedUser, ReviewClueRequest},
    services::case_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/clues")
            .route("/{clue_id}/review", web::patch().to(review_clue))
            .route("/{clue_id}/attachments", web::post().to(create_attachment)),
    );
}

async fn review_clue(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    clue_id: web::Path<String>,
    request: web::Json<ReviewClueRequest>,
) -> Result<HttpResponse, ApiError> {
    let clue = case_service::review_clue(&state.db, &auth, &clue_id, request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(clue))
}

async fn create_attachment(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    clue_id: web::Path<String>,
    multipart: Multipart,
) -> Result<HttpResponse, ApiError> {
    let (filename, content_type, bytes) =
        crate::services::case_resource_service::read_single_image_upload(
            multipart,
            state.attachment_max_image_bytes,
        )
        .await?;
    let attachment = crate::services::case_resource_service::store_image_attachment_for_clue(
        &state.db,
        &auth,
        &clue_id,
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
