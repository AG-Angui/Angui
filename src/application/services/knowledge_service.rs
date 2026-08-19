use std::{
    fs,
    path::{Path, PathBuf},
};

use std::collections::{HashMap, HashSet};

use actix_web::web;
use chrono::{SecondsFormat, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use sha2::{Digest, Sha256};

use crate::ai_gateway::{
    AiCapability, AiExecutionResult, AiGateway, AiPurpose, AiRequest, DataLevel,
};
use crate::{
    entities::{
        knowledge_bases, knowledge_images, knowledge_import_batches, knowledge_import_rows,
        knowledge_items,
    },
    error::ApiError,
    models::{
        AuthenticatedUser, CreateKnowledgeBaseRequest, CreateKnowledgeItemRequest,
        KnowledgeBaseOverviewResponse, KnowledgeBaseResponse, KnowledgeChatResponse,
        KnowledgeChatSourceResponse, KnowledgeImageInput, KnowledgeImageResponse,
        KnowledgeImportBatchResponse, KnowledgeImportRowResponse, KnowledgeOverviewResponse,
        KnowledgeSearchResponse, KnowledgeSearchResultResponse, UpdateKnowledgeBaseRequest,
        UpdateKnowledgeItemRequest,
    },
    roles::{AccountType, GlobalCapability},
    services::{
        case_resource_service::{AttachmentUpload, normalize_image_upload},
        case_service,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};

const MAX_QUERY_LENGTH: usize = 1_000;
const DEFAULT_LIMIT: u32 = 5;
const MAX_LIMIT: u32 = 20;
const MAX_IMPORT_BYTES: usize = 5 * 1024 * 1024;
const MAX_IMPORT_ROWS: usize = 5_000;
const CSV_HEADERS: [&str; 9] = [
    "knowledge_base_id",
    "title",
    "content",
    "summary",
    "category",
    "keywords",
    "source_name",
    "source_url",
    "visibility",
];

pub async fn list_bases(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
) -> Result<Vec<KnowledgeBaseResponse>, ApiError> {
    require_admin(auth)?;
    knowledge_bases::Entity::find()
        .order_by_asc(knowledge_bases::Column::Name)
        .all(db)
        .await?
        .into_iter()
        .map(base_response)
        .collect()
}
pub async fn overview(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
) -> Result<KnowledgeOverviewResponse, ApiError> {
    require_admin(auth)?;
    let bases = knowledge_bases::Entity::find()
        .order_by_asc(knowledge_bases::Column::Name)
        .all(db)
        .await?;
    let mut total_items = 0;
    let mut draft_items = 0;
    let mut reviewed_items = 0;
    let mut published_items = 0;
    let mut withdrawn_items = 0;
    let mut image_count = 0;
    let mut summaries = Vec::with_capacity(bases.len());
    for base in bases {
        let items = knowledge_items::Entity::find()
            .filter(knowledge_items::Column::KnowledgeBaseId.eq(&base.id))
            .all(db)
            .await?;
        let ids: Vec<_> = items.iter().map(|item| item.id.clone()).collect();
        let base_images = if ids.is_empty() {
            0
        } else {
            knowledge_images::Entity::find()
                .filter(knowledge_images::Column::KnowledgeItemId.is_in(ids))
                .count(db)
                .await?
        };
        let count = |status: &str| items.iter().filter(|item| item.status == status).count() as u64;
        let total = items.len() as u64;
        let draft = count("draft") + count("submitted");
        let reviewed = count("reviewed");
        let published = count("published");
        let withdrawn = count("withdrawn");
        total_items += total;
        draft_items += draft;
        reviewed_items += reviewed;
        published_items += published;
        withdrawn_items += withdrawn;
        image_count += base_images;
        summaries.push(KnowledgeBaseOverviewResponse {
            id: base.id,
            name: base.name,
            status: base.status,
            visibility: base.visibility,
            total_items: total,
            draft_items: draft,
            reviewed_items: reviewed,
            published_items: published,
            withdrawn_items: withdrawn,
            image_count: base_images,
        });
    }
    let total_bases = summaries.len() as u64;
    let enabled_bases = summaries
        .iter()
        .filter(|base| base.status == "enabled")
        .count() as u64;
    Ok(KnowledgeOverviewResponse {
        total_bases,
        enabled_bases,
        total_items,
        draft_items,
        reviewed_items,
        published_items,
        withdrawn_items,
        image_count,
        bases: summaries,
    })
}

pub async fn get_base(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    id: &str,
) -> Result<KnowledgeBaseResponse, ApiError> {
    require_admin(auth)?;
    base_response(find_base(db, id).await?)
}

pub async fn create_base(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    request: CreateKnowledgeBaseRequest,
) -> Result<KnowledgeBaseResponse, ApiError> {
    require_admin(auth)?;
    let timestamp = now();
    let model = knowledge_bases::ActiveModel {
        id: Set(case_service::new_id()),
        name: Set(required(&request.name, "name", 160)?),
        description: Set(optional(&request.description, 2_000)?),
        visibility: Set(visibility(&request.visibility)?),
        status: Set("enabled".to_owned()),
        created_by_user_id: Set(auth.id.clone()),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp),
    }
    .insert(db)
    .await?;
    base_response(model)
}

pub async fn update_base(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    id: &str,
    request: UpdateKnowledgeBaseRequest,
) -> Result<KnowledgeBaseResponse, ApiError> {
    require_admin(auth)?;
    let existing = find_base(db, id).await?;
    let mut active: knowledge_bases::ActiveModel = existing.into();
    if let Some(name) = request.name {
        active.name = Set(required(&name, "name", 160)?);
    }
    if let Some(description) = request.description {
        active.description = Set(optional(&description, 2_000)?);
    }
    if let Some(value) = request.visibility {
        active.visibility = Set(visibility(&value)?);
    }
    active.updated_at = Set(now());
    base_response(active.update(db).await?)
}

pub async fn set_base_status(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    id: &str,
    status: &str,
) -> Result<KnowledgeBaseResponse, ApiError> {
    require_admin(auth)?;
    let existing = find_base(db, id).await?;
    let mut active: knowledge_bases::ActiveModel = existing.into();
    active.status = Set(status.to_owned());
    active.updated_at = Set(now());
    base_response(active.update(db).await?)
}

pub async fn list_items(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    base_id: &str,
) -> Result<Vec<KnowledgeSearchResultResponse>, ApiError> {
    require_admin(auth)?;
    find_base(db, base_id).await?;
    let items = knowledge_items::Entity::find()
        .filter(knowledge_items::Column::KnowledgeBaseId.eq(base_id))
        .order_by_desc(knowledge_items::Column::UpdatedAt)
        .all(db)
        .await?;
    results_with_images(db, items, None).await
}

pub async fn get_item(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    id: &str,
) -> Result<KnowledgeSearchResultResponse, ApiError> {
    require_admin(auth)?;
    let item = find_item(db, id).await?;
    results_with_images(db, vec![item], None)
        .await?
        .into_iter()
        .next()
        .ok_or(ApiError::Internal)
}

pub async fn upload_image(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    item_id: &str,
    upload: AttachmentUpload<'_>,
    directory: &Path,
    max_image_bytes: usize,
) -> Result<KnowledgeImageResponse, ApiError> {
    require_admin(auth)?;
    let item = find_item(db, item_id).await?;
    let (mime_type, _filename, bytes) = normalize_image_upload(upload, max_image_bytes).await?;
    let image_id = case_service::new_id();
    let extension = if mime_type == "image/png" {
        "png"
    } else {
        "jpg"
    };
    let relative = PathBuf::from("knowledge")
        .join(item_id)
        .join(format!("{image_id}.{extension}"));
    let path = directory.join(&relative);
    let parent = path.parent().ok_or(ApiError::Internal)?.to_path_buf();
    let write_path = path.clone();
    web::block(move || {
        fs::create_dir_all(parent)?;
        fs::write(write_path, bytes)
    })
    .await
    .map_err(|_| ApiError::Internal)?
    .map_err(|_| ApiError::Internal)?;
    let storage_path = format!("/api/admin/knowledge-items/{item_id}/images/{image_id}");
    let image = knowledge_images::ActiveModel {
        id: Set(image_id),
        knowledge_item_id: Set(item.id),
        storage_path: Set(storage_path),
        mime_type: Set(mime_type),
        width: Set(None),
        height: Set(None),
        metadata_json: Set("{}".to_owned()),
        created_at: Set(now()),
    }
    .insert(db)
    .await;
    match image {
        Ok(image) => image_response(image),
        Err(error) => {
            let _ = fs::remove_file(path);
            Err(ApiError::Database(error))
        }
    }
}

pub async fn load_image(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    item_id: &str,
    image_id: &str,
    directory: &Path,
) -> Result<(String, Vec<u8>), ApiError> {
    let item = find_item(db, item_id).await?;
    if !auth.global_capabilities.contains(&GlobalCapability::Admin) {
        let base = find_base(db, &item.knowledge_base_id).await?;
        if base.status != "enabled"
            || item.status != "published"
            || item.effective_at > now()
            || item.withdrawn_at.is_some()
            || !visible_to(auth, &base.visibility)
            || !visible_to(auth, &item.visibility)
        {
            return Err(ApiError::NotFound(
                "knowledge image was not found".to_owned(),
            ));
        }
    }
    let image = knowledge_images::Entity::find_by_id(image_id)
        .one(db)
        .await?
        .filter(|image| image.knowledge_item_id == item_id)
        .ok_or_else(|| ApiError::NotFound("knowledge image was not found".to_owned()))?;
    let extension = if image.mime_type == "image/png" {
        "png"
    } else {
        "jpg"
    };
    let path = directory
        .join("knowledge")
        .join(item_id)
        .join(format!("{image_id}.{extension}"));
    let bytes = web::block(move || fs::read(path))
        .await
        .map_err(|_| ApiError::Internal)?
        .map_err(|_| ApiError::Internal)?;
    Ok((image.mime_type, bytes))
}

pub async fn create_item(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    base_id: &str,
    request: CreateKnowledgeItemRequest,
) -> Result<KnowledgeSearchResultResponse, ApiError> {
    require_admin(auth)?;
    let base = find_base(db, base_id).await?;
    let input = validated_item(request, &base.visibility)?;
    let content_hash = content_hash(
        &input.title,
        &input.summary,
        &input.content,
        &input.keywords,
    );
    if knowledge_items::Entity::find()
        .filter(knowledge_items::Column::KnowledgeBaseId.eq(base_id))
        .filter(knowledge_items::Column::ContentHash.eq(&content_hash))
        .one(db)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict(
            "an item with the same content already exists in this knowledge base".to_owned(),
        ));
    }
    let timestamp = now();
    let transaction = db.begin().await?;
    ensure_unique_images(&transaction, &input.images).await?;
    let item = knowledge_items::ActiveModel {
        id: Set(case_service::new_id()),
        knowledge_base_id: Set(base_id.to_owned()),
        title: Set(input.title),
        summary: Set(input.summary),
        content: Set(input.content),
        category: Set(input.category),
        category_id: Set(input.category_id),
        keywords_json: Set(serde_json::to_string(&input.keywords).map_err(|_| ApiError::Internal)?),
        metadata_json: Set("{}".to_owned()),
        source_name: Set(input.source_name),
        source_url: Set(input.source_url),
        visibility: Set(input.visibility),
        status: Set("draft".to_owned()),
        effective_at: Set(timestamp.clone()),
        withdrawn_at: Set(None),
        previous_version_id: Set(None),
        version: Set(1),
        content_hash: Set(content_hash),
        embedding_json: Set(None),
        embedding_model: Set(None),
        embedding_dimension: Set(None),
        embedding_status: Set("none".to_owned()),
        embedding_generated_at: Set(None),
        embedding_content_hash: Set(None),
        created_by_user_id: Set(auth.id.clone()),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp.clone()),
    }
    .insert(&transaction)
    .await?;
    insert_images(&transaction, &item.id, input.images, &timestamp).await?;
    transaction.commit().await?;
    get_item(db, auth, &item.id).await
}

