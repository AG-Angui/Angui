use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde_json::json;

use crate::{
    entities::{learning_question_answers, learning_questions, learning_resources},
    error::ApiError,
    models::{
        AuthenticatedUser, KnowledgeAnswerResponse, KnowledgeAskRequest, LearningAnswerSource,
        LearningQuestionQuery, LearningQuestionResponse, LearningResourceQuery,
        LearningResourceResponse, SubmitLearningAnswerRequest, SubmitLearningAnswerResponse,
    },
    roles::{AccountType, GlobalCapability},
    services::case_service,
};

const MAX_QUESTION_LENGTH: usize = 1_000;

pub async fn list_resources(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    query: LearningResourceQuery,
) -> Result<Vec<LearningResourceResponse>, ApiError> {
    let now = now();
    let resources = learning_resources::Entity::find()
        .filter(learning_resources::Column::Status.eq("published"))
        .filter(learning_resources::Column::EffectiveAt.lte(now))
        .order_by_asc(learning_resources::Column::ResourceType)
        .order_by_asc(learning_resources::Column::Title)
        .all(db)
        .await?;
    resources
        .into_iter()
        .filter(|resource| visible_to(auth, &resource.visibility))
        .filter(|resource| {
            query.resource_type.as_ref().is_none_or(|resource_type| {
                resource.resource_type == resource_type.trim().to_lowercase()
            })
        })
        .filter(|resource| {
            query.tag.as_ref().is_none_or(|tag| {
                parse_string_array(&resource.tags_json).is_ok_and(|tags| {
                    tags.iter()
                        .any(|value| value.eq_ignore_ascii_case(tag.trim()))
                })
            })
        })
        .map(resource_response)
        .collect()
}

pub async fn list_questions(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    query: LearningQuestionQuery,
) -> Result<Vec<LearningQuestionResponse>, ApiError> {
    let now = now();
    let questions = learning_questions::Entity::find()
        .filter(learning_questions::Column::Status.eq("published"))
        .filter(learning_questions::Column::EffectiveAt.lte(now))
        .order_by_asc(learning_questions::Column::CreatedAt)
        .all(db)
        .await?;
    questions
        .into_iter()
        .filter(|question| visible_to(auth, &question.visibility))
        .filter(|question| {
            query
                .difficulty
                .as_ref()
                .is_none_or(|difficulty| question.difficulty == difficulty.trim().to_lowercase())
        })
        .filter(|question| {
            query.tag.as_ref().is_none_or(|tag| {
                parse_string_array(&question.tags_json).is_ok_and(|tags| {
                    tags.iter()
                        .any(|value| value.eq_ignore_ascii_case(tag.trim()))
                })
            })
        })
        .map(question_response)
        .collect()
}

pub async fn submit_answer(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    question_id: &str,
    request: SubmitLearningAnswerRequest,
) -> Result<SubmitLearningAnswerResponse, ApiError> {
    let question = visible_question(db, auth, question_id).await?;
    let selected_option_id = request.selected_option_id.trim();
    let options = parse_options(&question.options_json)?;
    if selected_option_id.is_empty()
        || selected_option_id.chars().count() > 128
        || !options.iter().any(|option| option.id == selected_option_id)
    {
        return Err(ApiError::Validation(
            "selected_option_id is not an option on this question".to_owned(),
        ));
    }

    let source = learning_resources::Entity::find_by_id(&question.source_resource_id)
        .one(db)
        .await?
        .filter(|resource| resource.status == "published" && visible_to(auth, &resource.visibility))
        .ok_or_else(|| ApiError::NotFound("learning question was not found".to_owned()))?;
    let is_correct = selected_option_id == question.correct_option_id;
    let transaction = db.begin().await?;
    learning_question_answers::ActiveModel {
        id: Set(case_service::new_id()),
        question_id: Set(question.id.clone()),
        user_id: Set(auth.id.clone()),
        selected_option_id: Set(selected_option_id.to_owned()),
        is_correct: Set(is_correct),
        question_version: Set(question.version),
        created_at: Set(now()),
    }
    .insert(&transaction)
    .await?;
    case_service::write_audit(
        &transaction,
        None,
        auth,
        "learning_question.answered",
        "learning_question",
        question.id.clone(),
        Some(json!({ "question_version": question.version, "is_correct": is_correct })),
    )
    .await?;
    transaction.commit().await?;

    Ok(SubmitLearningAnswerResponse {
        question_id: question.id,
        is_correct,
        explanation: question.explanation,
        source: source_reference(&source),
    })
}

