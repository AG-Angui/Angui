use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{audit_events, intake_question_definitions, intake_sessions},
    error::ApiError,
    models::{
        AuthenticatedUser, CreateIntakeSessionRequest, IntakeInitialAnswers, IntakeQuestion,
        IntakeSessionResponse,
    },
    roles::AccountType,
};

pub async fn create_intake_session(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    request: CreateIntakeSessionRequest,
    answer_hard_max: usize,
) -> Result<IntakeSessionResponse, ApiError> {
    if auth.account_type != AccountType::Member {
        return Err(ApiError::Forbidden(
            "only operational member accounts can create intake sessions".to_owned(),
        ));
    }

    let transaction = db.begin().await?;
    let questions = active_questions(&transaction).await?;
    let question_set_version = questions
        .first()
        .map(|question| question.version)
        .ok_or(ApiError::Internal)?;
    let answers = normalize_answers(request.initial_answers, &questions, answer_hard_max)?;
    let answers_json = serde_json::to_string(&answers).map_err(|_| ApiError::Internal)?;
    let timestamp = now();
    let session_id = Uuid::new_v4().to_string();

    let session = intake_sessions::ActiveModel {
        id: Set(session_id.clone()),
        created_by_user_id: Set(auth.id.clone()),
        case_id: Set(None),
        question_set_version: Set(question_set_version),
        status: Set("collecting".to_owned()),
        answers_json: Set(answers_json),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp),
    }
    .insert(&transaction)
    .await?;

    // Deliberately omit answers from audit metadata: they can include health and
    // identifying information and must not become broadly queryable metadata.
    write_audit(
        &transaction,
        auth,
        session_id,
        Some(json!({ "status": "collecting", "guidance_mode": "rule_based" })),
    )
    .await?;

    transaction.commit().await?;
    Ok(response_for(session, answers, &questions))
}

async fn active_questions<C: ConnectionTrait>(
    db: &C,
) -> Result<Vec<intake_question_definitions::Model>, ApiError> {
    let latest = intake_question_definitions::Entity::find()
        .filter(intake_question_definitions::Column::Status.eq("active"))
        .order_by_desc(intake_question_definitions::Column::Version)
        .one(db)
        .await?
        .ok_or(ApiError::Internal)?;
    let questions = intake_question_definitions::Entity::find()
        .filter(intake_question_definitions::Column::Status.eq("active"))
        .filter(intake_question_definitions::Column::Version.eq(latest.version))
        .order_by_asc(intake_question_definitions::Column::DisplayOrder)
        .all(db)
        .await?;
    validate_question_configuration(&questions)?;
    Ok(questions)
}

async fn write_audit<C: ConnectionTrait>(
    db: &C,
    auth: &AuthenticatedUser,
    session_id: String,
    metadata: Option<serde_json::Value>,
) -> Result<(), ApiError> {
    let metadata_json = metadata.map(|mut value| {
        if let Some(object) = value.as_object_mut() {
            object.insert("actor_account_type".to_owned(), json!(auth.account_type));
            object.insert(
                "actor_global_capabilities".to_owned(),
                json!(auth.global_capabilities),
            );
        }
        value.to_string()
    });
    audit_events::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        case_id: Set(None),
        actor: Set(auth.id.clone()),
        action: Set("intake_session.created".to_owned()),
        entity_type: Set("intake_session".to_owned()),
        entity_id: Set(session_id),
        metadata_json: Set(metadata_json),
        created_at: Set(now()),
    }
    .insert(db)
    .await?;
    Ok(())
}

fn normalize_answers(
    mut answers: IntakeInitialAnswers,
    questions: &[intake_question_definitions::Model],
    answer_hard_max: usize,
) -> Result<IntakeInitialAnswers, ApiError> {
    for (field_code, answer) in [
        ("basic_information", &mut answers.basic_information),
        ("health_status", &mut answers.health_status),
        ("behavior_habits", &mut answers.behavior_habits),
        ("last_seen", &mut answers.last_seen),
        ("frequent_locations", &mut answers.frequent_locations),
        ("belongings", &mut answers.belongings),
        ("transport_ability", &mut answers.transport_ability),
        ("follow_up_clues", &mut answers.follow_up_clues),
    ]
    .into_iter()
    {
        let Some(answer) = answer else {
            continue;
        };
        let question = questions
            .iter()
            .find(|question| question.field_code == field_code)
            .ok_or_else(|| {
                ApiError::Validation(format!(
                    "{field_code} is not enabled in the active intake question set"
                ))
            })?;
        let answer_limit = usize::try_from(question.max_answer_chars)
            .map_err(|_| ApiError::Internal)?
            .min(answer_hard_max);
        let trimmed = answer.trim();
        if trimmed.is_empty() || trimmed.chars().count() > answer_limit {
            return Err(ApiError::Validation(format!(
                "{field_code} must contain between 1 and {answer_limit} characters"
            )));
        }
        *answer = trimmed.to_owned();
    }
    Ok(answers)
}

fn response_for(
    model: intake_sessions::Model,
    answers: IntakeInitialAnswers,
    questions: &[intake_question_definitions::Model],
) -> IntakeSessionResponse {
    let missing_fields = missing_fields(&answers, questions);
    let next_question = questions
        .iter()
        .find(|question| missing_fields.contains(&question.field_code))
        .map(|question| IntakeQuestion {
            field: question.field_code.clone(),
            prompt: question.prompt.clone(),
            required: question.is_required,
        });
    IntakeSessionResponse::new(model, answers, missing_fields, next_question)
}

fn missing_fields(
    answers: &IntakeInitialAnswers,
    questions: &[intake_question_definitions::Model],
) -> Vec<String> {
    questions
        .iter()
        .filter(|question| answer_for(answers, &question.field_code).is_none())
        .map(|question| question.field_code.clone())
        .collect()
}

fn answer_for<'a>(answers: &'a IntakeInitialAnswers, field_code: &str) -> Option<&'a String> {
    match field_code {
        "basic_information" => answers.basic_information.as_ref(),
        "health_status" => answers.health_status.as_ref(),
        "behavior_habits" => answers.behavior_habits.as_ref(),
        "last_seen" => answers.last_seen.as_ref(),
        "frequent_locations" => answers.frequent_locations.as_ref(),
        "belongings" => answers.belongings.as_ref(),
        "transport_ability" => answers.transport_ability.as_ref(),
        "follow_up_clues" => answers.follow_up_clues.as_ref(),
        _ => None,
    }
}

fn validate_question_configuration(
    questions: &[intake_question_definitions::Model],
) -> Result<(), ApiError> {
    if questions.is_empty()
        || questions.iter().any(|question| {
            question.max_answer_chars <= 0
                || !matches!(
                    question.field_code.as_str(),
                    "basic_information"
                        | "health_status"
                        | "behavior_habits"
                        | "last_seen"
                        | "frequent_locations"
                        | "belongings"
                        | "transport_ability"
                        | "follow_up_clues"
                )
        })
    {
        return Err(ApiError::Internal);
    }
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
