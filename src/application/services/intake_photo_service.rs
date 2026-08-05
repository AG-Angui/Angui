use std::{fs, path::Path};

use actix_web::web;
use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set, TransactionTrait,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    entities::{intake_session_photos, intake_sessions},
    error::ApiError,
    models::{AuthenticatedUser, IntakePhotoResponse},
    services::{
        case_resource_service::{AttachmentUpload, normalize_image_upload},
        intake_session_service,
    },
};

const MAX_PHOTOS_PER_SESSION: u64 = 4;

pub async fn store_photo(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    upload: AttachmentUpload<'_>,
    directory: &Path,
    max_image_bytes: usize,
) -> Result<IntakePhotoResponse, ApiError> {
    let (content_type, original_filename, normalized) =
        normalize_image_upload(upload, max_image_bytes).await?;
    let transaction = db.begin().await?;
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_creator(&session, auth)?;
    if !matches!(
        session.status.as_str(),
        "collecting" | "ready_for_confirmation"
    ) {
        return Err(ApiError::Conflict(
            "intake session is not accepting photos".to_owned(),
        ));
    }
    let count = intake_session_photos::Entity::find()
        .filter(intake_session_photos::Column::SessionId.eq(session_id))
        .count(&transaction)
        .await?;
    if count >= MAX_PHOTOS_PER_SESSION {
        return Err(ApiError::Validation(
            "an intake session can contain at most four photos".to_owned(),
        ));
    }
    let id = Uuid::new_v4().to_string();
    let extension = match content_type.as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        _ => return Err(ApiError::Internal),
    };
    let storage_key = format!("intake/{id}.{extension}");
    let storage_path = directory.join(&storage_key);
    let parent = storage_path
        .parent()
        .ok_or(ApiError::Internal)?
        .to_path_buf();
    let path_for_write = storage_path.clone();
    let bytes_for_write = normalized.clone();
    web::block(move || {
        fs::create_dir_all(parent)?;
        fs::write(path_for_write, bytes_for_write)
    })
    .await
    .map_err(|_| ApiError::Internal)?
    .map_err(|_| ApiError::Internal)?;
    let timestamp = now();
    let model = intake_session_photos::ActiveModel {
        id: Set(id),
        session_id: Set(session_id.to_owned()),
        storage_key: Set(storage_key),
        original_filename: Set(original_filename),
        content_type: Set(content_type),
        byte_size: Set(normalized.len() as i64),
        sha256: Set(hex::encode(Sha256::digest(&normalized))),
        created_by_user_id: Set(auth.id.clone()),
        created_at: Set(timestamp),
    }
    .insert(&transaction)
    .await;
    let model = match model {
        Ok(value) => value,
        Err(error) => {
            let _ = web::block(move || fs::remove_file(storage_path)).await;
            return Err(ApiError::Database(error));
        }
    };
    intake_session_service::write_attachment_audit(
        &transaction,
        auth,
        "intake_session.photo_uploaded",
        session_id,
        &model.id,
        &model.content_type,
    )
    .await?;
    transaction.commit().await?;
    Ok(response(model))
}

pub async fn list_photos(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
) -> Result<Vec<IntakePhotoResponse>, ApiError> {
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_creator(&session, auth)?;
    Ok(intake_session_photos::Entity::find()
        .filter(intake_session_photos::Column::SessionId.eq(session_id))
        .all(db)
        .await?
        .into_iter()
        .map(response)
        .collect())
}

pub async fn load_photo(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    session_id: &str,
    photo_id: &str,
    directory: &Path,
) -> Result<crate::services::case_resource_service::DownloadedAttachment, ApiError> {
    let session = intake_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    require_creator(&session, auth)?;
    let photo = intake_session_photos::Entity::find_by_id(photo_id)
        .one(db)
        .await?
        .filter(|photo| photo.session_id == session_id)
        .ok_or_else(|| ApiError::NotFound("intake photo was not found".to_owned()))?;
    let path = directory.join(photo.storage_key);
    let bytes = web::block(move || fs::read(path))
        .await
        .map_err(|_| ApiError::Internal)?
        .map_err(|_| ApiError::Internal)?;
    Ok(
        crate::services::case_resource_service::DownloadedAttachment {
            bytes,
            content_type: photo.content_type,
            filename: photo.original_filename,
        },
    )
}

fn response(model: intake_session_photos::Model) -> IntakePhotoResponse {
    IntakePhotoResponse {
        id: model.id,
        original_filename: model.original_filename,
        content_type: model.content_type,
        byte_size: model.byte_size,
        created_at: model.created_at,
    }
}
fn require_creator(
    session: &intake_sessions::Model,
    auth: &AuthenticatedUser,
) -> Result<(), ApiError> {
    if session.created_by_user_id != auth.id {
        return Err(ApiError::NotFound(
            "intake session was not found".to_owned(),
        ));
    }
    Ok(())
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