pub async fn update_item(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    id: &str,
    request: UpdateKnowledgeItemRequest,
) -> Result<KnowledgeSearchResultResponse, ApiError> {
    require_admin(auth)?;
    let existing = find_item(db, id).await?;
    let base = find_base(db, &existing.knowledge_base_id).await?;
    let input = merge_item(existing.clone(), request, &base.visibility)?;
    let hash = content_hash(
        &input.title,
        &input.summary,
        &input.content,
        &input.keywords,
    );
    if knowledge_items::Entity::find()
        .filter(knowledge_items::Column::KnowledgeBaseId.eq(&existing.knowledge_base_id))
        .filter(knowledge_items::Column::ContentHash.eq(&hash))
        .filter(knowledge_items::Column::Id.ne(&existing.id))
        .one(db)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict(
            "an item with the same content already exists in this knowledge base".to_owned(),
        ));
    }
    let transaction = db.begin().await?;
    if let Some(images) = input.images.as_ref() {
        ensure_unique_images_except(&transaction, images, id).await?;
        knowledge_images::Entity::delete_many()
            .filter(knowledge_images::Column::KnowledgeItemId.eq(id))
            .exec(&transaction)
            .await?;
    }
    let mut active: knowledge_items::ActiveModel = existing.into();
    active.title = Set(input.title);
    active.summary = Set(input.summary);
    active.content = Set(input.content);
    active.category = Set(input.category);
    active.category_id = Set(input.category_id);
    active.keywords_json =
        Set(serde_json::to_string(&input.keywords).map_err(|_| ApiError::Internal)?);
    active.source_name = Set(input.source_name);
    active.source_url = Set(input.source_url);
    active.visibility = Set(input.visibility);
    active.content_hash = Set(hash);
    active.status = Set("draft".to_owned());
    active.withdrawn_at = Set(None);
    let timestamp = now();
    active.updated_at = Set(timestamp.clone());
    let item = active.update(&transaction).await?;
    if let Some(images) = input.images {
        insert_images(&transaction, &item.id, images, &timestamp).await?;
    }
    transaction.commit().await?;
    get_item(db, auth, id).await
}

