use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{ai_execution_events, ai_executions, intake_sessions},
    error::ApiError,
    models::AuthenticatedUser,
    services::case_service,
};

#[derive(Clone, Debug, Serialize)]
pub struct AiExecutionResponse {
    pub execution_id: String,
    pub workflow: String,
    pub stage: String,
    pub status: String,
    pub failure_kind: Option<String>,
    pub result_status: Option<String>,
    pub fallback_used: bool,
    pub last_event_id: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct AiExecutionEventResponse {
    pub execution_id: String,
    pub event_id: i64,
    pub event_type: String,
    pub stage: Option<String>,
    pub created_at: String,
}

struct ExecutionTransition<'a> {
    stage: &'a str,
    status: &'a str,
    failure_kind: Option<&'a str>,
    result_status: Option<&'a str>,
    fallback_used: bool,
}

pub async fn start_intake_execution(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    intake_session_id: &str,
    workflow: &str,
) -> Result<(AiExecutionResponse, AiExecutionEventResponse), ApiError> {
    let transaction = db.begin().await?;
    let session = intake_sessions::Entity::find_by_id(intake_session_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("intake session was not found".to_owned()))?;
    if session.created_by_user_id != auth.id {
        return Err(ApiError::NotFound(
            "intake session was not found".to_owned(),
        ));
    }

    let timestamp = now();
    let execution_id = Uuid::new_v4().to_string();
    let model = ai_executions::ActiveModel {
        id: Set(execution_id.clone()),
        owner_user_id: Set(auth.id.clone()),
        intake_session_id: Set(Some(intake_session_id.to_owned())),
        workflow: Set(workflow.to_owned()),
        stage: Set("queued".to_owned()),
        status: Set("running".to_owned()),
        failure_kind: Set(None),
        result_status: Set(None),
        fallback_used: Set(false),
        last_event_id: Set(1),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp.clone()),
    }
    .insert(&transaction)
    .await?;
    let event = insert_event(
        &transaction,
        &execution_id,
        1,
        "ai_review.started",
        Some("queued"),
        &timestamp,
    )
    .await?;
    write_execution_audit(
        &transaction,
        auth,
        "ai_execution.started",
        &execution_id,
        &ExecutionTransition {
            stage: "queued",
            status: "running",
            failure_kind: None,
            result_status: None,
            fallback_used: false,
        },
    )
    .await?;
    transaction.commit().await?;
    Ok((response(model), event_response(event)))
}

/// Starts a case-scoped controlled execution. Case-role authorization remains
/// workflow-specific in the caller; this function deliberately records no
/// case content, source material, prompt, or candidate data.
pub async fn start_case_execution(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    workflow: &str,
) -> Result<(AiExecutionResponse, AiExecutionEventResponse), ApiError> {
    start_execution(db, auth, None, workflow).await
}

async fn start_execution(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    intake_session_id: Option<&str>,
    workflow: &str,
) -> Result<(AiExecutionResponse, AiExecutionEventResponse), ApiError> {
    let transaction = db.begin().await?;
    let timestamp = now();
    let execution_id = Uuid::new_v4().to_string();
    let model = ai_executions::ActiveModel {
        id: Set(execution_id.clone()),
        owner_user_id: Set(auth.id.clone()),
        intake_session_id: Set(intake_session_id.map(str::to_owned)),
        workflow: Set(workflow.to_owned()),
        stage: Set("queued".to_owned()),
        status: Set("running".to_owned()),
        failure_kind: Set(None),
        result_status: Set(None),
        fallback_used: Set(false),
        last_event_id: Set(1),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp.clone()),
    }
    .insert(&transaction)
    .await?;
    let event = insert_event(
        &transaction,
        &execution_id,
        1,
        "ai_review.started",
        Some("queued"),
        &timestamp,
    )
    .await?;
    write_execution_audit(
        &transaction,
        auth,
        "ai_execution.started",
        &execution_id,
        &ExecutionTransition {
            stage: "queued",
            status: "running",
            failure_kind: None,
            result_status: None,
            fallback_used: false,
        },
    )
    .await?;
    transaction.commit().await?;
    Ok((response(model), event_response(event)))
}

pub async fn get_execution(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    execution_id: &str,
) -> Result<AiExecutionResponse, ApiError> {
    let model = owned_execution(db, auth, execution_id).await?;
    Ok(response(model))
}

pub async fn list_events(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    execution_id: &str,
    after: i64,
) -> Result<Vec<AiExecutionEventResponse>, ApiError> {
    owned_execution(db, auth, execution_id).await?;
    let events = ai_execution_events::Entity::find()
        .filter(ai_execution_events::Column::ExecutionId.eq(execution_id))
        .filter(ai_execution_events::Column::EventId.gt(after.max(0)))
        .order_by_asc(ai_execution_events::Column::EventId)
        .all(db)
        .await?;
    Ok(events.into_iter().map(event_response).collect())
}

