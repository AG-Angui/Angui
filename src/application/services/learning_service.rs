use std::collections::{HashMap, HashSet};

use chrono::{DateTime, SecondsFormat, Utc};
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, Set, TransactionTrait, TryInsertResult,
};
use serde_json::json;

use crate::{
    entities::{
        learning_categories, learning_category_review_events, learning_content_review_events,
        learning_question_answers, learning_questions, learning_resources,
    },
    error::ApiError,
    models::{
        AuthenticatedUser, CreateLearningCategoryRequest, CreateLearningQuestionRequest,
        CreateLearningResourceRequest, KnowledgeAnswerResponse, KnowledgeAskRequest,
        LearningAnswerSource, LearningCategoryResponse, LearningContentActionRequest,
        LearningContentLifecycleResponse, LearningContentReviewEventResponse,
        LearningQuestionQuery, LearningQuestionResponse, LearningResourceQuery,
        LearningResourceResponse, ManagedLearningCategoryResponse, ManagedLearningQuestionResponse,
        ManagedLearningResourceResponse, SubmitLearningAnswerRequest, SubmitLearningAnswerResponse,
    },
    roles::{AccountType, GlobalCapability},
    services::case_service,
};

const MAX_QUESTION_LENGTH: usize = 1_000;

/// Returns the approved public prevention card that can be retained for offline use.
/// This endpoint deliberately excludes all account-scoped learning resources.
pub async fn public_prevention_card(
    db: &DatabaseConnection,
) -> Result<LearningResourceResponse, ApiError> {
    let now = now();
    let resources = learning_resources::Entity::find()
        .filter(learning_resources::Column::Status.eq("published"))
        .filter(learning_resources::Column::ResourceType.eq("prevention"))
        .filter(learning_resources::Column::Visibility.eq("public"))
        .filter(learning_resources::Column::EffectiveAt.lte(now))
        .order_by_desc(learning_resources::Column::UpdatedAt)
        .all(db)
        .await?;
    let states = lifecycle_states(
        db,
        "resource",
        resources.iter().map(|resource| resource.id.as_str()),
    )
    .await?;

    resources
        .into_iter()
        .find(|resource| {
            states
                .get(&resource.id)
                .is_some_and(ContentLifecycle::is_training_published)
        })
        .map(resource_response)
        .transpose()?
        .ok_or_else(|| ApiError::NotFound("公开防走失知识卡尚未发布".to_owned()))
}

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
    let states = lifecycle_states(
        db,
        "resource",
        resources.iter().map(|resource| resource.id.as_str()),
    )
    .await?;
    resources
        .into_iter()
        .filter(|resource| {
            states
                .get(&resource.id)
                .is_some_and(ContentLifecycle::is_training_published)
        })
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
        .filter(|resource| {
            query.category_id.as_ref().is_none_or(|category_id| {
                resource.category_id.as_deref() == Some(category_id.trim())
            })
        })
        .map(resource_response)
        .collect()
}

/// Lists only categories that may be selected for a new resource. Historical
/// resources retain their category snapshot even after a category is disabled.
pub async fn list_enabled_categories(
    db: &DatabaseConnection,
    _auth: &AuthenticatedUser,
) -> Result<Vec<LearningCategoryResponse>, ApiError> {
    learning_categories::Entity::find()
        .filter(learning_categories::Column::Status.eq("enabled"))
        .order_by_asc(learning_categories::Column::Name)
        .all(db)
        .await?
        .into_iter()
        .map(category_response)
        .collect()
}

pub async fn list_managed_categories(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
) -> Result<Vec<ManagedLearningCategoryResponse>, ApiError> {
    require_admin(auth)?;
    learning_categories::Entity::find()
        .order_by_asc(learning_categories::Column::Name)
        .all(db)
        .await?
        .into_iter()
        .map(|category| {
            Ok(ManagedLearningCategoryResponse {
                category: category_response(category.clone())?,
                submitted_by_user_id: category.submitted_by_user_id,
                reviewed_by_user_id: category.reviewed_by_user_id,
                created_at: category.created_at,
                updated_at: category.updated_at,
            })
        })
        .collect()
}

pub async fn propose_category(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    request: CreateLearningCategoryRequest,
) -> Result<LearningCategoryResponse, ApiError> {
    require_learner(auth)?;
    let name = normalized_name(&request.name, "category", 160)?;
    let reason = required_text(&request.submission_reason, "submission_reason", 1_000)?;
    let id = case_service::new_id();
    let timestamp = now();
    let transaction = db.begin().await?;
    let inserted = learning_categories::Entity::insert(learning_categories::ActiveModel {
        id: Set(id.clone()),
        name: Set(name.clone()),
        normalized_name: Set(normalized_key(&name)),
        status: Set("pending".to_owned()),
        submitted_by_user_id: Set(auth.id.clone()),
        reviewed_by_user_id: Set(None),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp.clone()),
    })
    .on_conflict(
        OnConflict::column(learning_categories::Column::NormalizedName)
            .do_nothing()
            .to_owned(),
    )
    .do_nothing()
    .exec(&transaction)
    .await?;
    if !matches!(inserted, TryInsertResult::Inserted(_)) {
        return Err(ApiError::Conflict(
            "category already exists or is awaiting review".to_owned(),
        ));
    }
    append_category_event(&transaction, auth, &id, "submitted", &reason).await?;
    write_learning_audit(
        &transaction,
        auth,
        "learning_category.submitted",
        &id,
        0,
        "training",
    )
    .await?;
    transaction.commit().await?;
    Ok(LearningCategoryResponse {
        id,
        name,
        status: "pending".to_owned(),
    })
}

