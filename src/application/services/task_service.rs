use std::{
    collections::{HashMap, HashSet},
    time::Duration as StdDuration,
};

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait, TryInsertResult,
};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    entities::{
        case_attachments, case_memberships, cases, clue_attachment_links, clue_attributions, clues,
        task_assignments, task_location_reports, task_operation_idempotency, tasks,
        user_global_capabilities, users,
    },
    error::ApiError,
    models::{
        AuthenticatedUser, CreateTaskRequest, SubmitTaskFeedbackRequest,
        SubmitTaskLocationReportRequest, TaskFeedbackReceipt, TaskListQuery, TaskListResponse,
        TaskLocationReportReceipt, TaskNavigationResponse, TaskResponse,
        TaskSafetyBriefingResponse, UpdateTaskStatusRequest,
    },
    roles::{CaseRole, GlobalCapability},
    services::case_service::{require_case_role, write_audit},
};

const TASK_STATUSES: &[&str] = &[
    "pending_claim",
    "assigned",
    "accepted",
    "active",
    "blocked",
    "completed",
    "cancelled",
];
const TASK_RISK_LEVELS: &[&str] = &["low", "medium", "high", "critical"];
const MAX_TASK_PAGE_SIZE: u64 = 100;
const LOCATION_REPORT_SOURCE: &str = "simulated";
const MAX_LOCATION_REPORT_AGE: Duration = Duration::minutes(15);
const MAX_LOCATION_REPORT_FUTURE_SKEW: Duration = Duration::minutes(5);
const LOCATION_REPORT_RETENTION: Duration = Duration::hours(24);
const LOCATION_REPORT_PURGE_INTERVAL: StdDuration = StdDuration::from_secs(60);
const CLUE_LOCATION_PRECISIONS: &[&str] = &["exact", "approximate", "unknown"];
const LOCATION_REPORT_OPERATION: &str = "location_report";
const TASK_FEEDBACK_OPERATION: &str = "task_feedback";

pub fn start_location_report_retention_purger(db: DatabaseConnection) {
    actix_web::rt::spawn(async move {
        let mut interval = tokio::time::interval(LOCATION_REPORT_PURGE_INTERVAL);
        loop {
            interval.tick().await;
            if let Err(error) = purge_expired_location_reports(&db).await {
                eprintln!("failed to purge expired task location reports: {error}");
            }
        }
    });
}

pub async fn purge_expired_location_reports(db: &DatabaseConnection) -> Result<u64, ApiError> {
    let deleted = task_location_reports::Entity::delete_many()
        .filter(task_location_reports::Column::RetentionExpiresAt.lte(now()))
        .exec(db)
        .await?;
    Ok(deleted.rows_affected)
}

pub async fn create_task(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: CreateTaskRequest,
) -> Result<TaskResponse, ApiError> {
    let request = ValidatedCreateTaskRequest::try_from(request)?;
    let transaction = db.begin().await?;
    require_case_role(&transaction, &auth.id, case_id, &[CaseRole::Commander]).await?;

    let case = cases::Entity::find_by_id(case_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("case was not found".to_owned()))?;
    if case.status == "closed" {
        return Err(ApiError::Conflict(
            "tasks cannot be created for a closed case".to_owned(),
        ));
    }

    let source_clue = clues::Entity::find_by_id(&request.source_clue_id)
        .one(&transaction)
        .await?
        .filter(|clue| clue.case_id == case_id && clue.status == "confirmed")
        .ok_or_else(|| {
            ApiError::Validation(
                "source_clue_id must reference a confirmed clue in this case".to_owned(),
            )
        })?;

    let volunteer_membership = case_memberships::Entity::find()
        .filter(case_memberships::Column::CaseId.eq(case_id))
        .filter(case_memberships::Column::UserId.eq(&request.volunteer_user_id))
        .filter(case_memberships::Column::Role.eq(CaseRole::Volunteer.to_string()))
        .one(&transaction)
        .await?;
    let volunteer_is_active = users::Entity::find_by_id(&request.volunteer_user_id)
        .filter(users::Column::Status.eq("active"))
        .one(&transaction)
        .await?
        .is_some();
    let volunteer_is_authorized = user_global_capabilities::Entity::find()
        .filter(user_global_capabilities::Column::UserId.eq(&request.volunteer_user_id))
        .filter(user_global_capabilities::Column::Capability.eq("volunteer"))
        .one(&transaction)
        .await?
        .is_some();
    if volunteer_membership.is_none() || !volunteer_is_active || !volunteer_is_authorized {
        return Err(ApiError::Validation(
            "volunteer_user_id must reference an active volunteer in this case".to_owned(),
        ));
    }

    let timestamp = now();
    let task_id = crate::services::case_service::new_id();
    let task = tasks::ActiveModel {
        id: Set(task_id.clone()),
        case_id: Set(case_id.to_owned()),
        source_clue_id: Set(Some(source_clue.id)),
        title: Set(request.title),
        objective: Set(request.objective),
        area_text: Set(request.area_text),
        latitude: Set(request.latitude),
        longitude: Set(request.longitude),
        due_at: Set(request.due_at),
        background: Set(request.background),
        risk_level: Set(request.risk_level),
        risk_notes: Set(request.risk_notes),
        safety_briefing: Set(request.safety_briefing),
        expected_feedback: Set(request.expected_feedback),
        status: Set("assigned".to_owned()),
        result_summary: Set(None),
        created_by_user_id: Set(auth.id.clone()),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp.clone()),
    }
    .insert(&transaction)
    .await?;
    let assignment = task_assignments::ActiveModel {
        task_id: Set(task_id.clone()),
        volunteer_user_id: Set(request.volunteer_user_id.clone()),
        assigned_by_user_id: Set(auth.id.clone()),
        assigned_at: Set(timestamp.clone()),
        updated_at: Set(timestamp),
    }
    .insert(&transaction)
    .await?;

    write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "task.created",
        "task",
        task_id.clone(),
        Some(json!({
            "status": "assigned",
            "source_clue_id": task.source_clue_id,
            "risk_level": task.risk_level,
        })),
    )
    .await?;
    write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "task.assigned",
        "task",
        task_id,
        Some(json!({ "volunteer_user_id": assignment.volunteer_user_id })),
    )
    .await?;

    transaction.commit().await?;
    Ok(TaskResponse::new(task, Some(assignment), true))
}