pub async fn transition_item(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    id: &str,
    action: &str,
) -> Result<KnowledgeSearchResultResponse, ApiError> {
    require_admin(auth)?;
    let item = find_item(db, id).await?;
    let target = match (action, item.status.as_str()) {
        ("review", "draft" | "submitted") => "reviewed",
        ("publish", "reviewed") => "published",
        ("withdraw", "published") => "withdrawn",
        _ => {
            return Err(ApiError::Conflict(
                "invalid knowledge item lifecycle transition".to_owned(),
            ));
        }
    };
    let timestamp = now();
    let mut active: knowledge_items::ActiveModel = item.into();
    active.status = Set(target.to_owned());
    active.withdrawn_at = Set((target == "withdrawn").then_some(timestamp.clone()));
    active.updated_at = Set(timestamp);
    active.update(db).await?;
    get_item(db, auth, id).await
}

pub async fn search(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    base_id: &str,
    query: &str,
    limit: Option<u32>,
) -> Result<KnowledgeSearchResponse, ApiError> {
    let query = validate_query(query)?;
    let limit = validate_limit(limit)?;
    let base = find_base(db, base_id).await?;
    if base.status != "enabled" {
        return Err(ApiError::Validation(
            "knowledge base is disabled".to_owned(),
        ));
    }
    if !visible_to(auth, &base.visibility) {
        return Err(ApiError::Forbidden(
            "knowledge base is not visible to this account".to_owned(),
        ));
    }
    let now = now();
    let postgres_full_text = db.get_database_backend() == DbBackend::Postgres;
    let items = if postgres_full_text {
        knowledge_items::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT * FROM knowledge_items \
                 WHERE knowledge_base_id = $1 AND status = 'published' \
                   AND effective_at <= $2 AND withdrawn_at IS NULL \
                   AND search_vector @@ websearch_to_tsquery('simple', $3) \
                 ORDER BY ts_rank_cd(search_vector, websearch_to_tsquery('simple', $3)) DESC, title ASC \
                 LIMIT $4",
                vec![
                    base_id.into(),
                    now.into(),
                    query.into(),
                    i64::from(limit).into(),
                ],
            ))
            .all(db)
            .await?
    } else {
        knowledge_items::Entity::find()
            .filter(knowledge_items::Column::KnowledgeBaseId.eq(base_id))
            .filter(knowledge_items::Column::Status.eq("published"))
            .filter(knowledge_items::Column::EffectiveAt.lte(now))
            .filter(knowledge_items::Column::WithdrawnAt.is_null())
            .all(db)
            .await?
    }
        .into_iter()
        .filter(|item| visible_to(auth, &item.visibility))
        .filter_map(|item| score_item(&item, query).map(|score| (item, score)))
        .collect::<Vec<_>>();
    let mut items = items;
    if !postgres_full_text {
        items.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.title.cmp(&b.0.title)));
    }
    let scored: HashMap<String, f64> = items
        .iter()
        .take(limit as usize)
        .map(|(item, score)| (item.id.clone(), *score))
        .collect();
    let results = results_with_images(
        db,
        items
            .into_iter()
            .take(limit as usize)
            .map(|(item, _)| item)
            .collect(),
        Some(&scored),
    )
    .await?;
    Ok(KnowledgeSearchResponse { results })
}

pub async fn chat(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    base_id: &str,
    query: &str,
    limit: Option<u32>,
) -> Result<KnowledgeChatResponse, ApiError> {
    let results = search(db, auth, base_id, query, limit).await?.results;
    let sources = results
        .iter()
        .map(|result| KnowledgeChatSourceResponse {
            knowledge_item_id: result.knowledge_item_id.clone(),
            title: result.title.clone(),
            version: result.version,
            score: result.score,
            images: result.images.clone(),
        })
        .collect();
    if results.is_empty() {
        return Ok(KnowledgeChatResponse { answer: "The published knowledge base does not contain enough material to answer this question.".to_owned(), certainty: "insufficient_sources".to_owned(), sources, human_review_notice: review_notice().to_owned() });
    }
    let answer = results
        .iter()
        .map(|result| format!("{} [source:{}]", result.content, result.knowledge_item_id))
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(KnowledgeChatResponse {
        answer,
        certainty: "rule_based".to_owned(),
        sources,
        human_review_notice: review_notice().to_owned(),
    })
}