pub async fn transition_category(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    category_id: &str,
    action: &str,
    request: LearningContentActionRequest,
) -> Result<LearningCategoryResponse, ApiError> {
    require_admin(auth)?;
    let reason = required_text(&request.reason, "reason", 1_000)?;
    let category = learning_categories::Entity::find_by_id(category_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("learning category does not exist".to_owned()))?;
    let permitted = matches!(
        (category.status.as_str(), action),
        ("pending", "enable") | ("pending", "reject") | ("enabled", "disable")
    );
    if !permitted {
        return Err(ApiError::Conflict(
            "category lifecycle transition is not allowed".to_owned(),
        ));
    }
    let status = match action {
        "enable" => "enabled",
        "reject" => "rejected",
        "disable" => "disabled",
        _ => return Err(ApiError::Validation("unknown category action".to_owned())),
    };
    let transaction = db.begin().await?;
    let mut update = category.into_active_model();
    update.status = Set(status.to_owned());
    update.reviewed_by_user_id = Set(Some(auth.id.clone()));
    update.updated_at = Set(now());
    let category = update.update(&transaction).await?;
    append_category_event(&transaction, auth, &category.id, status, &reason).await?;
    write_learning_audit(
        &transaction,
        auth,
        &format!("learning_category.{status}"),
        &category.id,
        0,
        "training",
    )
    .await?;
    transaction.commit().await?;
    category_response(category)
}

pub async fn list_questions(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    query: LearningQuestionQuery,
) -> Result<Vec<LearningQuestionResponse>, ApiError> {
    let now = now();
    let questions = learning_questions::Entity::find()
        .filter(learning_questions::Column::Status.eq("published"))
        .filter(learning_questions::Column::EffectiveAt.lte(now.clone()))
        .order_by_asc(learning_questions::Column::CreatedAt)
        .all(db)
        .await?;
    let question_states = lifecycle_states(
        db,
        "question",
        questions.iter().map(|question| question.id.as_str()),
    )
    .await?;
    let source_ids: Vec<_> = questions
        .iter()
        .map(|question| question.source_resource_id.clone())
        .collect();
    let sources = learning_resources::Entity::find()
        .filter(learning_resources::Column::Id.is_in(source_ids))
        .all(db)
        .await?;
    let source_states = lifecycle_states(
        db,
        "resource",
        sources.iter().map(|resource| resource.id.as_str()),
    )
    .await?;
    let sources: HashMap<_, _> = sources
        .into_iter()
        .map(|resource| (resource.id.clone(), resource))
        .collect();
    questions
        .into_iter()
        .filter(|question| {
            question_states
                .get(&question.id)
                .is_some_and(ContentLifecycle::is_training_published)
                && sources
                    .get(&question.source_resource_id)
                    .is_some_and(|source| {
                        source.status == "published"
                            && source.effective_at <= now
                            && visible_to(auth, &source.visibility)
                            && source_states
                                .get(&source.id)
                                .is_some_and(ContentLifecycle::is_training_published)
                    })
        })
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
        .filter(|resource| {
            resource.status == "published"
                && resource.effective_at <= now()
                && visible_to(auth, &resource.visibility)
        })
        .ok_or_else(|| ApiError::NotFound("learning question was not found".to_owned()))?;
    if !content_lifecycle(db, "resource", &source.id, source.version)
        .await?
        .is_training_published()
    {
        return Err(ApiError::NotFound(
            "learning question was not found".to_owned(),
        ));
    }
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
            category_id: None,
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

pub async fn list_managed_resources(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
) -> Result<Vec<ManagedLearningResourceResponse>, ApiError> {
    require_admin(auth)?;
    let resources = learning_resources::Entity::find()
        .order_by_desc(learning_resources::Column::UpdatedAt)
        .all(db)
        .await?;
    let states = lifecycle_states(
        db,
        "resource",
        resources.iter().map(|resource| resource.id.as_str()),
    )
    .await?;
    resources
        .into_iter()
        .map(|resource| {
            let lifecycle = states
                .get(&resource.id)
                .cloned()
                .unwrap_or_default()
                .response()?;
            Ok(ManagedLearningResourceResponse {
                resource: resource_response(resource)?,
                lifecycle,
            })
        })
        .collect()
}

pub async fn create_resource(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    request: CreateLearningResourceRequest,
) -> Result<ManagedLearningResourceResponse, ApiError> {
    require_admin(auth)?;
    create_resource_inner(db, auth, request, false).await
}

/// A learner can contribute a draft, but its audience and permitted use are
/// deliberately fixed. It still enters the same independent governance chain
/// as administrator-authored content and is never readable on submission.
pub async fn submit_resource_draft(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    request: CreateLearningResourceRequest,
) -> Result<ManagedLearningResourceResponse, ApiError> {
    require_learner(auth)?;
    create_resource_inner(db, auth, request, true).await
}

async fn create_resource_inner(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    request: CreateLearningResourceRequest,
    learner_submission: bool,
) -> Result<ManagedLearningResourceResponse, ApiError> {
    let title = required_text(&request.title, "title", 160)?;
    let summary = required_text(&request.summary, "summary", 800)?;
    let content = required_text(&request.content, "content", 20_000)?;
    let resource_type = enum_value(
        &request.resource_type,
        "resource_type",
        &["team_intro", "manual", "prevention", "case_study"],
    )?;
    let visibility = enum_value(
        &request.visibility,
        "visibility",
        &["public", "authenticated", "volunteer", "learner"],
    )?;
    let permitted_use = enum_value(
        &request.permitted_use,
        "permitted_use",
        &["training", "public_information"],
    )?;
    if learner_submission && (visibility != "learner" || permitted_use != "training") {
        return Err(ApiError::Forbidden(
            "learner drafts must use learner visibility and training-only use".to_owned(),
        ));
    }
    let source_name = required_text(&request.source_name, "source_name", 240)?;
    let source_url = validate_source_url(request.source_url)?;
    let effective_at = valid_timestamp(&request.effective_at, "effective_at")?;
    let submission_reason = required_text(&request.submission_reason, "submission_reason", 1_000)?;
    let tags_json =
        serde_json::to_string(&normalized_tags(&request.tags)?).map_err(|_| ApiError::Internal)?;
    let (category_id, category_name) =
        enabled_category_assignment(db, request.category_id.as_deref()).await?;
    let (previous_version_id, version) =
        resource_revision(db, request.previous_version_id.as_deref()).await?;
    let timestamp = now();
    let id = case_service::new_id();
    let transaction = db.begin().await?;
    let insert = learning_resources::Entity::insert(learning_resources::ActiveModel {
        id: Set(id.clone()),
        title: Set(title),
        summary: Set(summary),
        content: Set(content),
        resource_type: Set(resource_type),
        tags_json: Set(tags_json),
        category_id: Set(category_id),
        category_name: Set(category_name),
        source_name: Set(source_name),
        source_url: Set(source_url),
        previous_version_id: Set(previous_version_id),
        version: Set(version),
        visibility: Set(visibility),
        // Drafts are deliberately non-readable by legacy queries too.
        status: Set("withdrawn".to_owned()),
        effective_at: Set(effective_at),
        withdrawn_at: Set(None),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp.clone()),
    })
    .on_conflict(
        OnConflict::column(learning_resources::Column::PreviousVersionId)
            .do_nothing()
            .to_owned(),
    )
    .do_nothing()
    .exec(&transaction)
    .await?;
    if !matches!(insert, TryInsertResult::Inserted(_)) {
        return Err(ApiError::Conflict(
            "该学习资源已被其他管理员更正，请刷新后重试".to_owned(),
        ));
    }
    append_lifecycle_event(
        &transaction,
        auth,
        LifecycleEventInput::new(
            "resource",
            &id,
            version,
            "submitted",
            &submission_reason,
            &permitted_use,
        ),
    )
    .await?;
    write_learning_audit(
        &transaction,
        auth,
        "learning_resource.submitted",
        &id,
        version,
        &permitted_use,
    )
    .await?;
    transaction.commit().await?;
    managed_resource(db, &id).await
}