pub async fn list_tasks(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    query: TaskListQuery,
) -> Result<TaskListResponse, ApiError> {
    let query = ValidatedTaskListQuery::try_from(query)?;
    let visible_tasks = load_visible_tasks(db, auth, case_id).await?;
    let total = u64::try_from(visible_tasks.len()).map_err(|_| ApiError::Internal)?;
    let start = query.offset()?;
    let items = visible_tasks
        .into_iter()
        .skip(start)
        .take(query.page_size_usize())
        .collect();

    Ok(TaskListResponse {
        items,
        page: query.page,
        page_size: query.page_size,
        total,
    })
}

async fn load_visible_tasks(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<Vec<TaskResponse>, ApiError> {
    let case_role = require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    if case_role == CaseRole::Family {
        return Ok(Vec::new());
    }

    let task_models = tasks::Entity::find()
        .filter(tasks::Column::CaseId.eq(case_id))
        .order_by_asc(tasks::Column::DueAt)
        .order_by_asc(tasks::Column::Id)
        .all(db)
        .await?;
    let assignments =
        assignments_for_tasks(db, task_models.iter().map(|task| task.id.clone())).await?;
    Ok(task_models
        .into_iter()
        .filter(|task| {
            case_role == CaseRole::Commander
                || assignments
                    .get(&task.id)
                    .is_some_and(|assignment| assignment.volunteer_user_id == auth.id)
        })
        .map(|task| {
            let assignment = assignments.get(&task.id).cloned();
            TaskResponse::new(task, assignment, case_role == CaseRole::Commander)
        })
        .collect())
}

pub async fn list_all_visible_tasks(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<Vec<TaskResponse>, ApiError> {
    load_visible_tasks(db, auth, case_id).await
}

pub async fn list_my_tasks(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
) -> Result<Vec<TaskResponse>, ApiError> {
    if !auth
        .global_capabilities
        .contains(&GlobalCapability::Volunteer)
    {
        return Err(ApiError::Forbidden(
            "only volunteer accounts can access the personal task queue".to_owned(),
        ));
    }

    let volunteer_case_ids = case_memberships::Entity::find()
        .filter(case_memberships::Column::UserId.eq(&auth.id))
        .filter(case_memberships::Column::Role.eq(CaseRole::Volunteer.to_string()))
        .all(db)
        .await?
        .into_iter()
        .map(|membership| membership.case_id)
        .collect::<HashSet<_>>();
    if volunteer_case_ids.is_empty() {
        return Ok(Vec::new());
    }
    let assignments = task_assignments::Entity::find()
        .filter(task_assignments::Column::VolunteerUserId.eq(&auth.id))
        .all(db)
        .await?;
    if assignments.is_empty() {
        return Ok(Vec::new());
    }
    let assignments = assignments
        .into_iter()
        .map(|assignment| (assignment.task_id.clone(), assignment))
        .collect::<HashMap<_, _>>();
    let task_models = tasks::Entity::find()
        .filter(tasks::Column::Id.is_in(assignments.keys().cloned()))
        .order_by_asc(tasks::Column::DueAt)
        .order_by_asc(tasks::Column::Id)
        .all(db)
        .await?;

    Ok(task_models
        .into_iter()
        .filter(|task| volunteer_case_ids.contains(&task.case_id))
        .map(|task| {
            let assignment = assignments.get(&task.id).cloned();
            TaskResponse::new(task, assignment, false)
        })
        .collect())
}

pub async fn get_task_safety_briefing(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    task_id: &str,
) -> Result<TaskSafetyBriefingResponse, ApiError> {
    let task = load_task_for_assignee_or_commander(db, auth, task_id).await?;
    let mut notices = vec![task.safety_briefing.clone(), task.risk_notes.clone()];
    notices.push(format!("区域提示：{}", task.area_text));
    notices.push("夜间、恶劣天气或能见度不足时不要单独行动；情况不明时先联系指挥。".to_owned());
    notices.push(match task.risk_level.as_str() {
        "critical" | "high" => {
            "高风险任务不得擅自进入受限区域、跨越危险地带或改变任务目标。".to_owned()
        }
        "medium" => "执行中保持联络，遇到阻碍先暂停并报告。".to_owned(),
        _ => "保持任务边界，任何异常情况先暂停并报告。".to_owned(),
    });
    notices.push("当前为规则化安全提示；出发前请自行核实当地天气、道路与现场条件。".to_owned());

    Ok(TaskSafetyBriefingResponse {
        task_id: task.id,
        risk_level: task.risk_level,
        notices,
        emergency_stop_message:
            "立即停止行动并前往安全、公开地点；通过既有人工联系方式向指挥报告。".to_owned(),
        source: "rule_based".to_owned(),
        degradation_status: "rule_based_fallback".to_owned(),
        updated_at: task.updated_at,
    })
}

pub async fn get_task_navigation(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    task_id: &str,
) -> Result<TaskNavigationResponse, ApiError> {
    let task = load_task_for_assignee_or_commander(db, auth, task_id).await?;
    let coordinate_note = if task.latitude.is_some() && task.longitude.is_some() {
        "任务点坐标仅在受限任务卡内可见；因当前任务坐标未声明坐标系，服务不会生成可能偏移的第三方导航链接。"
    } else {
        "该任务未配置坐标，请依据文字区域说明并与指挥确认集合点或路线。"
    };

    Ok(TaskNavigationResponse {
        task_id: task.id,
        area_text: task.area_text.clone(),
        navigation_url: None,
        route_summary: format!(
            "前往任务区域：{}。{} 执行前阅读任务安全提示，导航不可用时继续使用文字任务卡并联系指挥。",
            task.area_text, coordinate_note
        ),
        source: "task_area_text".to_owned(),
        degradation_status: "text_fallback".to_owned(),
        fallback_message: Some(
            "自动导航当前不可用；请使用任务区域文字说明并联系指挥确认路线。".to_owned(),
        ),
        updated_at: task.updated_at,
    })
}

async fn load_task_for_assignee_or_commander(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    task_id: &str,
) -> Result<tasks::Model, ApiError> {
    let task = tasks::Entity::find_by_id(task_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("task was not found".to_owned()))?;
    let role = require_case_role(
        db,
        &auth.id,
        &task.case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    if role == CaseRole::Commander {
        return Ok(task);
    }
    let is_assignee = role == CaseRole::Volunteer
        && task_assignments::Entity::find()
            .filter(task_assignments::Column::TaskId.eq(task_id))
            .filter(task_assignments::Column::VolunteerUserId.eq(&auth.id))
            .one(db)
            .await?
            .is_some_and(|assignment| assignment.volunteer_user_id == auth.id);
    if !is_assignee {
        return Err(ApiError::NotFound("task was not found".to_owned()));
    }
    Ok(task)
}

pub async fn update_task_status(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    task_id: &str,
    request: UpdateTaskStatusRequest,
) -> Result<TaskResponse, ApiError> {
    let next_status = request.status.trim().to_lowercase();
    if !TASK_STATUSES.contains(&next_status.as_str()) {
        return Err(ApiError::Validation("status is unsupported".to_owned()));
    }
    let transaction = db.begin().await?;
    let task = tasks::Entity::find_by_id(task_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("task was not found".to_owned()))?;
    let case_role = require_case_role(
        &transaction,
        &auth.id,
        &task.case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let assignment = task_assignments::Entity::find()
        .filter(task_assignments::Column::TaskId.eq(task_id))
        .filter(task_assignments::Column::VolunteerUserId.eq(&auth.id))
        .one(&transaction)
        .await?
        .ok_or_else(|| {
            ApiError::Database(sea_orm::DbErr::Custom(
                "task is missing its required assignment".to_owned(),
            ))
        })?;

    if case_role == CaseRole::Commander {
        if next_status != "cancelled" || is_terminal_status(&task.status) {
            return Err(ApiError::Conflict(
                "commanders can only cancel unfinished tasks".to_owned(),
            ));
        }
    } else {
        if case_role != CaseRole::Volunteer || assignment.volunteer_user_id != auth.id {
            return Err(ApiError::NotFound("task was not found".to_owned()));
        }
        if !volunteer_transition_allowed(&task.status, &next_status) {
            return Err(ApiError::Conflict(format!(
                "task status cannot change from {} to {}",
                task.status, next_status
            )));
        }
    }

    let previous_status = task.status.clone();
    let update_result = tasks::Entity::update_many()
        .col_expr(tasks::Column::Status, Expr::value(next_status.clone()))
        .col_expr(tasks::Column::UpdatedAt, Expr::value(now()))
        .filter(tasks::Column::Id.eq(task_id))
        .filter(tasks::Column::Status.eq(&previous_status))
        .exec(&transaction)
        .await?;
    if update_result.rows_affected != 1 {
        return Err(ApiError::Conflict(
            "task status changed before this transition could be applied".to_owned(),
        ));
    }
    let updated = tasks::Entity::find_by_id(task_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::Internal)?;
    write_audit(
        &transaction,
        Some(updated.case_id.clone()),
        auth,
        "task.status_changed",
        "task",
        updated.id.clone(),
        Some(json!({ "from": previous_status, "to": next_status })),
    )
    .await?;
    transaction.commit().await?;

    Ok(TaskResponse::new(
        updated,
        Some(assignment),
        case_role == CaseRole::Commander,
    ))
}

pub async fn submit_location_report(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    task_id: &str,
    request: SubmitTaskLocationReportRequest,
    idempotency_key: &str,
) -> Result<TaskLocationReportReceipt, ApiError> {
    let request = ValidatedLocationReportRequest::try_from(request)?;
    let request_fingerprint = request_fingerprint(&json!({
        "latitude": request.latitude,
        "longitude": request.longitude,
        "accuracy_meters": request.accuracy_meters,
        "captured_at": format_timestamp(request.captured_at),
    }))?;
    let transaction = db.begin().await?;
    let task = tasks::Entity::find_by_id(task_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("task was not found".to_owned()))?;
    let case_role = require_case_role(
        &transaction,
        &auth.id,
        &task.case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let assignment = task_assignments::Entity::find()
        .filter(task_assignments::Column::TaskId.eq(task_id))
        .filter(task_assignments::Column::VolunteerUserId.eq(&auth.id))
        .one(&transaction)
        .await?
        .ok_or_else(|| {
            ApiError::Database(sea_orm::DbErr::Custom(
                "task is missing its required assignment".to_owned(),
            ))
        })?;
    if case_role != CaseRole::Volunteer || assignment.volunteer_user_id != auth.id {
        return Err(ApiError::NotFound("task was not found".to_owned()));
    }
    if let Some(receipt) = existing_idempotent_receipt::<TaskLocationReportReceipt>(
        &transaction,
        task_id,
        &auth.id,
        LOCATION_REPORT_OPERATION,
        idempotency_key,
        &request_fingerprint,
    )
    .await?
    {
        transaction.commit().await?;
        return Ok(receipt);
    }
    if !claim_idempotency_key(
        &transaction,
        task_id,
        &auth.id,
        LOCATION_REPORT_OPERATION,
        idempotency_key,
        &request_fingerprint,
    )
    .await?
    {
        let receipt = existing_idempotent_receipt::<TaskLocationReportReceipt>(
            &transaction,
            task_id,
            &auth.id,
            LOCATION_REPORT_OPERATION,
            idempotency_key,
            &request_fingerprint,
        )
        .await?
        .ok_or_else(|| {
            ApiError::Conflict("a concurrent request is using this Idempotency-Key".to_owned())
        })?;
        transaction.commit().await?;
        return Ok(receipt);
    }
    if task.status != "active" {
        return Err(ApiError::Conflict(
            "location reports can only be submitted while the task is active".to_owned(),
        ));
    }

    let report_id = crate::services::case_service::new_id();
    let created_at = now();
    let active_guard = tasks::Entity::update_many()
        .col_expr(tasks::Column::UpdatedAt, Expr::value(created_at.clone()))
        .filter(tasks::Column::Id.eq(task_id))
        .filter(tasks::Column::Status.eq("active"))
        .exec(&transaction)
        .await?;
    if active_guard.rows_affected != 1 {
        return Err(ApiError::Conflict(
            "location reports can only be submitted while the task is active".to_owned(),
        ));
    }
    let retention_expires_at = request
        .captured_at
        .checked_add_signed(LOCATION_REPORT_RETENTION)
        .ok_or_else(|| ApiError::Internal)?;
    let report = task_location_reports::ActiveModel {
        id: Set(report_id.clone()),
        task_id: Set(task_id.to_owned()),
        volunteer_user_id: Set(auth.id.clone()),
        source: Set(LOCATION_REPORT_SOURCE.to_owned()),
        latitude: Set(request.latitude),
        longitude: Set(request.longitude),
        accuracy_meters: Set(request.accuracy_meters),
        captured_at: Set(format_timestamp(request.captured_at)),
        retention_expires_at: Set(format_timestamp(retention_expires_at)),
        created_at: Set(created_at.clone()),
    }
    .insert(&transaction)
    .await?;

    // Exact coordinates and accuracy are deliberately absent from audit metadata.
    write_audit(
        &transaction,
        Some(task.case_id),
        auth,
        "task.location_reported",
        "task_location_report",
        report_id,
        Some(json!({
            "source": LOCATION_REPORT_SOURCE,
            "captured_at": report.captured_at,
        })),
    )
    .await?;
    let receipt = TaskLocationReportReceipt {
        id: report.id,
        source: report.source,
        captured_at: report.captured_at,
        retention_expires_at: report.retention_expires_at,
        created_at: report.created_at,
    };
    persist_idempotent_receipt(
        &transaction,
        task_id,
        &auth.id,
        LOCATION_REPORT_OPERATION,
        idempotency_key,
        &receipt,
    )
    .await?;
    transaction.commit().await?;

    Ok(receipt)
}

pub async fn submit_task_feedback(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    task_id: &str,
    request: SubmitTaskFeedbackRequest,
    idempotency_key: &str,
) -> Result<TaskFeedbackReceipt, ApiError> {
    let request = ValidatedTaskFeedbackRequest::try_from(request)?;
    let request_fingerprint = request_fingerprint(&json!({
        "content": request.content,
        "occurred_at": request.occurred_at,
        "location_text": request.location_text,
        "location_precision": request.location_precision,
        "attachment_ids": request.attachment_ids,
    }))?;
    let transaction = db.begin().await?;
    let task = tasks::Entity::find_by_id(task_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("task was not found".to_owned()))?;
    let case_role = require_case_role(
        &transaction,
        &auth.id,
        &task.case_id,
        &[CaseRole::Family, CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let assignment = task_assignments::Entity::find()
        .filter(task_assignments::Column::TaskId.eq(task_id))
        .filter(task_assignments::Column::VolunteerUserId.eq(&auth.id))
        .one(&transaction)
        .await?
        .ok_or_else(|| {
            ApiError::Database(sea_orm::DbErr::Custom(
                "task is missing its required assignment".to_owned(),
            ))
        })?;
    if case_role != CaseRole::Volunteer || assignment.volunteer_user_id != auth.id {
        return Err(ApiError::NotFound("task was not found".to_owned()));
    }
    if let Some(receipt) = existing_idempotent_receipt::<TaskFeedbackReceipt>(
        &transaction,
        task_id,
        &auth.id,
        TASK_FEEDBACK_OPERATION,
        idempotency_key,
        &request_fingerprint,
    )
    .await?
    {
        transaction.commit().await?;
        return Ok(receipt);
    }
    if !claim_idempotency_key(
        &transaction,
        task_id,
        &auth.id,
        TASK_FEEDBACK_OPERATION,
        idempotency_key,
        &request_fingerprint,
    )
    .await?
    {
        let receipt = existing_idempotent_receipt::<TaskFeedbackReceipt>(
            &transaction,
            task_id,
            &auth.id,
            TASK_FEEDBACK_OPERATION,
            idempotency_key,
            &request_fingerprint,
        )
        .await?
        .ok_or_else(|| {
            ApiError::Conflict("a concurrent request is using this Idempotency-Key".to_owned())
        })?;
        transaction.commit().await?;
        return Ok(receipt);
    }
    if task.status != "active" {
        return Err(ApiError::Conflict(
            "feedback can only be submitted while the task is active".to_owned(),
        ));
    }
    let case_model = cases::Entity::find_by_id(&task.case_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("case was not found".to_owned()))?;
    if case_model.status != "active" {
        return Err(ApiError::Conflict(
            "feedback cannot be submitted for a non-active case".to_owned(),
        ));
    }

    let timestamp = now();
    let active_guard = tasks::Entity::update_many()
        .col_expr(tasks::Column::UpdatedAt, Expr::value(timestamp.clone()))
        .filter(tasks::Column::Id.eq(task_id))
        .filter(tasks::Column::Status.eq("active"))
        .exec(&transaction)
        .await?;
    if active_guard.rows_affected != 1 {
        return Err(ApiError::Conflict(
            "feedback can only be submitted while the task is active".to_owned(),
        ));
    }

    let clue_id = crate::services::case_service::new_id();
    let clue = clues::ActiveModel {
        id: Set(clue_id.clone()),
        case_id: Set(task.case_id.clone()),
        status: Set("pending_review".to_owned()),
        source: Set("task_feedback".to_owned()),
        source_type: Set("field_report".to_owned()),
        content: Set(request.content),
        raw_record_reference: Set(None),
        occurred_at: Set(request.occurred_at),
        reported_at: Set(timestamp.clone()),
        confirmed_at: Set(None),
        location_text: Set(request.location_text),
        location_precision: Set(request.location_precision),
        next_action: Set(None),
        linked_task_reference: Set(Some(task.id.clone())),
        related_clue_id: Set(None),
        relationship_type: Set(None),
        review_reason: Set(None),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp.clone()),
    }
    .insert(&transaction)
    .await?;

    for attachment_id in &request.attachment_ids {
        let attachment = case_attachments::Entity::find_by_id(attachment_id)
            .one(&transaction)
            .await?
            .filter(|attachment| attachment.case_id == task.case_id)
            .ok_or_else(|| {
                ApiError::Validation(
                    "attachment_ids must reference attachments in this case".to_owned(),
                )
            })?;
        if attachment.created_by_user_id != auth.id {
            return Err(ApiError::Forbidden(
                "an attachment can only be linked by its uploader".to_owned(),
            ));
        }
        clue_attachment_links::ActiveModel {
            clue_id: Set(clue_id.clone()),
            attachment_id: Set(attachment_id.clone()),
            created_at: Set(timestamp.clone()),
        }
        .insert(&transaction)
        .await?;
    }

    clue_attributions::ActiveModel {
        clue_id: Set(clue_id.clone()),
        submitted_by_user_id: Set(Some(auth.id.clone())),
        reviewed_by_user_id: Set(None),
        reviewed_at: Set(None),
    }
    .insert(&transaction)
    .await?;
    write_audit(
        &transaction,
        Some(task.case_id.clone()),
        auth,
        "task.feedback_submitted",
        "task",
        task.id.clone(),
        Some(json!({
            "clue_id": clue_id,
            "status": "pending_review",
            "attachment_count": request.attachment_ids.len(),
        })),
    )
    .await?;
    write_audit(
        &transaction,
        Some(task.case_id),
        auth,
        "clue.submitted",
        "clue",
        clue.id.clone(),
        Some(json!({
            "status": "pending_review",
            "source_type": "field_report",
            "linked_task_reference": task.id,
        })),
    )
    .await?;
    let receipt = TaskFeedbackReceipt {
        task_id: task_id.to_owned(),
        clue_id: clue.id,
        status: clue.status,
        submitted_at: timestamp,
    };
    persist_idempotent_receipt(
        &transaction,
        task_id,
        &auth.id,
        TASK_FEEDBACK_OPERATION,
        idempotency_key,
        &receipt,
    )
    .await?;
    transaction.commit().await?;

    Ok(receipt)
}

async fn existing_idempotent_receipt<T: serde::de::DeserializeOwned>(
    transaction: &sea_orm::DatabaseTransaction,
    task_id: &str,
    volunteer_user_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_fingerprint: &str,
) -> Result<Option<T>, ApiError> {
    let Some(record) = task_operation_idempotency::Entity::find()
        .filter(task_operation_idempotency::Column::TaskId.eq(task_id))
        .filter(task_operation_idempotency::Column::VolunteerUserId.eq(volunteer_user_id))
        .filter(task_operation_idempotency::Column::Operation.eq(operation))
        .filter(task_operation_idempotency::Column::IdempotencyKey.eq(idempotency_key))
        .one(transaction)
        .await?
    else {
        return Ok(None);
    };
    if record.request_fingerprint != request_fingerprint {
        return Err(ApiError::Conflict(
            "Idempotency-Key was reused with a different payload".to_owned(),
        ));
    }
    serde_json::from_str(&record.response_json)
        .map(Some)
        .map_err(|_| ApiError::Internal)
}

async fn claim_idempotency_key(
    transaction: &sea_orm::DatabaseTransaction,
    task_id: &str,
    volunteer_user_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_fingerprint: &str,
) -> Result<bool, ApiError> {
    let conflict_target = OnConflict::columns([
        task_operation_idempotency::Column::TaskId,
        task_operation_idempotency::Column::VolunteerUserId,
        task_operation_idempotency::Column::Operation,
        task_operation_idempotency::Column::IdempotencyKey,
    ])
    .do_nothing()
    .to_owned();
    let result = task_operation_idempotency::Entity::insert_many([
        task_operation_idempotency::ActiveModel {
            id: Set(crate::services::case_service::new_id()),
            task_id: Set(task_id.to_owned()),
            volunteer_user_id: Set(volunteer_user_id.to_owned()),
            operation: Set(operation.to_owned()),
            idempotency_key: Set(idempotency_key.to_owned()),
            request_fingerprint: Set(request_fingerprint.to_owned()),
            response_json: Set("{}".to_owned()),
            created_at: Set(now()),
        },
    ])
    .on_conflict(conflict_target)
    .do_nothing()
    .exec(transaction)
    .await?;
    Ok(matches!(result, TryInsertResult::Inserted(_)))
}

fn request_fingerprint(value: &serde_json::Value) -> Result<String, ApiError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ApiError::Internal)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

async fn persist_idempotent_receipt<T: serde::Serialize>(
    transaction: &sea_orm::DatabaseTransaction,
    task_id: &str,
    volunteer_user_id: &str,
    operation: &str,
    idempotency_key: &str,
    receipt: &T,
) -> Result<(), ApiError> {
    let response_json = serde_json::to_string(receipt).map_err(|_| ApiError::Internal)?;
    let update = task_operation_idempotency::Entity::update_many()
        .col_expr(
            task_operation_idempotency::Column::ResponseJson,
            Expr::value(response_json),
        )
        .filter(task_operation_idempotency::Column::TaskId.eq(task_id))
        .filter(task_operation_idempotency::Column::VolunteerUserId.eq(volunteer_user_id))
        .filter(task_operation_idempotency::Column::Operation.eq(operation))
        .filter(task_operation_idempotency::Column::IdempotencyKey.eq(idempotency_key))
        .exec(transaction)
        .await?;
    if update.rows_affected != 1 {
        return Err(ApiError::Internal);
    }
    Ok(())
}

async fn assignments_for_tasks(
    db: &DatabaseConnection,
    task_ids: impl IntoIterator<Item = String>,
) -> Result<HashMap<String, task_assignments::Model>, ApiError> {
    let task_ids = task_ids.into_iter().collect::<Vec<_>>();
    if task_ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(task_assignments::Entity::find()
        .filter(task_assignments::Column::TaskId.is_in(task_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|assignment| (assignment.task_id.clone(), assignment))
        .collect())
}

struct ValidatedCreateTaskRequest {
    source_clue_id: String,
    volunteer_user_id: String,
    title: String,
    objective: String,
    area_text: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    due_at: String,
    background: String,
    risk_level: String,
    risk_notes: String,
    safety_briefing: String,
    expected_feedback: String,
}

impl TryFrom<CreateTaskRequest> for ValidatedCreateTaskRequest {
    type Error = ApiError;

    fn try_from(value: CreateTaskRequest) -> Result<Self, Self::Error> {
        let source_clue_id = required_field("source_clue_id", value.source_clue_id, 36)?;
        let volunteer_user_id = required_field("volunteer_user_id", value.volunteer_user_id, 36)?;
        let title = required_field("title", value.title, 200)?;
        let objective = required_field("objective", value.objective, 4_000)?;
        let area_text = required_field("area_text", value.area_text, 500)?;
        validate_coordinates(value.latitude, value.longitude)?;
        let due_at = parse_due_at(&value.due_at)?;
        let background = required_field("background", value.background, 10_000)?;
        let risk_level = value.risk_level.trim().to_lowercase();
        if !TASK_RISK_LEVELS.contains(&risk_level.as_str()) {
            return Err(ApiError::Validation("risk_level is unsupported".to_owned()));
        }
        let risk_notes = required_field("risk_notes", value.risk_notes, 4_000)?;
        let safety_briefing = required_field("safety_briefing", value.safety_briefing, 4_000)?;
        let expected_feedback =
            required_field("expected_feedback", value.expected_feedback, 4_000)?;
        Ok(Self {
            source_clue_id,
            volunteer_user_id,
            title,
            objective,
            area_text,
            latitude: value.latitude,
            longitude: value.longitude,
            due_at,
            background,
            risk_level,
            risk_notes,
            safety_briefing,
            expected_feedback,
        })
    }
}

struct ValidatedTaskListQuery {
    page: u64,
    page_size: u64,
}

struct ValidatedLocationReportRequest {
    latitude: f64,
    longitude: f64,
    accuracy_meters: f64,
    captured_at: DateTime<Utc>,
}

struct ValidatedTaskFeedbackRequest {
    content: String,
    occurred_at: Option<String>,
    location_text: Option<String>,
    location_precision: Option<String>,
    attachment_ids: Vec<String>,
}

impl TryFrom<SubmitTaskLocationReportRequest> for ValidatedLocationReportRequest {
    type Error = ApiError;

    fn try_from(value: SubmitTaskLocationReportRequest) -> Result<Self, Self::Error> {
        if value.source.trim().to_lowercase() != LOCATION_REPORT_SOURCE {
            return Err(ApiError::Validation("source must be simulated".to_owned()));
        }
        if !value.latitude.is_finite()
            || !value.longitude.is_finite()
            || !(-90.0..=90.0).contains(&value.latitude)
            || !(-180.0..=180.0).contains(&value.longitude)
        {
            return Err(ApiError::Validation(
                "latitude and longitude must be within range".to_owned(),
            ));
        }
        if !value.accuracy_meters.is_finite() || !(0.0..=10_000.0).contains(&value.accuracy_meters)
        {
            return Err(ApiError::Validation(
                "accuracy_meters must be between 0 and 10000".to_owned(),
            ));
        }
        let captured_at = DateTime::parse_from_rfc3339(value.captured_at.trim())
            .map_err(|_| {
                ApiError::Validation("captured_at must be an RFC 3339 timestamp".to_owned())
            })?
            .with_timezone(&Utc);
        let current_time = Utc::now();
        if captured_at < current_time - MAX_LOCATION_REPORT_AGE {
            return Err(ApiError::Validation("captured_at is too old".to_owned()));
        }
        if captured_at > current_time + MAX_LOCATION_REPORT_FUTURE_SKEW {
            return Err(ApiError::Validation(
                "captured_at cannot be too far in the future".to_owned(),
            ));
        }
        Ok(Self {
            latitude: value.latitude,
            longitude: value.longitude,
            accuracy_meters: value.accuracy_meters,
            captured_at,
        })
    }
}

impl TryFrom<SubmitTaskFeedbackRequest> for ValidatedTaskFeedbackRequest {
    type Error = ApiError;

    fn try_from(value: SubmitTaskFeedbackRequest) -> Result<Self, Self::Error> {
        let content = required_field("content", value.content, 4_000)?;
        let occurred_at = match optional_field("occurred_at", value.occurred_at, 64)? {
            Some(occurred_at) => Some(
                DateTime::parse_from_rfc3339(&occurred_at)
                    .map_err(|_| {
                        ApiError::Validation("occurred_at must be an RFC 3339 timestamp".to_owned())
                    })?
                    .with_timezone(&Utc)
                    .to_rfc3339_opts(SecondsFormat::Millis, true),
            ),
            None => None,
        };
        let location_text = optional_field("location_text", value.location_text, 500)?;
        let location_precision =
            optional_field("location_precision", value.location_precision, 16)?
                .map(|precision| precision.to_lowercase());
        if location_precision
            .as_deref()
            .is_some_and(|precision| !CLUE_LOCATION_PRECISIONS.contains(&precision))
        {
            return Err(ApiError::Validation(
                "location_precision is unsupported".to_owned(),
            ));
        }
        if location_precision.is_some() && location_text.is_none() {
            return Err(ApiError::Validation(
                "location_precision requires location_text".to_owned(),
            ));
        }
        if value.attachment_ids.len() > 10 {
            return Err(ApiError::Validation(
                "attachment_ids cannot contain more than 10 items".to_owned(),
            ));
        }
        let mut unique_ids = HashSet::new();
        let attachment_ids = value
            .attachment_ids
            .into_iter()
            .map(|attachment_id| attachment_id.trim().to_owned())
            .collect::<Vec<_>>();
        if attachment_ids
            .iter()
            .any(|attachment_id| attachment_id.is_empty() || !unique_ids.insert(attachment_id))
        {
            return Err(ApiError::Validation(
                "attachment_ids must contain unique non-empty IDs".to_owned(),
            ));
        }
        Ok(Self {
            content,
            occurred_at,
            location_text,
            location_precision,
            attachment_ids,
        })
    }
}

impl ValidatedTaskListQuery {
    fn offset(&self) -> Result<usize, ApiError> {
        let offset = self
            .page
            .checked_sub(1)
            .and_then(|page| page.checked_mul(self.page_size))
            .ok_or_else(|| ApiError::Validation("page is too large".to_owned()))?;
        usize::try_from(offset).map_err(|_| ApiError::Validation("page is too large".to_owned()))
    }

    fn page_size_usize(&self) -> usize {
        self.page_size as usize
    }
}

impl TryFrom<TaskListQuery> for ValidatedTaskListQuery {
    type Error = ApiError;

    fn try_from(value: TaskListQuery) -> Result<Self, Self::Error> {
        let page = value.page.unwrap_or(1);
        let page_size = value.page_size.unwrap_or(25);
        if page == 0 {
            return Err(ApiError::Validation("page must be at least 1".to_owned()));
        }
        if page_size == 0 || page_size > MAX_TASK_PAGE_SIZE {
            return Err(ApiError::Validation(format!(
                "page_size must be between 1 and {MAX_TASK_PAGE_SIZE}"
            )));
        }
        Ok(Self { page, page_size })
    }
}

fn required_field(label: &str, value: String, maximum: usize) -> Result<String, ApiError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > maximum {
        return Err(ApiError::Validation(format!(
            "{label} must contain between 1 and {maximum} characters"
        )));
    }
    Ok(value.to_owned())
}

fn optional_field(
    label: &str,
    value: Option<String>,
    maximum: usize,
) -> Result<Option<String>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > maximum {
        return Err(ApiError::Validation(format!(
            "{label} must contain at most {maximum} characters"
        )));
    }
    Ok(Some(value.to_owned()))
}

fn validate_coordinates(latitude: Option<f64>, longitude: Option<f64>) -> Result<(), ApiError> {
    match (latitude, longitude) {
        (None, None) => Ok(()),
        (Some(latitude), Some(longitude))
            if latitude.is_finite()
                && longitude.is_finite()
                && (-90.0..=90.0).contains(&latitude)
                && (-180.0..=180.0).contains(&longitude) =>
        {
            Ok(())
        }
        _ => Err(ApiError::Validation(
            "latitude and longitude must be provided together and be within range".to_owned(),
        )),
    }
}

fn parse_due_at(value: &str) -> Result<String, ApiError> {
    let due_at = DateTime::parse_from_rfc3339(value.trim())
        .map_err(|_| ApiError::Validation("due_at must be an RFC 3339 timestamp".to_owned()))?
        .with_timezone(&Utc);
    if due_at <= Utc::now() {
        return Err(ApiError::Validation(
            "due_at must be in the future".to_owned(),
        ));
    }
    Ok(due_at.to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn volunteer_transition_allowed(current: &str, next: &str) -> bool {
    matches!(
        (current, next),
        ("assigned", "accepted")
            | ("accepted", "active")
            | ("active", "blocked")
            | ("blocked", "active")
            | ("active", "completed")
    )
}

fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "cancelled")
}

fn now() -> String {
    format_timestamp(Utc::now())
}

fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::Millis, true)
}