pub async fn chat_with_gateway(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    base_id: &str,
    query: &str,
    limit: Option<u32>,
    gateway: &AiGateway,
) -> Result<KnowledgeChatResponse, ApiError> {
    let results = search(db, auth, base_id, query, limit).await?.results;
    let sources = results
        .iter()
        .map(|result| KnowledgeChatSourceResponse {
            knowledge_item_id: result.knowledge_item_id.clone(),
            title: result.title.clone(),
            version: result.version,
            score: result.score,
            images: result.images.clone(),
        })
        .collect::<Vec<_>>();
    if results.is_empty() {
        return Ok(KnowledgeChatResponse { answer: "The published knowledge base does not contain enough material to answer this question.".to_owned(), certainty: "insufficient_sources".to_owned(), sources, human_review_notice: review_notice().to_owned() });
    }
    let deterministic = results
        .iter()
        .map(|result| format!("{} [source:{}]", result.content, result.knowledge_item_id))
        .collect::<Vec<_>>()
        .join("\n\n");
    let context = build_context(&results);
    let request = AiRequest {
        capability: AiCapability::KnowledgeAnswer,
        data_level: DataLevel::Public,
        purpose: AiPurpose::KnowledgeAnswer,
        data_region: "CN".to_owned(),
        system_instruction: Some("Return JSON only with an answer string. Use only the supplied knowledge sources. Do not invent facts or source IDs. Add citations in the form [source:<knowledge_item_id>] when making factual claims. If the sources are insufficient, say so clearly.".to_owned()),
        output_schema: Some(json!({"type":"object","properties":{"answer":{"type":"string"}},"required":["answer"],"additionalProperties":false})),
        output_schema_name: Some("knowledge_answer".to_owned()),
        input: serde_json::to_string(&json!({"context": context, "question": query})).map_err(|_| ApiError::Internal)?,
        requested_output_tokens: 700,
        template_version: "knowledge-answer-v1".to_owned(),
        input_scope_reference: format!("knowledge-base:{}", base_id),
        redaction_policy_version: "knowledge-public-v1".to_owned(),
    };
    match gateway.execute(&request).await {
        AiExecutionResult::Completed { output, .. } => {
            #[derive(Deserialize)]
            struct ModelAnswer {
                answer: String,
            }
            match gateway.decode_json::<ModelAnswer>(&output) {
                Ok(value) if !value.answer.trim().is_empty() => Ok(KnowledgeChatResponse {
                    answer: value.answer,
                    certainty: "source_backed".to_owned(),
                    sources,
                    human_review_notice: review_notice().to_owned(),
                }),
                _ => Ok(KnowledgeChatResponse {
                    answer: deterministic,
                    certainty: "rule_based".to_owned(),
                    sources,
                    human_review_notice: review_notice().to_owned(),
                }),
            }
        }
        AiExecutionResult::Degraded { .. } | AiExecutionResult::Failed { .. } => {
            Ok(KnowledgeChatResponse {
                answer: deterministic,
                certainty: "rule_based".to_owned(),
                sources,
                human_review_notice: review_notice().to_owned(),
            })
        }
    }
}

pub async fn preview_csv(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    base_id: &str,
    file_name: String,
    bytes: Vec<u8>,
) -> Result<KnowledgeImportBatchResponse, ApiError> {
    require_admin(auth)?;
    let base = find_base(db, base_id).await?;
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(ApiError::Validation(
            "CSV file must not exceed 5 MB".to_owned(),
        ));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| ApiError::Validation("CSV must be UTF-8".to_owned()))?;
    let records = parse_csv_records(text)?;
    let headers = records.first().cloned().unwrap_or_default();
    if headers != CSV_HEADERS {
        return Err(ApiError::Validation(format!(
            "CSV header must be exactly: {}",
            CSV_HEADERS.join(",")
        )));
    }
    let timestamp = now();
    let batch_id = case_service::new_id();
    let transaction = db.begin().await?;
    knowledge_import_batches::ActiveModel {
        id: Set(batch_id.clone()),
        knowledge_base_id: Set(base_id.to_owned()),
        file_name: Set(file_name),
        status: Set("previewed".to_owned()),
        total_rows: Set(0),
        valid_rows: Set(0),
        invalid_rows: Set(0),
        created_by_user_id: Set(auth.id.clone()),
        confirmed_by_user_id: Set(None),
        created_at: Set(timestamp.clone()),
        confirmed_at: Set(None),
        updated_at: Set(timestamp.clone()),
    }
    .insert(&transaction)
    .await?;
    let mut rows = Vec::new();
    let mut seen_hashes = HashSet::new();
    for (index, record) in records.into_iter().skip(1).enumerate() {
        if index >= MAX_IMPORT_ROWS {
            return Err(ApiError::Validation(
                "CSV must not contain more than 5000 rows".to_owned(),
            ));
        }
        let row_number = (index + 2) as i32;
        if record.len() != CSV_HEADERS.len() {
            return Err(ApiError::Validation(format!(
                "CSV row {row_number} must contain exactly 9 columns"
            )));
        }
        let raw = record.clone();
        let raw_json = serde_json::to_string(&raw).map_err(|_| ApiError::Internal)?;
        let cell = |index: usize| record.get(index).map(String::as_str).unwrap_or("").trim();
        let base_value = cell(0);
        let title = cell(1);
        let content = cell(2);
        let summary = cell(3);
        let category = cell(4);
        let keywords = record
            .get(5)
            .map(String::as_str)
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let source_name = if cell(6).is_empty() {
            "CSV Import"
        } else {
            cell(6)
        };
        let source_url = cell(7);
        let visibility = if cell(8).is_empty() {
            base.visibility.as_str()
        } else {
            cell(8)
        };
        let validated = validated_item(
            CreateKnowledgeItemRequest {
                title: title.to_owned(),
                summary: summary.to_owned(),
                content: content.to_owned(),
                category: category.to_owned(),
                category_id: None,
                keywords: keywords.clone(),
                source_name: Some(source_name.to_owned()),
                source_url: (!source_url.is_empty()).then(|| source_url.to_owned()),
                visibility: visibility.to_owned(),
                images: Vec::new(),
            },
            &base.visibility,
        );
        let normalized = if let Ok(item) = validated.as_ref() {
            json!({
                "knowledge_base_id": base_value,
                "title": item.title,
                "content": item.content,
                "summary": item.summary,
                "category": item.category,
                "keywords": item.keywords,
                "source_name": item.source_name,
                "source_url": item.source_url,
                "visibility": item.visibility,
            })
        } else {
            json!({
                "knowledge_base_id": base_value,
                "title": title,
                "content": content,
                "summary": summary,
                "category": category,
                "keywords": keywords,
                "source_name": source_name,
                "source_url": if source_url.is_empty() { Value::Null } else { json!(source_url) },
                "visibility": visibility,
            })
        };
        let duplicate = if base_value != base_id {
            false
        } else if let Ok(item) = validated.as_ref() {
            let hash = content_hash(&item.title, &item.summary, &item.content, &item.keywords);
            !seen_hashes.insert(hash.clone())
                || knowledge_items::Entity::find()
                    .filter(knowledge_items::Column::KnowledgeBaseId.eq(base_id))
                    .filter(knowledge_items::Column::ContentHash.eq(hash))
                    .one(&transaction)
                    .await?
                    .is_some()
        } else {
            false
        };
        let validation = if base_value != base_id {
            Some("knowledge_base_id does not match the target knowledge base".to_owned())
        } else if let Err(error) = validated {
            Some(error.to_string())
        } else if duplicate {
            Some("duplicate content is not allowed".to_owned())
        } else {
            None
        };
        let status = if duplicate {
            "duplicate"
        } else if validation.is_some() {
            "invalid"
        } else {
            "valid"
        };
        knowledge_import_rows::ActiveModel {
            id: Set(case_service::new_id()),
            batch_id: Set(batch_id.clone()),
            row_number: Set(row_number),
            raw_data_json: Set(raw_json),
            normalized_data_json: Set(
                serde_json::to_string(&normalized).map_err(|_| ApiError::Internal)?
            ),
            status: Set(status.to_owned()),
            error_message: Set(validation),
            knowledge_item_id: Set(None),
            created_at: Set(timestamp.clone()),
        }
        .insert(&transaction)
        .await?;
        rows.push((row_number, status.to_owned()));
    }
    let valid_rows = rows.iter().filter(|(_, status)| status == "valid").count() as i32;
    let invalid_rows = rows.len() as i32 - valid_rows;
    knowledge_import_batches::Entity::update_many()
        .filter(knowledge_import_batches::Column::Id.eq(&batch_id))
        .col_expr(
            knowledge_import_batches::Column::TotalRows,
            Expr::value(rows.len() as i32),
        )
        .col_expr(
            knowledge_import_batches::Column::ValidRows,
            Expr::value(valid_rows),
        )
        .col_expr(
            knowledge_import_batches::Column::InvalidRows,
            Expr::value(invalid_rows),
        )
        .col_expr(
            knowledge_import_batches::Column::UpdatedAt,
            Expr::value(timestamp),
        )
        .exec(&transaction)
        .await?;
    transaction.commit().await?;
    get_import(db, auth, &batch_id).await
}