pub async fn deidentify_resource(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    resource_id: &str,
    request: LearningContentActionRequest,
) -> Result<ManagedLearningResourceResponse, ApiError> {
    transition_resource(db, auth, resource_id, "deidentified", request).await
}

pub async fn review_resource(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    resource_id: &str,
    request: LearningContentActionRequest,
) -> Result<ManagedLearningResourceResponse, ApiError> {
    transition_resource(db, auth, resource_id, "reviewed", request).await
}

pub async fn publish_resource(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    resource_id: &str,
    request: LearningContentActionRequest,
) -> Result<ManagedLearningResourceResponse, ApiError> {
    transition_resource(db, auth, resource_id, "published", request).await
}

pub async fn withdraw_resource(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    resource_id: &str,
    request: LearningContentActionRequest,
) -> Result<ManagedLearningResourceResponse, ApiError> {
    transition_resource(db, auth, resource_id, "withdrawn", request).await
}

pub async fn list_managed_questions(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
) -> Result<Vec<ManagedLearningQuestionResponse>, ApiError> {
    require_admin(auth)?;
    let questions = learning_questions::Entity::find()
        .order_by_desc(learning_questions::Column::UpdatedAt)
        .all(db)
        .await?;
    let states = lifecycle_states(
        db,
        "question",
        questions.iter().map(|question| question.id.as_str()),
    )
    .await?;
    questions
        .into_iter()
        .map(|question| {
            let lifecycle = states
                .get(&question.id)
                .cloned()
                .unwrap_or_default()
                .response()?;
            Ok(ManagedLearningQuestionResponse {
                question: question_response(question)?,
                lifecycle,
            })
        })
        .collect()
}