pub async fn ask_knowledge(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    request: KnowledgeAskRequest,
) -> Result<KnowledgeAnswerResponse, ApiError> {
    let prompt = request.question.trim();
    if prompt.is_empty() || prompt.chars().count() > MAX_QUESTION_LENGTH {
        return Err(ApiError::Validation(
            "question must contain between 1 and 1000 characters".to_owned(),
        ));
    }
    let keywords = knowledge_keywords(prompt);
    let resources = list_resources(
        db,
        auth,
        LearningResourceQuery {
            resource_type: None,
            tag: None,
        },
    )
    .await?;
    let matches: Vec<_> = resources
        .into_iter()
        .filter(|resource| {
            let haystack = format!(
                "{} {} {} {}",
                resource.title,
                resource.summary,
                resource.content,
                resource.tags.join(" ")
            )
            .to_lowercase();
            keywords.iter().any(|word| haystack.contains(word))
        })
        .take(2)
        .collect();
    if matches.is_empty() {
        return Ok(KnowledgeAnswerResponse {
            answer: "没有可支持该问题的已审核学习资料。请联系负责人或查阅经审核的手册，不能据此形成行动结论。".to_owned(),
            certainty: "insufficient_sources".to_owned(),
            sources: Vec::new(),
            human_review_notice: "学习问答仅提供资料定位；现场行动以人工审核记录和负责人指令为准。".to_owned(),
        });
    }
    Ok(KnowledgeAnswerResponse {
        answer: matches
            .iter()
            .map(|resource| resource.content.clone())
            .collect::<Vec<_>>()
            .join("\n\n"),
        certainty: "source_backed".to_owned(),
        sources: matches
            .iter()
            .map(|resource| LearningAnswerSource {
                resource_id: resource.id.clone(),
                title: resource.title.clone(),
                version: resource.version,
            })
            .collect(),
        human_review_notice: "学习问答仅提供资料定位；现场行动以人工审核记录和负责人指令为准。"
            .to_owned(),
    })
}

async fn visible_question(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    question_id: &str,
) -> Result<learning_questions::Model, ApiError> {
    let question = learning_questions::Entity::find_by_id(question_id)
        .one(db)
        .await?
        .filter(|question| {
            question.status == "published"
                && question.effective_at <= now()
                && visible_to(auth, &question.visibility)
        })
        .ok_or_else(|| ApiError::NotFound("learning question was not found".to_owned()))?;
    Ok(question)
}

fn visible_to(auth: &AuthenticatedUser, visibility: &str) -> bool {
    match visibility {
        "public" | "authenticated" => true,
        "learner" => auth.account_type == AccountType::Learner,
        "volunteer" => auth
            .global_capabilities
            .contains(&GlobalCapability::Volunteer),
        _ => false,
    }
}

fn resource_response(
    resource: learning_resources::Model,
) -> Result<LearningResourceResponse, ApiError> {
    Ok(LearningResourceResponse {
        id: resource.id,
        title: resource.title,
        summary: resource.summary,
        content: resource.content,
        resource_type: resource.resource_type,
        tags: parse_string_array(&resource.tags_json)?,
        source_name: resource.source_name,
        source_url: resource.source_url,
        version: resource.version,
        effective_at: resource.effective_at,
    })
}

fn question_response(
    question: learning_questions::Model,
) -> Result<LearningQuestionResponse, ApiError> {
    Ok(LearningQuestionResponse {
        id: question.id,
        prompt: question.prompt,
        question_type: question.question_type,
        difficulty: question.difficulty,
        tags: parse_string_array(&question.tags_json)?,
        options: serde_json::from_str(&question.options_json).map_err(|_| ApiError::Internal)?,
        source_resource_id: question.source_resource_id,
        version: question.version,
    })
}

fn source_reference(resource: &learning_resources::Model) -> LearningAnswerSource {
    LearningAnswerSource {
        resource_id: resource.id.clone(),
        title: resource.title.clone(),
        version: resource.version,
    }
}

#[derive(serde::Deserialize)]
struct LearningOption {
    id: String,
}

fn parse_options(value: &str) -> Result<Vec<LearningOption>, ApiError> {
    let options: Vec<LearningOption> =
        serde_json::from_str(value).map_err(|_| ApiError::Internal)?;
    if options.is_empty() || options.iter().any(|option| option.id.trim().is_empty()) {
        return Err(ApiError::Internal);
    }
    Ok(options)
}

fn parse_string_array(value: &str) -> Result<Vec<String>, ApiError> {
    serde_json::from_str(value).map_err(|_| ApiError::Internal)
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn knowledge_keywords(prompt: &str) -> Vec<String> {
    let mut keywords: Vec<String> = prompt
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.chars().count() > 1)
        .map(str::to_lowercase)
        .collect();

    let chinese: Vec<char> = prompt
        .chars()
        .filter(|character| ('\u{4e00}'..='\u{9fff}').contains(character))
        .collect();
    keywords.extend(
        chinese
            .windows(2)
            .map(|window| window.iter().collect::<String>()),
    );
    keywords.sort();
    keywords.dedup();
    keywords
}

#[cfg(test)]
mod tests {
    use super::knowledge_keywords;

    #[test]
    fn chinese_questions_produce_searchable_bigrams() {
        let keywords = knowledge_keywords("如何防止走失？");

        assert!(keywords.iter().any(|keyword| keyword == "防止"));
        assert!(keywords.iter().any(|keyword| keyword == "走失"));
    }
}
