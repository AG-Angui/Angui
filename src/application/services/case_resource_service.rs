use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use actix_multipart::Multipart;
use actix_web::web;
use chrono::{SecondsFormat, Utc};
use futures_util::StreamExt;
use image::ImageFormat;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, DatabaseTransaction, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    entities::{
        case_attachments, case_places, cases, clue_attachment_links, clue_attributions, clues,
    },
    error::ApiError,
    models::{
        AuthenticatedUser, CaseAttachmentResponse, CasePlaceResponse, CreateCasePlaceRequest,
    },
    roles::CaseRole,
    services::case_service::{require_case_role, write_audit},
};

pub struct AttachmentUpload<'a> {
    pub filename: &'a str,
    pub declared_content_type: &'a str,
    pub bytes: &'a [u8],
}

pub struct AttachmentStorage<'a> {
    pub directory: &'a Path,
    pub max_image_bytes: usize,
    pub max_attachments_per_case: u64,
}

pub async fn read_single_image_upload(
    mut multipart: Multipart,
    max_image_bytes: usize,
) -> Result<(String, String, Vec<u8>), ApiError> {
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
            if bytes.len().saturating_add(chunk.len()) > max_image_bytes {
                return Err(ApiError::Validation(format!(
                    "image must not exceed {max_image_bytes} bytes"
                )));
            }
            bytes.extend_from_slice(&chunk);
        }
        file = Some((filename, content_type, bytes));
    }
    file.ok_or_else(|| ApiError::Validation("file field is required".to_owned()))
}

pub async fn create_place(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: CreateCasePlaceRequest,
    allowed_place_types: &[String],
) -> Result<CasePlaceResponse, ApiError> {
    validate_place(&request, allowed_place_types)?;
    let transaction = db.begin().await?;
    let role = require_case_role(
        &transaction,
        &auth.id,
        case_id,
        &[CaseRole::Family, CaseRole::Commander],
    )
    .await?;
    ensure_case_is_open(&transaction, case_id).await?;
    let timestamp = now();
    let model = case_places::ActiveModel {
        id: Set(new_id()),
        case_id: Set(case_id.to_owned()),
        name: Set(request.name.trim().to_owned()),
        place_type: Set(request.place_type.trim().to_lowercase()),
        address: Set(request.address.trim().to_owned()),
        longitude: Set(request.longitude),
        latitude: Set(request.latitude),
        source: Set(role.to_string()),
        visibility: Set(request.visibility.as_str().to_owned()),
        review_status: Set("pending_review".to_owned()),
        created_by_user_id: Set(auth.id.clone()),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp),
    }
    .insert(&transaction)
    .await?;
    write_audit(&transaction, Some(case_id.to_owned()), auth, "case.place_submitted", "case_place", model.id.clone(), Some(json!({ "review_status": "pending_review", "visibility": model.visibility, "actor_case_role": role }))).await?;
    transaction.commit().await?;
    Ok(place_response(model, &auth.id))
}