pub async fn create_question(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    request: CreateLearningQuestionRequest,
) -> Result<ManagedLearningQuestionResponse, ApiError> {
    require_admin(auth)?;
    let source_resource_id = required_text(&request.source_resource_id, "source_resource_id", 64)?;
    learning_resources::Entity::find_by_id(&source_resource_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::Validation("source_resource_id does not exist".to_owned()))?;
    let prompt = required_text(&request.prompt, "prompt", 2_000)?;
    let question_type = enum_value(
        &request.question_type,
        "question_type",
        &["single_choice", "true_false", "scenario"],
    )?;
    let difficulty = enum_value(
        &request.difficulty,
        "difficulty",
        &["basic", "intermediate", "advanced"],
    )?;
    let visibility = enum_value(
        &request.visibility,
        "visibility",
        &["authenticated", "volunteer", "learner"],
    )?;
    let permitted_use = enum_value(&request.permitted_use, "permitted_use", &["training"])?;
    let options = validated_options(request.options)?;
    if !options
        .iter()
        .any(|option| option.id == request.correct_option_id.trim())
    {
        return Err(ApiError::Validation(
            "correct_option_id must reference an option".to_owned(),
        ));
    }
    let explanation = required_text(&request.explanation, "explanation", 4_000)?;
    let effective_at = valid_timestamp(&request.effective_at, "effective_at")?;
    let submission_reason = required_text(&request.submission_reason, "submission_reason", 1_000)?;
    let tags_json =
        serde_json::to_string(&normalized_tags(&request.tags)?).map_err(|_| ApiError::Internal)?;
    let options_json = serde_json::to_string(&options).map_err(|_| ApiError::Internal)?;
    let (previous_version_id, version) = question_revision(
        db,
        request.previous_version_id.as_deref(),
        &source_resource_id,
    )
    .await?;
    let timestamp = now();
    let id = case_service::new_id();
    let transaction = db.begin().await?;
    let insert = learning_questions::Entity::insert(learning_questions::ActiveModel {
        id: Set(id.clone()),
        source_resource_id: Set(source_resource_id),
        prompt: Set(prompt),
        question_type: Set(question_type),
        difficulty: Set(difficulty),
        tags_json: Set(tags_json),
        options_json: Set(options_json),
        correct_option_id: Set(request.correct_option_id.trim().to_owned()),
        explanation: Set(explanation),
        previous_version_id: Set(previous_version_id),
        version: Set(version),
        visibility: Set(visibility),
        status: Set("withdrawn".to_owned()),
        effective_at: Set(effective_at),
        withdrawn_at: Set(None),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp.clone()),
    })
    .on_conflict(
        OnConflict::column(learning_questions::Column::PreviousVersionId)
            .do_nothing()
            .to_owned(),
    )
    .do_nothing()
    .exec(&transaction)
    .await?;
    if !matches!(insert, TryInsertResult::Inserted(_)) {
        return Err(ApiError::Conflict(
            "该学习题目已被其他管理员更正，请刷新后重试".to_owned(),
        ));
    }
    append_lifecycle_event(
        &transaction,
        auth,
        LifecycleEventInput::new(
            "question",
            &id,
            version,
            "submitted",
            &submission_reason,
            &permitted_use,
        ),
    )
    .await?;
    write_learning_audit(
        &transaction,
        auth,
        "learning_question.submitted",
        &id,
        version,
        &permitted_use,
    )
    .await?;
    transaction.commit().await?;
    managed_question(db, &id).await
}

pub async fn deidentify_question(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    question_id: &str,
    request: LearningContentActionRequest,
) -> Result<ManagedLearningQuestionResponse, ApiError> {
    transition_question(db, auth, question_id, "deidentified", request).await
}

pub async fn review_question(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    question_id: &str,
    request: LearningContentActionRequest,
) -> Result<ManagedLearningQuestionResponse, ApiError> {
    transition_question(db, auth, question_id, "reviewed", request).await
}

pub async fn publish_question(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    question_id: &str,
    request: LearningContentActionRequest,
) -> Result<ManagedLearningQuestionResponse, ApiError> {
    transition_question(db, auth, question_id, "published", request).await
}

pub async fn withdraw_question(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    question_id: &str,
    request: LearningContentActionRequest,
) -> Result<ManagedLearningQuestionResponse, ApiError> {
    transition_question(db, auth, question_id, "withdrawn", request).await
}

async fn transition_resource(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    resource_id: &str,
    event_type: &str,
    request: LearningContentActionRequest,
) -> Result<ManagedLearningResourceResponse, ApiError> {
    require_admin(auth)?;
    let resource = learning_resources::Entity::find_by_id(resource_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("学习资源不存在".to_owned()))?;
    transition_content(
        db,
        auth,
        "resource",
        &resource.id,
        resource.version,
        event_type,
        request,
    )
    .await?;
    managed_resource(db, resource_id).await
}

async fn transition_question(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    question_id: &str,
    event_type: &str,
    request: LearningContentActionRequest,
) -> Result<ManagedLearningQuestionResponse, ApiError> {
    require_admin(auth)?;
    let question = learning_questions::Entity::find_by_id(question_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("学习题目不存在".to_owned()))?;
    if event_type == "published" {
        let source = learning_resources::Entity::find_by_id(&question.source_resource_id)
            .one(db)
            .await?
            .ok_or_else(|| ApiError::Validation("题目来源资源不存在".to_owned()))?;
        if source.status != "published"
            || source.effective_at > now()
            || !content_lifecycle(db, "resource", &source.id, source.version)
                .await?
                .is_training_published()
        {
            return Err(ApiError::Conflict(
                "题目只能在其已审核、已发布的培训来源有效时发布".to_owned(),
            ));
        }
    }
    transition_content(
        db,
        auth,
        "question",
        &question.id,
        question.version,
        event_type,
        request,
    )
    .await?;
    managed_question(db, question_id).await
}