pub async fn get_import(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    batch_id: &str,
) -> Result<KnowledgeImportBatchResponse, ApiError> {
    require_admin(auth)?;
    let batch = knowledge_import_batches::Entity::find_by_id(batch_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("knowledge import batch was not found".to_owned()))?;
    let rows = knowledge_import_rows::Entity::find()
        .filter(knowledge_import_rows::Column::BatchId.eq(batch_id))
        .order_by_asc(knowledge_import_rows::Column::RowNumber)
        .all(db)
        .await?
        .into_iter()
        .map(|row| {
            Ok(KnowledgeImportRowResponse {
                id: row.id,
                row_number: row.row_number,
                status: row.status,
                error_message: row.error_message,
                normalized_data: serde_json::from_str(&row.normalized_data_json)
                    .map_err(|_| ApiError::Internal)?,
                knowledge_item_id: row.knowledge_item_id,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Ok(KnowledgeImportBatchResponse {
        id: batch.id,
        knowledge_base_id: batch.knowledge_base_id,
        file_name: batch.file_name,
        status: batch.status,
        total_rows: batch.total_rows,
        valid_rows: batch.valid_rows,
        invalid_rows: batch.invalid_rows,
        rows,
        confirmed_at: batch.confirmed_at,
    })
}

pub async fn confirm_import(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    batch_id: &str,
) -> Result<KnowledgeImportBatchResponse, ApiError> {
    require_admin(auth)?;
    let transaction = db.begin().await?;
    let batch = knowledge_import_batches::Entity::find_by_id(batch_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("knowledge import batch was not found".to_owned()))?;
    if batch.status == "confirmed" {
        transaction.rollback().await?;
        return get_import(db, auth, batch_id).await;
    }
    if batch.status != "previewed" {
        transaction.rollback().await?;
        return Err(ApiError::Conflict(
            "only previewed imports can be confirmed".to_owned(),
        ));
    }

    // Claim the batch before importing. The status condition makes competing
    // confirm/cancel requests observe one winner without importing twice.
    let timestamp = now();
    let transitioned = knowledge_import_batches::Entity::update_many()
        .filter(knowledge_import_batches::Column::Id.eq(batch_id))
        .filter(knowledge_import_batches::Column::Status.eq("previewed"))
        .col_expr(
            knowledge_import_batches::Column::Status,
            Expr::value("confirmed"),
        )
        .col_expr(
            knowledge_import_batches::Column::ConfirmedByUserId,
            Expr::value(auth.id.clone()),
        )
        .col_expr(
            knowledge_import_batches::Column::ConfirmedAt,
            Expr::value(timestamp.clone()),
        )
        .col_expr(
            knowledge_import_batches::Column::UpdatedAt,
            Expr::value(timestamp),
        )
        .exec(&transaction)
        .await?;
    if transitioned.rows_affected != 1 {
        transaction.rollback().await?;
        let current = knowledge_import_batches::Entity::find_by_id(batch_id)
            .one(db)
            .await?
            .ok_or_else(|| ApiError::NotFound("knowledge import batch was not found".to_owned()))?;
        if current.status == "confirmed" {
            return get_import(db, auth, batch_id).await;
        }
        return Err(ApiError::Conflict(
            "only previewed imports can be confirmed".to_owned(),
        ));
    }

    let rows = knowledge_import_rows::Entity::find()
        .filter(knowledge_import_rows::Column::BatchId.eq(batch_id))
        .filter(knowledge_import_rows::Column::Status.eq("valid"))
        .all(&transaction)
        .await?;
    for row in rows {
        let value: Value =
            serde_json::from_str(&row.normalized_data_json).map_err(|_| ApiError::Internal)?;
        let title = value["title"].as_str().unwrap_or_default().to_owned();
        let summary = value["summary"].as_str().unwrap_or_default().to_owned();
        let content = value["content"].as_str().unwrap_or_default().to_owned();
        let keywords: Vec<String> =
            serde_json::from_value(value["keywords"].clone()).map_err(|_| ApiError::Internal)?;
        let hash = content_hash(&title, &summary, &content, &keywords);
        if knowledge_items::Entity::find()
            .filter(knowledge_items::Column::KnowledgeBaseId.eq(&batch.knowledge_base_id))
            .filter(knowledge_items::Column::ContentHash.eq(&hash))
            .one(&transaction)
            .await?
            .is_some()
        {
            knowledge_import_rows::Entity::update_many()
                .filter(knowledge_import_rows::Column::Id.eq(&row.id))
                .col_expr(
                    knowledge_import_rows::Column::Status,
                    Expr::value("duplicate"),
                )
                .exec(&transaction)
                .await?;
            continue;
        }
        let timestamp = now();
        let item_id = case_service::new_id();
        knowledge_items::ActiveModel {
            id: Set(item_id.clone()),
            knowledge_base_id: Set(batch.knowledge_base_id.clone()),
            title: Set(title),
            summary: Set(summary),
            content: Set(content),
            category: Set(value["category"].as_str().unwrap_or_default().to_owned()),
            category_id: Set(None),
            keywords_json: Set(serde_json::to_string(&keywords).map_err(|_| ApiError::Internal)?),
            metadata_json: Set("{}".to_owned()),
            source_name: Set(value["source_name"]
                .as_str()
                .unwrap_or("CSV Import")
                .to_owned()),
            source_url: Set(value["source_url"].as_str().map(str::to_owned)),
            visibility: Set(value["visibility"].as_str().unwrap_or("learner").to_owned()),
            status: Set("draft".to_owned()),
            effective_at: Set(timestamp.clone()),
            withdrawn_at: Set(None),
            previous_version_id: Set(None),
            version: Set(1),
            content_hash: Set(hash),
            embedding_json: Set(None),
            embedding_model: Set(None),
            embedding_dimension: Set(None),
            embedding_status: Set("none".to_owned()),
            embedding_generated_at: Set(None),
            embedding_content_hash: Set(None),
            created_by_user_id: Set(auth.id.clone()),
            created_at: Set(timestamp.clone()),
            updated_at: Set(timestamp),
        }
        .insert(&transaction)
        .await?;
        knowledge_import_rows::Entity::update_many()
            .filter(knowledge_import_rows::Column::Id.eq(&row.id))
            .col_expr(
                knowledge_import_rows::Column::Status,
                Expr::value("imported"),
            )
            .col_expr(
                knowledge_import_rows::Column::KnowledgeItemId,
                Expr::value(item_id),
            )
            .exec(&transaction)
            .await?;
    }
    transaction.commit().await?;
    get_import(db, auth, batch_id).await
}
pub async fn cancel_import(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    batch_id: &str,
) -> Result<KnowledgeImportBatchResponse, ApiError> {
    require_admin(auth)?;
    let transaction = db.begin().await?;
    let batch = knowledge_import_batches::Entity::find_by_id(batch_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("knowledge import batch was not found".to_owned()))?;
    if batch.status == "confirmed" {
        transaction.rollback().await?;
        return Err(ApiError::Conflict(
            "confirmed imports cannot be cancelled".to_owned(),
        ));
    }
    if batch.status != "previewed" {
        transaction.rollback().await?;
        return Err(ApiError::Conflict(
            "only previewed imports can be cancelled".to_owned(),
        ));
    }
    let transitioned = knowledge_import_batches::Entity::update_many()
        .filter(knowledge_import_batches::Column::Id.eq(batch_id))
        .filter(knowledge_import_batches::Column::Status.eq("previewed"))
        .col_expr(
            knowledge_import_batches::Column::Status,
            Expr::value("cancelled"),
        )
        .col_expr(
            knowledge_import_batches::Column::UpdatedAt,
            Expr::value(now()),
        )
        .exec(&transaction)
        .await?;
    if transitioned.rows_affected != 1 {
        transaction.rollback().await?;
        let current = knowledge_import_batches::Entity::find_by_id(batch_id)
            .one(db)
            .await?
            .ok_or_else(|| ApiError::NotFound("knowledge import batch was not found".to_owned()))?;
        let message = if current.status == "confirmed" {
            "confirmed imports cannot be cancelled"
        } else {
            "only previewed imports can be cancelled"
        };
        return Err(ApiError::Conflict(message.to_owned()));
    }
    transaction.commit().await?;
    get_import(db, auth, batch_id).await
}
async fn results_with_images(
    db: &DatabaseConnection,
    items: Vec<knowledge_items::Model>,
    scores: Option<&HashMap<String, f64>>,
) -> Result<Vec<KnowledgeSearchResultResponse>, ApiError> {
    let ids: Vec<_> = items.iter().map(|item| item.id.clone()).collect();
    let images = if ids.is_empty() {
        Vec::new()
    } else {
        knowledge_images::Entity::find()
            .filter(knowledge_images::Column::KnowledgeItemId.is_in(ids))
            .all(db)
            .await?
    };
    let mut images_by_item: HashMap<String, Vec<KnowledgeImageResponse>> = HashMap::new();
    for image in images {
        images_by_item
            .entry(image.knowledge_item_id.clone())
            .or_default()
            .push(image_response(image)?);
    }
    Ok(items
        .into_iter()
        .map(|item| KnowledgeSearchResultResponse {
            knowledge_item_id: item.id.clone(),
            title: item.title,
            content: item.content,
            score: scores
                .and_then(|value| value.get(&item.id).copied())
                .unwrap_or(0.0),
            knowledge_base_id: item.knowledge_base_id,
            version: item.version,
            source_name: item.source_name,
            source_url: item.source_url,
            status: item.status,
            images: images_by_item.remove(&item.id).unwrap_or_default(),
        })
        .collect())
}

async fn find_base(db: &DatabaseConnection, id: &str) -> Result<knowledge_bases::Model, ApiError> {
    knowledge_bases::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("knowledge base was not found".to_owned()))
}
async fn find_item(db: &DatabaseConnection, id: &str) -> Result<knowledge_items::Model, ApiError> {
    knowledge_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("knowledge item was not found".to_owned()))
}
fn base_response(model: knowledge_bases::Model) -> Result<KnowledgeBaseResponse, ApiError> {
    Ok(KnowledgeBaseResponse {
        id: model.id,
        name: model.name,
        description: model.description,
        visibility: model.visibility,
        status: model.status,
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}
fn image_response(image: knowledge_images::Model) -> Result<KnowledgeImageResponse, ApiError> {
    Ok(KnowledgeImageResponse {
        id: image.id,
        storage_path: image.storage_path,
        mime_type: image.mime_type,
        width: image.width,
        height: image.height,
        metadata: serde_json::from_str(&image.metadata_json).map_err(|_| ApiError::Internal)?,
    })
}
pub fn require_admin(auth: &AuthenticatedUser) -> Result<(), ApiError> {
    auth.global_capabilities
        .contains(&GlobalCapability::Admin)
        .then_some(())
        .ok_or_else(|| ApiError::Forbidden("administrator capability required".to_owned()))
}
fn visible_to(auth: &AuthenticatedUser, visibility: &str) -> bool {
    match visibility {
        "public" | "authenticated" => true,
        "volunteer" => {
            auth.global_capabilities
                .contains(&GlobalCapability::Volunteer)
                || auth.global_capabilities.contains(&GlobalCapability::Admin)
        }
        "learner" => {
            auth.account_type == AccountType::Learner
                || auth.global_capabilities.contains(&GlobalCapability::Admin)
        }
        _ => false,
    }
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_csv_records(input: &str) -> Result<Vec<Vec<String>>, ApiError> {
    let mut records = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = input.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                row.push(field.trim().to_owned());
                field.clear();
            }
            '\n' if !quoted => {
                row.push(field.trim().to_owned());
                field.clear();
                let record = std::mem::take(&mut row);
                if record.iter().any(|value| !value.is_empty()) {
                    records.push(record);
                }
            }
            '\r' if !quoted => {}
            _ => field.push(character),
        }
    }
    if quoted {
        return Err(ApiError::Validation(
            "CSV contains an unterminated quoted field".to_owned(),
        ));
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field.trim().to_owned());
        if row.iter().any(|value| !value.is_empty()) {
            records.push(row);
        }
    }
    if records.is_empty() {
        return Err(ApiError::Validation(
            "CSV must contain a header row".to_owned(),
        ));
    }
    Ok(records)
}
fn required(value: &str, field: &str, max: usize) -> Result<String, ApiError> {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if value.is_empty() || value.chars().count() > max {
        Err(ApiError::Validation(format!(
            "{field} must contain between 1 and {max} characters"
        )))
    } else {
        Ok(value)
    }
}
fn optional(value: &str, max: usize) -> Result<String, ApiError> {
    let value = value.trim();
    if value.chars().count() > max {
        Err(ApiError::Validation(format!(
            "field must not exceed {max} characters"
        )))
    } else {
        Ok(value.to_owned())
    }
}
fn visibility(value: &str) -> Result<String, ApiError> {
    let value = value.trim().to_lowercase();
    ["public", "authenticated", "volunteer", "learner"]
        .contains(&value.as_str())
        .then_some(value)
        .ok_or_else(|| ApiError::Validation("visibility is invalid".to_owned()))
}
fn validate_query(value: &str) -> Result<&str, ApiError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > MAX_QUERY_LENGTH {
        Err(ApiError::Validation(
            "query must contain between 1 and 1000 characters".to_owned(),
        ))
    } else {
        Ok(value)
    }
}
fn validate_limit(value: Option<u32>) -> Result<u32, ApiError> {
    let value = value.unwrap_or(DEFAULT_LIMIT);
    if (1..=MAX_LIMIT).contains(&value) {
        Ok(value)
    } else {
        Err(ApiError::Validation(
            "limit must be between 1 and 20".to_owned(),
        ))
    }
}
fn review_notice() -> &'static str {
    "Knowledge answers are based only on published materials. Field action remains subject to human review and responsible-person instructions."
}
fn content_hash(title: &str, summary: &str, content: &str, keywords: &[String]) -> String {
    let mut hash = Sha256::new();
    hash.update(title.as_bytes());
    hash.update([0]);
    hash.update(summary.as_bytes());
    hash.update([0]);
    hash.update(content.as_bytes());
    hash.update([0]);
    for keyword in keywords {
        hash.update(keyword.as_bytes());
        hash.update([0]);
    }
    hex::encode(hash.finalize())
}
fn score_item(item: &knowledge_items::Model, query: &str) -> Option<f64> {
    let terms = search_terms(query);
    let keywords: Vec<String> = serde_json::from_str(&item.keywords_json).unwrap_or_default();
    let title = item.title.to_lowercase();
    let category = item.category.to_lowercase();
    let summary = item.summary.to_lowercase();
    let content = item.content.to_lowercase();
    let mut score = 0.0;
    for term in terms {
        if title.contains(&term) {
            score += 8.0;
        }
        if category.contains(&term)
            || keywords
                .iter()
                .any(|word| word.to_lowercase().contains(&term))
        {
            score += 4.0;
        }
        if summary.contains(&term) {
            score += 2.0;
        }
        if content.contains(&term) {
            score += 1.0;
        }
    }
    (score > 0.0).then_some(score)
}
fn search_terms(query: &str) -> Vec<String> {
    let mut terms: Vec<String> = query
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect();
    let chinese: Vec<char> = query
        .chars()
        .filter(|character| ('\u{4e00}'..='\u{9fff}').contains(character))
        .collect();
    terms.extend(chinese.windows(2).map(|window| window.iter().collect()));
    terms.sort();
    terms.dedup();
    terms
}