pub async fn store_image_attachment(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    upload: AttachmentUpload<'_>,
    storage: AttachmentStorage<'_>,
) -> Result<CaseAttachmentResponse, ApiError> {
    let (content_type, original_filename, normalized) =
        normalize_attachment_upload(upload, storage.max_image_bytes).await?;
    let transaction = db.begin().await?;
    let role = require_case_role(
        &transaction,
        &auth.id,
        case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    ensure_case_is_open(&transaction, case_id).await?;
    let (model, storage_path) = persist_image_attachment(
        &transaction,
        auth,
        case_id,
        role,
        content_type,
        original_filename,
        normalized,
        storage,
        None,
    )
    .await?;
    if let Err(error) = transaction.commit().await {
        remove_file_best_effort(storage_path).await;
        return Err(ApiError::Database(error));
    }
    Ok(attachment_response(model, &auth.id))
}

pub async fn store_image_attachment_for_clue(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    clue_id: &str,
    upload: AttachmentUpload<'_>,
    storage: AttachmentStorage<'_>,
) -> Result<CaseAttachmentResponse, ApiError> {
    let (content_type, original_filename, normalized) =
        normalize_attachment_upload(upload, storage.max_image_bytes).await?;
    let transaction = db.begin().await?;
    let clue = clues::Entity::find_by_id(clue_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("clue was not found".to_owned()))?;
    let role = require_case_role(
        &transaction,
        &auth.id,
        &clue.case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    if role != CaseRole::Commander {
        let is_submitter = clue_attributions::Entity::find_by_id(&clue.id)
            .one(&transaction)
            .await?
            .and_then(|attribution| attribution.submitted_by_user_id)
            .as_deref()
            == Some(auth.id.as_str());
        if !is_submitter {
            return Err(ApiError::NotFound("clue was not found".to_owned()));
        }
    }
    ensure_case_is_open(&transaction, &clue.case_id).await?;
    let (model, storage_path) = persist_image_attachment(
        &transaction,
        auth,
        &clue.case_id,
        role,
        content_type,
        original_filename,
        normalized,
        storage,
        Some(&clue.id),
    )
    .await?;
    if let Err(error) = transaction.commit().await {
        remove_file_best_effort(storage_path).await;
        return Err(ApiError::Database(error));
    }
    Ok(attachment_response(model, &auth.id))
}

pub async fn load_attachment_for_download(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    attachment_id: &str,
    storage_directory: &Path,
) -> Result<DownloadedAttachment, ApiError> {
    let role = require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let attachment = case_attachments::Entity::find_by_id(attachment_id)
        .one(db)
        .await?
        .filter(|record| record.case_id == case_id)
        .ok_or_else(|| ApiError::NotFound("attachment was not found".to_owned()))?;
    let allowed = role == CaseRole::Commander || attachment.created_by_user_id == auth.id;
    if !allowed {
        return Err(ApiError::Forbidden(
            "this attachment is not available to the current case role".to_owned(),
        ));
    }
    let storage_path = storage_directory.join(&attachment.storage_key);
    let bytes = web::block(move || fs::read(storage_path))
        .await
        .map_err(|_| ApiError::Internal)?
        .map_err(|_| ApiError::Internal)?;
    Ok(DownloadedAttachment {
        bytes,
        content_type: attachment.content_type,
        filename: attachment.original_filename,
    })
}

pub async fn visible_places(
    db: &DatabaseConnection,
    case_id: &str,
    viewer_id: &str,
    role: CaseRole,
) -> Result<Vec<CasePlaceResponse>, ApiError> {
    let mut query = case_places::Entity::find().filter(case_places::Column::CaseId.eq(case_id));
    query = match role {
        CaseRole::Commander => query,
        CaseRole::Family => query.filter(
            Condition::any()
                .add(case_places::Column::CreatedByUserId.eq(viewer_id))
                .add(
                    Condition::all()
                        .add(case_places::Column::ReviewStatus.eq("confirmed"))
                        .add(case_places::Column::Visibility.ne("internal")),
                ),
        ),
        CaseRole::Volunteer => query
            .filter(case_places::Column::Visibility.eq("public"))
            .filter(case_places::Column::ReviewStatus.eq("confirmed")),
    };
    let records = query
        .order_by_desc(case_places::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(records
        .into_iter()
        .map(|place| place_response(place, viewer_id))
        .collect())
}

pub async fn visible_attachments(
    db: &DatabaseConnection,
    case_id: &str,
    viewer_id: &str,
    role: CaseRole,
) -> Result<Vec<CaseAttachmentResponse>, ApiError> {
    let mut query =
        case_attachments::Entity::find().filter(case_attachments::Column::CaseId.eq(case_id));
    if role != CaseRole::Commander {
        query = query.filter(case_attachments::Column::CreatedByUserId.eq(viewer_id));
    }
    let records = query
        .order_by_desc(case_attachments::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(records
        .into_iter()
        .filter(|attachment| {
            role == CaseRole::Commander || attachment.created_by_user_id == viewer_id
        })
        .map(|attachment| attachment_response(attachment, viewer_id))
        .collect())
}

pub struct DownloadedAttachment {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub filename: String,
}

async fn normalize_attachment_upload(
    upload: AttachmentUpload<'_>,
    max_image_bytes: usize,
) -> Result<(String, String, Vec<u8>), ApiError> {
    let declared_content_type = upload.declared_content_type.to_owned();
    let uploaded_bytes = upload.bytes.to_vec();
    let (content_type, extension, normalized) = web::block(move || {
        normalize_image(&declared_content_type, &uploaded_bytes, max_image_bytes)
    })
    .await
    .map_err(|_| ApiError::Internal)??;
    Ok((
        content_type,
        safe_filename(upload.filename, extension)?,
        normalized,
    ))
}

#[allow(clippy::too_many_arguments)]
async fn persist_image_attachment(
    transaction: &DatabaseTransaction,
    auth: &AuthenticatedUser,
    case_id: &str,
    role: CaseRole,
    content_type: String,
    original_filename: String,
    normalized: Vec<u8>,
    storage: AttachmentStorage<'_>,
    clue_id: Option<&str>,
) -> Result<(case_attachments::Model, PathBuf), ApiError> {
    let current_count = case_attachments::Entity::find()
        .filter(case_attachments::Column::CaseId.eq(case_id))
        .count(transaction)
        .await?;
    if current_count >= storage.max_attachments_per_case {
        return Err(ApiError::Validation(
            "this case already has the maximum number of attachments".to_owned(),
        ));
    }

    let extension = match content_type.as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        _ => return Err(ApiError::Internal),
    };
    let id = new_id();
    let storage_key = format!("{id}.{extension}");
    let storage_path = storage.directory.join(&storage_key);
    let storage_directory = storage.directory.to_path_buf();
    let path_for_write = storage_path.clone();
    let bytes_for_write = normalized.clone();
    web::block(move || {
        fs::create_dir_all(storage_directory)?;
        fs::write(path_for_write, bytes_for_write)
    })
    .await
    .map_err(|_| ApiError::Internal)?
    .map_err(|_| ApiError::Internal)?;

    let timestamp = now();
    let model = case_attachments::ActiveModel {
        id: Set(id),
        case_id: Set(case_id.to_owned()),
        storage_key: Set(storage_key),
        original_filename: Set(original_filename),
        content_type: Set(content_type),
        byte_size: Set(normalized.len() as i64),
        sha256: Set(hex::encode(Sha256::digest(&normalized))),
        source: Set(role.to_string()),
        review_status: Set("pending_review".to_owned()),
        created_by_user_id: Set(auth.id.clone()),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp.clone()),
    }
    .insert(transaction)
    .await;
    let model = match model {
        Ok(model) => model,
        Err(error) => {
            remove_file_best_effort(storage_path.clone()).await;
            return Err(ApiError::Database(error));
        }
    };

    let (action, entity_type, entity_id, metadata) = if let Some(clue_id) = clue_id {
        let link = clue_attachment_links::ActiveModel {
            clue_id: Set(clue_id.to_owned()),
            attachment_id: Set(model.id.clone()),
            created_at: Set(timestamp),
        }
        .insert(transaction)
        .await;
        if let Err(error) = link {
            remove_file_best_effort(storage_path.clone()).await;
            return Err(ApiError::Database(error));
        }
        (
            "clue.attachment_submitted",
            "clue_attachment",
            model.id.clone(),
            json!({
                "clue_id": clue_id,
                "review_status": "pending_review",
                "content_type": model.content_type,
            }),
        )
    } else {
        (
            "case.attachment_submitted",
            "case_attachment",
            model.id.clone(),
            json!({ "review_status": "pending_review", "content_type": model.content_type }),
        )
    };
    if let Err(error) = write_audit(
        transaction,
        Some(case_id.to_owned()),
        auth,
        action,
        entity_type,
        entity_id,
        Some(metadata),
    )
    .await
    {
        remove_file_best_effort(storage_path.clone()).await;
        return Err(error);
    }
    Ok((model, storage_path))
}

fn validate_place(
    request: &CreateCasePlaceRequest,
    allowed_place_types: &[String],
) -> Result<(), ApiError> {
    for (label, value, maximum) in [
        ("name", request.name.trim(), 120),
        ("address", request.address.trim(), 500),
    ] {
        if value.is_empty() || value.chars().count() > maximum {
            return Err(ApiError::Validation(format!(
                "{label} must contain between 1 and {maximum} characters"
            )));
        }
    }
    let place_type = request.place_type.trim().to_lowercase();
    if !allowed_place_types
        .iter()
        .any(|allowed| allowed == &place_type)
    {
        return Err(ApiError::Validation(
            "place_type is not supported".to_owned(),
        ));
    }
    match (request.longitude, request.latitude) {
        (Some(longitude), Some(latitude))
            if (-180.0..=180.0).contains(&longitude) && (-90.0..=90.0).contains(&latitude) =>
        {
            Ok(())
        }
        (None, None) => Ok(()),
        _ => Err(ApiError::Validation(
            "longitude and latitude must be supplied together and be in range".to_owned(),
        )),
    }
}

fn normalize_image(
    declared_content_type: &str,
    bytes: &[u8],
    max_image_bytes: usize,
) -> Result<(String, &'static str, Vec<u8>), ApiError> {
    if bytes.is_empty() || bytes.len() > max_image_bytes {
        return Err(ApiError::Validation(format!(
            "image must contain between 1 byte and {max_image_bytes} bytes"
        )));
    }
    let format = image::guess_format(bytes)
        .map_err(|_| ApiError::Validation("file content is not a supported image".to_owned()))?;
    let (content_type, extension) = match format {
        ImageFormat::Jpeg => ("image/jpeg", "jpg"),
        ImageFormat::Png => ("image/png", "png"),
        _ => {
            return Err(ApiError::Validation(
                "only JPEG and PNG images are accepted".to_owned(),
            ));
        }
    };
    let declared_essence = declared_content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if declared_essence != content_type {
        return Err(ApiError::Validation(
            "declared content type does not match image content".to_owned(),
        ));
    }
    let decoded = image::load_from_memory_with_format(bytes, format)
        .map_err(|_| ApiError::Validation("image data could not be decoded".to_owned()))?;
    if decoded.width() > 8_000
        || decoded.height() > 8_000
        || u64::from(decoded.width()) * u64::from(decoded.height()) > 20_000_000
    {
        return Err(ApiError::Validation(
            "image dimensions exceed the allowed limit".to_owned(),
        ));
    }
    let mut output = Cursor::new(Vec::new());
    decoded
        .write_to(&mut output, format)
        .map_err(|_| ApiError::Validation("image could not be normalized".to_owned()))?;
    let output = output.into_inner();
    if output.len() > max_image_bytes {
        return Err(ApiError::Validation(format!(
            "normalized image exceeds the {max_image_bytes}-byte limit"
        )));
    }
    Ok((content_type.to_owned(), extension, output))
}

async fn remove_file_best_effort(path: PathBuf) {
    let _ = web::block(move || fs::remove_file(path)).await;
}

fn safe_filename(filename: &str, extension: &str) -> Result<String, ApiError> {
    let name = Path::new(filename)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("image");
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let normalized: String = stem
        .chars()
        .filter_map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => Some(character),
            ' ' => Some('_'),
            _ => None,
        })
        .take(100)
        .collect();
    if normalized.is_empty() {
        return Err(ApiError::Validation("filename is invalid".to_owned()));
    }
    Ok(format!("{normalized}.{extension}"))
}