async fn transition_content(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    content_type: &str,
    content_id: &str,
    content_version: i32,
    event_type: &str,
    request: LearningContentActionRequest,
) -> Result<(), ApiError> {
    let reason = required_text(&request.reason, "reason", 1_000)?;
    let transaction = db.begin().await?;
    let state = content_lifecycle(&transaction, content_type, content_id, content_version).await?;
    let is_correction = match content_type {
        "resource" => learning_resources::Entity::find_by_id(content_id)
            .one(&transaction)
            .await?
            .ok_or(ApiError::Internal)?
            .previous_version_id
            .is_some(),
        "question" => learning_questions::Entity::find_by_id(content_id)
            .one(&transaction)
            .await?
            .ok_or(ApiError::Internal)?
            .previous_version_id
            .is_some(),
        _ => return Err(ApiError::Internal),
    };
    validate_transition(&state, auth, event_type, is_correction)?;
    append_lifecycle_event(
        &transaction,
        auth,
        LifecycleEventInput::new(
            content_type,
            content_id,
            content_version,
            event_type,
            &reason,
            &state.permitted_use,
        ),
    )
    .await?;
    match (content_type, event_type) {
        ("resource", "published") => {
            let resource = learning_resources::Entity::find_by_id(content_id)
                .one(&transaction)
                .await?
                .ok_or(ApiError::Internal)?;
            learning_resources::Entity::update_many()
                .col_expr(learning_resources::Column::Status, Expr::value("published"))
                .col_expr(
                    learning_resources::Column::WithdrawnAt,
                    Expr::value(None::<String>),
                )
                .col_expr(learning_resources::Column::UpdatedAt, Expr::value(now()))
                .filter(learning_resources::Column::Id.eq(content_id))
                .exec(&transaction)
                .await?;
            if let Some(previous_version_id) = resource.previous_version_id {
                let previous = learning_resources::Entity::find_by_id(&previous_version_id)
                    .one(&transaction)
                    .await?
                    .ok_or(ApiError::Internal)?;
                if previous.status != "withdrawn" {
                    learning_resources::Entity::update_many()
                        .col_expr(learning_resources::Column::Status, Expr::value("withdrawn"))
                        .col_expr(
                            learning_resources::Column::WithdrawnAt,
                            Expr::value(Some(now())),
                        )
                        .col_expr(learning_resources::Column::UpdatedAt, Expr::value(now()))
                        .filter(learning_resources::Column::Id.eq(&previous_version_id))
                        .exec(&transaction)
                        .await?;
                    append_lifecycle_event(
                        &transaction,
                        auth,
                        LifecycleEventInput::new(
                            "resource",
                            &previous.id,
                            previous.version,
                            "withdrawn",
                            "已由审核发布的更正版本替代",
                            &state.permitted_use,
                        ),
                    )
                    .await?;
                }
            }
        }
        ("resource", "withdrawn") => {
            let update = learning_resources::Entity::update_many()
                .col_expr(learning_resources::Column::Status, Expr::value("withdrawn"))
                .col_expr(
                    learning_resources::Column::WithdrawnAt,
                    Expr::value(Some(now())),
                )
                .col_expr(learning_resources::Column::UpdatedAt, Expr::value(now()))
                .filter(learning_resources::Column::Id.eq(content_id));
            if is_correction && state.state() != "published" {
                update
                    .col_expr(
                        learning_resources::Column::PreviousVersionId,
                        Expr::value(None::<String>),
                    )
                    .exec(&transaction)
                    .await?;
            } else {
                update.exec(&transaction).await?;
            }
        }
        ("question", "published") => {
            let question = learning_questions::Entity::find_by_id(content_id)
                .one(&transaction)
                .await?
                .ok_or(ApiError::Internal)?;
            learning_questions::Entity::update_many()
                .col_expr(learning_questions::Column::Status, Expr::value("published"))
                .col_expr(
                    learning_questions::Column::WithdrawnAt,
                    Expr::value(None::<String>),
                )
                .col_expr(learning_questions::Column::UpdatedAt, Expr::value(now()))
                .filter(learning_questions::Column::Id.eq(content_id))
                .exec(&transaction)
                .await?;
            if let Some(previous_version_id) = question.previous_version_id {
                let previous = learning_questions::Entity::find_by_id(&previous_version_id)
                    .one(&transaction)
                    .await?
                    .ok_or(ApiError::Internal)?;
                if previous.status != "withdrawn" {
                    learning_questions::Entity::update_many()
                        .col_expr(learning_questions::Column::Status, Expr::value("withdrawn"))
                        .col_expr(
                            learning_questions::Column::WithdrawnAt,
                            Expr::value(Some(now())),
                        )
                        .col_expr(learning_questions::Column::UpdatedAt, Expr::value(now()))
                        .filter(learning_questions::Column::Id.eq(&previous_version_id))
                        .exec(&transaction)
                        .await?;
                    append_lifecycle_event(
                        &transaction,
                        auth,
                        LifecycleEventInput::new(
                            "question",
                            &previous.id,
                            previous.version,
                            "withdrawn",
                            "已由审核发布的更正版本替代",
                            &state.permitted_use,
                        ),
                    )
                    .await?;
                }
            }
        }
        ("question", "withdrawn") => {
            let update = learning_questions::Entity::update_many()
                .col_expr(learning_questions::Column::Status, Expr::value("withdrawn"))
                .col_expr(
                    learning_questions::Column::WithdrawnAt,
                    Expr::value(Some(now())),
                )
                .col_expr(learning_questions::Column::UpdatedAt, Expr::value(now()))
                .filter(learning_questions::Column::Id.eq(content_id));
            if is_correction && state.state() != "published" {
                update
                    .col_expr(
                        learning_questions::Column::PreviousVersionId,
                        Expr::value(None::<String>),
                    )
                    .exec(&transaction)
                    .await?;
            } else {
                update.exec(&transaction).await?;
            }
        }
        _ => {}
    }
    write_learning_audit(
        &transaction,
        auth,
        &format!("learning_{content_type}.{event_type}"),
        content_id,
        content_version,
        &state.permitted_use,
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn managed_resource(
    db: &DatabaseConnection,
    resource_id: &str,
) -> Result<ManagedLearningResourceResponse, ApiError> {
    let resource = learning_resources::Entity::find_by_id(resource_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("学习资源不存在".to_owned()))?;
    Ok(ManagedLearningResourceResponse {
        lifecycle: content_lifecycle(db, "resource", &resource.id, resource.version)
            .await?
            .response()?,
        resource: resource_response(resource)?,
    })
}

pub async fn export_resource(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    resource_id: &str,
) -> Result<LearningResourceResponse, ApiError> {
    require_admin(auth)?;
    let resource = learning_resources::Entity::find_by_id(resource_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("学习资源不存在".to_owned()))?;
    if resource.status != "published"
        || !content_lifecycle(db, "resource", &resource.id, resource.version)
            .await?
            .is_training_published()
    {
        return Err(ApiError::NotFound("学习资源不存在".to_owned()));
    }
    resource_response(resource)
}

async fn managed_question(
    db: &DatabaseConnection,
    question_id: &str,
) -> Result<ManagedLearningQuestionResponse, ApiError> {
    let question = learning_questions::Entity::find_by_id(question_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("学习题目不存在".to_owned()))?;
    Ok(ManagedLearningQuestionResponse {
        lifecycle: content_lifecycle(db, "question", &question.id, question.version)
            .await?
            .response()?,
        question: question_response(question)?,
    })
}

pub async fn export_question(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    question_id: &str,
) -> Result<LearningQuestionResponse, ApiError> {
    require_admin(auth)?;
    let question = learning_questions::Entity::find_by_id(question_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("学习题目不存在".to_owned()))?;
    if question.status != "published"
        || !content_lifecycle(db, "question", &question.id, question.version)
            .await?
            .is_training_published()
    {
        return Err(ApiError::NotFound("学习题目不存在".to_owned()));
    }
    let source = learning_resources::Entity::find_by_id(&question.source_resource_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("学习题目的来源资源不存在".to_owned()))?;
    if source.status != "published"
        || !content_lifecycle(db, "resource", &source.id, source.version)
            .await?
            .is_training_published()
    {
        return Err(ApiError::NotFound("学习题目不存在".to_owned()));
    }
    question_response(question)
}

struct LifecycleEventInput<'a> {
    content_type: &'a str,
    content_id: &'a str,
    content_version: i32,
    event_type: &'a str,
    reason: &'a str,
    permitted_use: &'a str,
}

impl<'a> LifecycleEventInput<'a> {
    fn new(
        content_type: &'a str,
        content_id: &'a str,
        content_version: i32,
        event_type: &'a str,
        reason: &'a str,
        permitted_use: &'a str,
    ) -> Self {
        Self {
            content_type,
            content_id,
            content_version,
            event_type,
            reason,
            permitted_use,
        }
    }
}

async fn append_lifecycle_event(
    transaction: &sea_orm::DatabaseTransaction,
    auth: &AuthenticatedUser,
    event: LifecycleEventInput<'_>,
) -> Result<(), ApiError> {
    let result = learning_content_review_events::Entity::insert_many([
        learning_content_review_events::ActiveModel {
            id: Set(case_service::new_id()),
            content_type: Set(event.content_type.to_owned()),
            content_id: Set(event.content_id.to_owned()),
            content_version: Set(event.content_version),
            event_type: Set(event.event_type.to_owned()),
            actor_user_id: Set(auth.id.clone()),
            reason: Set(event.reason.to_owned()),
            permitted_use: Set(event.permitted_use.to_owned()),
            created_at: Set(now()),
        },
    ])
    .on_conflict(
        OnConflict::columns([
            learning_content_review_events::Column::ContentType,
            learning_content_review_events::Column::ContentId,
            learning_content_review_events::Column::ContentVersion,
            learning_content_review_events::Column::EventType,
        ])
        .do_nothing()
        .to_owned(),
    )
    .do_nothing()
    .exec(transaction)
    .await?;
    if matches!(result, TryInsertResult::Inserted(_)) {
        Ok(())
    } else {
        Err(ApiError::Conflict(
            "学习内容状态已被其他管理员更新，请刷新后重试".to_owned(),
        ))
    }
}

async fn write_learning_audit(
    transaction: &sea_orm::DatabaseTransaction,
    auth: &AuthenticatedUser,
    action: &str,
    content_id: &str,
    content_version: i32,
    permitted_use: &str,
) -> Result<(), ApiError> {
    case_service::write_audit(
        transaction,
        None,
        auth,
        action,
        "learning_content",
        content_id.to_owned(),
        Some(json!({ "version": content_version, "permitted_use": permitted_use })),
    )
    .await
}

async fn append_category_event(
    transaction: &sea_orm::DatabaseTransaction,
    auth: &AuthenticatedUser,
    category_id: &str,
    event_type: &str,
    reason: &str,
) -> Result<(), ApiError> {
    learning_category_review_events::Entity::insert(learning_category_review_events::ActiveModel {
        id: Set(case_service::new_id()),
        category_id: Set(category_id.to_owned()),
        event_type: Set(event_type.to_owned()),
        actor_user_id: Set(auth.id.clone()),
        reason: Set(reason.to_owned()),
        created_at: Set(now()),
    })
    .exec(transaction)
    .await?;
    Ok(())
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
    if !content_lifecycle(db, "question", &question.id, question.version)
        .await?
        .is_training_published()
    {
        return Err(ApiError::NotFound(
            "learning question was not found".to_owned(),
        ));
    }
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

async fn resource_revision(
    db: &DatabaseConnection,
    previous_version_id: Option<&str>,
) -> Result<(Option<String>, i32), ApiError> {
    let Some(previous_version_id) = previous_version_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok((None, 1));
    };
    let previous = learning_resources::Entity::find_by_id(previous_version_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("要更正的学习资源不存在".to_owned()))?;
    if previous.status != "published"
        || !content_lifecycle(db, "resource", &previous.id, previous.version)
            .await?
            .is_training_published()
    {
        return Err(ApiError::Conflict(
            "只能更正已审核发布的学习资源".to_owned(),
        ));
    }
    Ok((Some(previous.id), previous.version + 1))
}

async fn enabled_category_assignment(
    db: &DatabaseConnection,
    category_id: Option<&str>,
) -> Result<(Option<String>, Option<String>), ApiError> {
    let Some(category_id) = category_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return Ok((None, None));
    };
    let category = learning_categories::Entity::find_by_id(category_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::Validation("category_id does not exist".to_owned()))?;
    if category.status != "enabled" {
        return Err(ApiError::Validation(
            "category_id is not enabled".to_owned(),
        ));
    }
    Ok((Some(category.id), Some(category.name)))
}