struct ValidatedItem {
    title: String,
    summary: String,
    content: String,
    category: String,
    category_id: Option<String>,
    keywords: Vec<String>,
    source_name: String,
    source_url: Option<String>,
    visibility: String,
    images: Vec<KnowledgeImageInput>,
}
fn validated_item(
    request: CreateKnowledgeItemRequest,
    default_visibility: &str,
) -> Result<ValidatedItem, ApiError> {
    let mut unique = HashSet::new();
    let keywords = request
        .keywords
        .iter()
        .map(|value| required(value, "keyword", 64))
        .collect::<Result<Vec<_>, _>>()?;
    if keywords.len() > 20
        || !keywords
            .iter()
            .all(|value| unique.insert(value.to_lowercase()))
    {
        return Err(ApiError::Validation(
            "keywords must be unique and contain at most 20 values".to_owned(),
        ));
    }
    if request.images.len() > 12 {
        return Err(ApiError::Validation(
            "an item may contain at most 12 images".to_owned(),
        ));
    }
    let mut paths = HashSet::new();
    for image in &request.images {
        if !paths.insert(image.storage_path.trim().to_owned())
            || !image.storage_path.starts_with("/")
            || image.storage_path.contains("..")
            || !image.mime_type.starts_with("image/")
            || image.width.is_some_and(|value| value <= 0)
            || image.height.is_some_and(|value| value <= 0)
            || !image.metadata.is_object()
        {
            return Err(ApiError::Validation("image metadata is invalid".to_owned()));
        }
    }
    Ok(ValidatedItem {
        title: required(&request.title, "title", 240)?,
        summary: optional(&request.summary, 2_000)?,
        content: required(&request.content, "content", 50_000)?,
        category: optional(&request.category, 160)?,
        category_id: request
            .category_id
            .map(|value| required(&value, "category_id", 191))
            .transpose()?,
        keywords,
        source_name: request
            .source_name
            .map(|value| required(&value, "source_name", 240))
            .transpose()?
            .unwrap_or_else(|| "Manual entry".to_owned()),
        source_url: request
            .source_url
            .filter(|value| !value.trim().is_empty())
            .map(|value| {
                if value.starts_with("https://") && value.chars().count() <= 2_000 {
                    Ok(value)
                } else {
                    Err(ApiError::Validation(
                        "source_url must be an HTTPS URL".to_owned(),
                    ))
                }
            })
            .transpose()?,
        visibility: if request.visibility.trim().is_empty() {
            default_visibility.to_owned()
        } else {
            visibility(&request.visibility)?
        },
        images: request.images,
    })
}