pub async fn advance_execution(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    execution_id: &str,
    stage: &str,
) -> Result<AiExecutionEventResponse, ApiError> {
    update_execution(
        db,
        auth,
        execution_id,
        &ExecutionTransition {
            stage,
            status: "running",
            failure_kind: None,
            result_status: None,
            fallback_used: false,
        },
        "ai_review.stage",
        "ai_execution.stage",
    )
    .await
}

pub async fn complete_execution(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    execution_id: &str,
    result_status: &str,
    fallback_used: bool,
) -> Result<AiExecutionEventResponse, ApiError> {
    update_execution(
        db,
        auth,
        execution_id,
        &ExecutionTransition {
            stage: "ready_for_review",
            status: "completed",
            failure_kind: None,
            result_status: Some(result_status),
            fallback_used,
        },
        "ai_review.completed",
        "ai_execution.completed",
    )
    .await
}

pub async fn fail_execution(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    execution_id: &str,
    failure_kind: &str,
) -> Result<AiExecutionEventResponse, ApiError> {
    update_execution(
        db,
        auth,
        execution_id,
        &ExecutionTransition {
            stage: "failed",
            status: "failed",
            failure_kind: Some(failure_kind),
            result_status: None,
            fallback_used: false,
        },
        "ai_review.failed",
        "ai_execution.failed",
    )
    .await
}

async fn update_execution(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    execution_id: &str,
    transition: &ExecutionTransition<'_>,
    event_type: &str,
    audit_action: &str,
) -> Result<AiExecutionEventResponse, ApiError> {
    let transaction = db.begin().await?;
    let existing = owned_execution(&transaction, auth, execution_id).await?;
    if existing.status != "running" {
        return Err(ApiError::Conflict(
            "AI execution has already reached a terminal state".to_owned(),
        ));
    }
    let timestamp = now();
    let event_id = existing.last_event_id + 1;
    let mut active = existing.into_active_model();
    active.stage = Set(transition.stage.to_owned());
    active.status = Set(transition.status.to_owned());
    active.failure_kind = Set(transition.failure_kind.map(str::to_owned));
    active.result_status = Set(transition.result_status.map(str::to_owned));
    active.fallback_used = Set(transition.fallback_used);
    active.last_event_id = Set(event_id);
    active.updated_at = Set(timestamp.clone());
    active.update(&transaction).await?;
    let event = insert_event(
        &transaction,
        execution_id,
        event_id,
        event_type,
        Some(transition.stage),
        &timestamp,
    )
    .await?;
    write_execution_audit(&transaction, auth, audit_action, execution_id, transition).await?;
    transaction.commit().await?;
    Ok(event_response(event))
}

async fn owned_execution<C: sea_orm::ConnectionTrait>(
    db: &C,
    auth: &AuthenticatedUser,
    execution_id: &str,
) -> Result<ai_executions::Model, ApiError> {
    ai_executions::Entity::find_by_id(execution_id)
        .filter(ai_executions::Column::OwnerUserId.eq(&auth.id))
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("AI execution was not found".to_owned()))
}

async fn insert_event<C: sea_orm::ConnectionTrait>(
    db: &C,
    execution_id: &str,
    event_id: i64,
    event_type: &str,
    stage: Option<&str>,
    created_at: &str,
) -> Result<ai_execution_events::Model, ApiError> {
    Ok(ai_execution_events::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        execution_id: Set(execution_id.to_owned()),
        event_id: Set(event_id),
        event_type: Set(event_type.to_owned()),
        stage: Set(stage.map(str::to_owned)),
        created_at: Set(created_at.to_owned()),
    }
    .insert(db)
    .await?)
}

async fn write_execution_audit<C: sea_orm::ConnectionTrait>(
    db: &C,
    auth: &AuthenticatedUser,
    action: &str,
    execution_id: &str,
    transition: &ExecutionTransition<'_>,
) -> Result<(), ApiError> {
    case_service::write_audit(
        db,
        None,
        auth,
        action,
        "ai_execution",
        execution_id.to_owned(),
        Some(json!({
            "execution_id": execution_id,
            "stage": transition.stage,
            "status": transition.status,
            "result_status": transition.result_status,
            "failure_kind": transition.failure_kind,
            "fallback_used": transition.fallback_used,
        })),
    )
    .await
}

fn response(model: ai_executions::Model) -> AiExecutionResponse {
    AiExecutionResponse {
        execution_id: model.id,
        workflow: model.workflow,
        stage: model.stage,
        status: model.status,
        failure_kind: model.failure_kind,
        result_status: model.result_status,
        fallback_used: model.fallback_used,
        last_event_id: model.last_event_id,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

fn event_response(model: ai_execution_events::Model) -> AiExecutionEventResponse {
    AiExecutionEventResponse {
        execution_id: model.execution_id,
        event_id: model.event_id,
        event_type: model.event_type,
        stage: model.stage,
        created_at: model.created_at,
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
