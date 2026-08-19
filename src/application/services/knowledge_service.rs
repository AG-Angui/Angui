use std::collections::{HashMap, HashSet};

use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use sha2::{Digest, Sha256};

use crate::{
    entities::{knowledge_bases, knowledge_images, knowledge_items},
    error::ApiError,
    models::{
        AuthenticatedUser, CreateKnowledgeBaseRequest, CreateKnowledgeItemRequest,
        KnowledgeBaseResponse, KnowledgeChatResponse, KnowledgeChatSourceResponse,
        KnowledgeImageInput, KnowledgeImageResponse, KnowledgeSearchResponse,
        KnowledgeSearchResultResponse, UpdateKnowledgeBaseRequest,
    },
    roles::{AccountType, GlobalCapability},
    services::case_service,
};

const MAX_QUERY_LENGTH: usize = 1_000;
const DEFAULT_LIMIT: u32 = 5;
const MAX_LIMIT: u32 = 20;

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
fn require_admin(auth: &AuthenticatedUser) -> Result<(), ApiError> {
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
        };
        let context = build_context(&[result]);
        assert!(context.contains("id: item-1"));
        assert!(context.contains("[Knowledge Source 1]"));
    }
}