struct MergedItem {
    title: String,
    summary: String,
    content: String,
    category: String,
    category_id: Option<String>,
    keywords: Vec<String>,
    source_name: String,
    source_url: Option<String>,
    visibility: String,
    images: Option<Vec<KnowledgeImageInput>>,
}

fn merge_item(
    existing: knowledge_items::Model,
    request: UpdateKnowledgeItemRequest,
    default_visibility: &str,
) -> Result<MergedItem, ApiError> {
    let keywords = request
        .keywords
        .unwrap_or_else(|| serde_json::from_str(&existing.keywords_json).unwrap_or_default());
    let images = request.images;
    let input = CreateKnowledgeItemRequest {
        title: request.title.unwrap_or(existing.title),
        summary: request.summary.unwrap_or(existing.summary),
        content: request.content.unwrap_or(existing.content),
        category: request.category.unwrap_or(existing.category),
        category_id: request.category_id.or(existing.category_id),
        keywords,
        source_name: request.source_name.or(Some(existing.source_name)),
        source_url: request.source_url.or(existing.source_url),
        visibility: request.visibility.unwrap_or(existing.visibility),
        images: images.clone().unwrap_or_default(),
    };
    let validated = validated_item(input, default_visibility)?;
    Ok(MergedItem {
        title: validated.title,
        summary: validated.summary,
        content: validated.content,
        category: validated.category,
        category_id: validated.category_id,
        keywords: validated.keywords,
        source_name: validated.source_name,
        source_url: validated.source_url,
        visibility: validated.visibility,
        images,
    })
}
async fn ensure_unique_images<C: sea_orm::ConnectionTrait>(
    db: &C,
    images: &[KnowledgeImageInput],
) -> Result<(), ApiError> {
    let paths: Vec<_> = images
        .iter()
        .map(|image| image.storage_path.trim().to_owned())
        .collect();
    if !paths.is_empty()
        && knowledge_images::Entity::find()
            .filter(knowledge_images::Column::StoragePath.is_in(paths))
            .one(db)
            .await?
            .is_some()
    {
        return Err(ApiError::Conflict(
            "an image path is already bound to another knowledge item".to_owned(),
        ));
    }
    Ok(())
}