async fn question_revision(
    db: &DatabaseConnection,
    previous_version_id: Option<&str>,
    source_resource_id: &str,
) -> Result<(Option<String>, i32), ApiError> {
    let Some(previous_version_id) = previous_version_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok((None, 1));
    };
    let previous = learning_questions::Entity::find_by_id(previous_version_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("要更正的学习题目不存在".to_owned()))?;
    if previous.status != "published"
        || !content_lifecycle(db, "question", &previous.id, previous.version)
            .await?
            .is_training_published()
    {
        return Err(ApiError::Conflict(
            "只能更正已审核发布的学习题目".to_owned(),
        ));
    }
    if previous.source_resource_id != source_resource_id {
        return Err(ApiError::Conflict(
            "更正题目必须保留原版本的来源资源".to_owned(),
        ));
    }
    Ok((Some(previous.id), previous.version + 1))
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
        category: match (resource.category_id, resource.category_name) {
            (Some(id), Some(name)) => Some(LearningCategoryResponse {
                id,
                name,
                // This is a historical resource snapshot. Its category may now
                // be disabled, but that must not hide a previously published
                // learning record.
                status: "assigned".to_owned(),
            }),
            _ => None,
        },
        source_name: resource.source_name,
        source_url: resource.source_url,
        previous_version_id: resource.previous_version_id,
        version: resource.version,
        effective_at: resource.effective_at,
    })
}