async fn ensure_case_is_open<C: sea_orm::ConnectionTrait>(
    db: &C,
    case_id: &str,
) -> Result<(), ApiError> {
    let case_model = cases::Entity::find_by_id(case_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("case was not found".to_owned()))?;
    if case_model.status == "closed" {
        return Err(ApiError::Conflict(
            "new supplementary information cannot be added to a closed case".to_owned(),
        ));
    }
    Ok(())
}

fn place_response(model: case_places::Model, viewer_id: &str) -> CasePlaceResponse {
    CasePlaceResponse {
        id: model.id,
        case_id: model.case_id,
        name: model.name,
        place_type: model.place_type,
        address: model.address,
        longitude: model.longitude,
        latitude: model.latitude,
        source: model.source,
        visibility: model.visibility,
        review_status: model.review_status,
        created_at: model.created_at,
        updated_at: model.updated_at,
        is_own_submission: model.created_by_user_id == viewer_id,
    }
}
fn attachment_response(model: case_attachments::Model, viewer_id: &str) -> CaseAttachmentResponse {
    CaseAttachmentResponse {
        id: model.id,
        case_id: model.case_id,
        original_filename: model.original_filename,
        content_type: model.content_type,
        byte_size: model.byte_size,
        source: model.source,
        review_status: model.review_status,
        created_at: model.created_at,
        updated_at: model.updated_at,
        is_own_submission: model.created_by_user_id == viewer_id,
    }
}
fn new_id() -> String {
    Uuid::new_v4().to_string()
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