async fn ensure_unique_images_except<C: sea_orm::ConnectionTrait>(
    db: &C,
    images: &[KnowledgeImageInput],
    item_id: &str,
) -> Result<(), ApiError> {
    let paths: Vec<_> = images
        .iter()
        .map(|image| image.storage_path.trim().to_owned())
        .collect();
    if !paths.is_empty()
        && knowledge_images::Entity::find()
            .filter(knowledge_images::Column::StoragePath.is_in(paths))
            .filter(knowledge_images::Column::KnowledgeItemId.ne(item_id))
            .one(db)
            .await?
            .is_some()
    {
        return Err(ApiError::Conflict(
            "an image path is already bound to another knowledge item".to_owned(),
        ));
    }
    Ok(())
}
async fn insert_images<C: sea_orm::ConnectionTrait>(
    db: &C,
    item_id: &str,
    images: Vec<KnowledgeImageInput>,
    timestamp: &str,
) -> Result<(), ApiError> {
    for image in images {
        knowledge_images::ActiveModel {
            id: Set(case_service::new_id()),
            knowledge_item_id: Set(item_id.to_owned()),
            storage_path: Set(image.storage_path.trim().to_owned()),
            mime_type: Set(image.mime_type.trim().to_owned()),
            width: Set(image.width),
            height: Set(image.height),
            metadata_json: Set(
                serde_json::to_string(&image.metadata).map_err(|_| ApiError::Internal)?
            ),
            created_at: Set(timestamp.to_owned()),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

pub fn build_context(results: &[KnowledgeSearchResultResponse]) -> String {
    results
        .iter()
        .enumerate()
        .map(|(index, result)| {
            format!(
                "[Knowledge Source {}]\nid: {}\ntitle: {}\ncontent: {}\nsource: {}",
                index + 1,
                result.knowledge_item_id,
                result.title,
                result.content,
                result.source_name
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_builder_never_invents_source_ids() {
        let result = KnowledgeSearchResultResponse {
            knowledge_item_id: "item-1".to_owned(),
            title: "Title".to_owned(),
            content: "Body".to_owned(),
            score: 1.0,
            knowledge_base_id: "base".to_owned(),
            version: 1,
            source_name: "Source".to_owned(),
            source_url: None,
            images: Vec::new(),
            status: "published".to_owned(),
        };
        let context = build_context(&[result]);
        assert!(context.contains("id: item-1"));
        assert!(context.contains("[Knowledge Source 1]"));
    }

    #[test]
    fn csv_parser_skips_blank_records() {
        let records = parse_csv_records("first,second\n\n  ,  \nvalue,content\n\n").unwrap();

        assert_eq!(
            records,
            vec![
                vec!["first".to_owned(), "second".to_owned()],
                vec!["value".to_owned(), "content".to_owned()],
            ]
        );
    }
}