fn category_response(
    category: learning_categories::Model,
) -> Result<LearningCategoryResponse, ApiError> {
    Ok(LearningCategoryResponse {
        id: category.id,
        name: category.name,
        status: category.status,
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
        previous_version_id: question.previous_version_id,
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

#[derive(serde::Deserialize, serde::Serialize)]
struct LearningOption {
    id: String,
    text: String,
}

fn parse_options(value: &str) -> Result<Vec<LearningOption>, ApiError> {
    let options: Vec<LearningOption> =
        serde_json::from_str(value).map_err(|_| ApiError::Internal)?;
    if options.is_empty()
        || options.len() > 12
        || options.iter().any(|option| {
            option.id.trim().is_empty()
                || option.id.chars().count() > 128
                || option.text.trim().is_empty()
                || option.text.chars().count() > 800
        })
        || options
            .iter()
            .map(|option| option.id.trim())
            .collect::<HashSet<_>>()
            .len()
            != options.len()
    {
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

#[derive(Clone, Default)]
struct ContentLifecycle {
    submitted_by_user_id: Option<String>,
    deidentified_by_user_id: Option<String>,
    reviewed_by_user_id: Option<String>,
    published_by_user_id: Option<String>,
    withdrawn_by_user_id: Option<String>,
    permitted_use: String,
    events: Vec<learning_content_review_events::Model>,
}

impl ContentLifecycle {
    fn state(&self) -> &'static str {
        if self.withdrawn_by_user_id.is_some() {
            "withdrawn"
        } else if self.published_by_user_id.is_some() {
            "published"
        } else if self.reviewed_by_user_id.is_some() {
            "reviewed"
        } else if self.deidentified_by_user_id.is_some() {
            "deidentified"
        } else if self.submitted_by_user_id.is_some() {
            "submitted"
        } else {
            "unmanaged"
        }
    }

    fn is_training_published(&self) -> bool {
        self.state() == "published"
            && matches!(
                self.permitted_use.as_str(),
                "training" | "public_information"
            )
    }

    fn response(self) -> Result<LearningContentLifecycleResponse, ApiError> {
        let state = self.state().to_owned();
        Ok(LearningContentLifecycleResponse {
            submitted_by_user_id: self.submitted_by_user_id.unwrap_or_default(),
            deidentified_by_user_id: self.deidentified_by_user_id,
            reviewed_by_user_id: self.reviewed_by_user_id,
            published_by_user_id: self.published_by_user_id,
            withdrawn_by_user_id: self.withdrawn_by_user_id,
            state,
            permitted_use: self.permitted_use,
            events: self
                .events
                .into_iter()
                .map(|event| LearningContentReviewEventResponse {
                    event_type: event.event_type,
                    actor_user_id: event.actor_user_id,
                    reason: event.reason,
                    created_at: event.created_at,
                })
                .collect(),
        })
    }
}

async fn lifecycle_states<'a>(
    db: &DatabaseConnection,
    content_type: &str,
    content_ids: impl Iterator<Item = &'a str>,
) -> Result<HashMap<String, ContentLifecycle>, ApiError> {
    let content_ids: Vec<_> = content_ids.map(str::to_owned).collect();
    if content_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let events = learning_content_review_events::Entity::find()
        .filter(learning_content_review_events::Column::ContentType.eq(content_type))
        .filter(learning_content_review_events::Column::ContentId.is_in(content_ids))
        .order_by_asc(learning_content_review_events::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(events
        .into_iter()
        .fold(HashMap::new(), |mut states, event| {
            apply_lifecycle_event(states.entry(event.content_id.clone()).or_default(), event);
            states
        }))
}

async fn content_lifecycle<C: sea_orm::ConnectionTrait>(
    db: &C,
    content_type: &str,
    content_id: &str,
    content_version: i32,
) -> Result<ContentLifecycle, ApiError> {
    let events = learning_content_review_events::Entity::find()
        .filter(learning_content_review_events::Column::ContentType.eq(content_type))
        .filter(learning_content_review_events::Column::ContentId.eq(content_id))
        .filter(learning_content_review_events::Column::ContentVersion.eq(content_version))
        .order_by_asc(learning_content_review_events::Column::CreatedAt)
        .all(db)
        .await?;
    let mut state = ContentLifecycle::default();
    for event in events {
        apply_lifecycle_event(&mut state, event);
    }
    Ok(state)
}

fn apply_lifecycle_event(
    state: &mut ContentLifecycle,
    event: learning_content_review_events::Model,
) {
    match event.event_type.as_str() {
        "submitted" => {
            state.submitted_by_user_id = Some(event.actor_user_id.clone());
            state.deidentified_by_user_id = None;
            state.reviewed_by_user_id = None;
            state.published_by_user_id = None;
            state.withdrawn_by_user_id = None;
            state.permitted_use = event.permitted_use.clone();
        }
        "deidentified" => state.deidentified_by_user_id = Some(event.actor_user_id.clone()),
        "reviewed" => state.reviewed_by_user_id = Some(event.actor_user_id.clone()),
        "published" => {
            state.published_by_user_id = Some(event.actor_user_id.clone());
            state.withdrawn_by_user_id = None;
        }
        "withdrawn" => state.withdrawn_by_user_id = Some(event.actor_user_id.clone()),
        "rejected" => {
            state.deidentified_by_user_id = None;
            state.reviewed_by_user_id = None;
            state.published_by_user_id = None;
            state.withdrawn_by_user_id = Some(event.actor_user_id.clone());
        }
        _ => {}
    }
    state.events.push(event);
}

fn validate_transition(
    state: &ContentLifecycle,
    auth: &AuthenticatedUser,
    event_type: &str,
    is_correction: bool,
) -> Result<(), ApiError> {
    let submitted_by = state
        .submitted_by_user_id
        .as_deref()
        .ok_or_else(|| ApiError::Conflict("内容尚未提交治理流程".to_owned()))?;
    match event_type {
        "deidentified" if state.state() == "submitted" && submitted_by != auth.id => Ok(()),
        "reviewed" if state.state() == "deidentified" && submitted_by != auth.id => Ok(()),
        "published" if state.state() == "reviewed" => Ok(()),
        "withdrawn" if state.state() == "published" => Ok(()),
        "withdrawn"
            if is_correction
                && matches!(state.state(), "submitted" | "deidentified" | "reviewed") =>
        {
            Ok(())
        }
        "deidentified" | "reviewed" => Err(ApiError::Conflict(
            "脱敏和审核必须由非提交人按顺序完成".to_owned(),
        )),
        "published" => Err(ApiError::Conflict(
            "内容必须先完成独立脱敏和审核才能发布".to_owned(),
        )),
        "withdrawn" => Err(ApiError::Conflict("只有已发布内容可以撤回".to_owned())),
        _ => Err(ApiError::Validation("未知的内容治理操作".to_owned())),
    }
}

fn require_admin(auth: &AuthenticatedUser) -> Result<(), ApiError> {
    auth.global_capabilities
        .contains(&GlobalCapability::Admin)
        .then_some(())
        .ok_or_else(|| ApiError::Forbidden("只有管理员可以管理学习内容".to_owned()))
}

fn require_learner(auth: &AuthenticatedUser) -> Result<(), ApiError> {
    (auth.account_type == AccountType::Learner)
        .then_some(())
        .ok_or_else(|| {
            ApiError::Forbidden("only learner accounts may submit learning drafts".to_owned())
        })
}

fn required_text(value: &str, field: &str, maximum: usize) -> Result<String, ApiError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > maximum {
        return Err(ApiError::Validation(format!(
            "{field} 必须为 1 到 {maximum} 个字符"
        )));
    }
    Ok(value.to_owned())
}

fn enum_value(value: &str, field: &str, allowed: &[&str]) -> Result<String, ApiError> {
    let value = value.trim().to_lowercase();
    if allowed.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(ApiError::Validation(format!("{field} 取值无效")))
    }
}

fn normalized_tags(tags: &[String]) -> Result<Vec<String>, ApiError> {
    if tags.len() > 12 {
        return Err(ApiError::Validation("标签最多 12 个".to_owned()));
    }
    let tags: Vec<_> = tags
        .iter()
        .map(|tag| normalized_name(tag, "tag", 64))
        .collect::<Result<_, _>>()?;
    if tags
        .iter()
        .map(|tag| normalized_key(tag))
        .collect::<HashSet<_>>()
        .len()
        != tags.len()
    {
        return Err(ApiError::Validation("标签不能重复".to_owned()));
    }
    Ok(tags)
}

fn normalized_name(value: &str, field: &str, maximum: usize) -> Result<String, ApiError> {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    required_text(&value, field, maximum)
}

fn normalized_key(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn valid_timestamp(value: &str, field: &str) -> Result<String, ApiError> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|timestamp| timestamp.to_rfc3339_opts(SecondsFormat::Millis, true))
        .map_err(|_| ApiError::Validation(format!("{field} 必须是 RFC 3339 时间")))
}

fn validate_source_url(value: Option<String>) -> Result<Option<String>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > 2_000 || !value.starts_with("https://") {
        return Err(ApiError::Validation(
            "source_url 必须是 HTTPS 地址".to_owned(),
        ));
    }
    Ok(Some(value.to_owned()))
}

fn validated_options(value: serde_json::Value) -> Result<Vec<LearningOption>, ApiError> {
    let serialized = serde_json::to_string(&value)
        .map_err(|_| ApiError::Validation("题目选项无效".to_owned()))?;
    parse_options(&serialized).map_err(|_| ApiError::Validation("题目选项无效".to_owned()))
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
