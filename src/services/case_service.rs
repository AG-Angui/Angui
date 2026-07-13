use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{audit_events, cases, clues, elder_profiles},
    error::ApiError,
    models::{
        CaseDetail, CaseListItem, ClueResponse, CreateCaseRequest, CreateClueRequest,
        ReviewClueRequest, UpdateCaseStatusRequest,
    },
};

const CASE_STATUSES: &[&str] = &["active", "resolved", "closed"];
const CLUE_REVIEW_STATUSES: &[&str] = &[
    "needs_verification",
    "confirmed",
    "rejected",
    "expired",
    "duplicate",
];

pub async fn list_cases(db: &DatabaseConnection) -> Result<Vec<CaseListItem>, ApiError> {
    let rows = cases::Entity::find()
        .find_also_related(elder_profiles::Entity)
        .order_by_desc(cases::Column::CreatedAt)
        .all(db)
        .await?;

    rows.into_iter()
        .map(|(case_model, profile)| {
            let profile = profile.ok_or_else(|| {
                ApiError::Database(sea_orm::DbErr::Custom(format!(
                    "case {} is missing its elder profile",
                    case_model.id
                )))
            })?;

            Ok(CaseListItem {
                id: case_model.id,
                case_code: case_model.case_code,
                status: case_model.status,
                display_name: profile.display_name,
                last_seen_at: profile.last_seen_at,
                last_seen_location: profile.last_seen_location,
                created_at: case_model.created_at,
                updated_at: case_model.updated_at,
            })
        })
        .collect()
}

pub async fn get_case(db: &DatabaseConnection, case_id: &str) -> Result<CaseDetail, ApiError> {
    let case_model = cases::Entity::find_by_id(case_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("case {case_id} was not found")))?;
    let profile = elder_profiles::Entity::find()
        .filter(elder_profiles::Column::CaseId.eq(case_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            ApiError::Database(sea_orm::DbErr::Custom("elder profile missing".into()))
        })?;
    let clue_models = clues::Entity::find()
        .filter(clues::Column::CaseId.eq(case_id))
        .order_by_desc(clues::Column::CreatedAt)
        .all(db)
        .await?;

    Ok(CaseDetail::new(case_model, profile, clue_models))
}

pub async fn create_case(
    db: &DatabaseConnection,
    request: CreateCaseRequest,
) -> Result<CaseDetail, ApiError> {
    validate_case_request(&request)?;

    let transaction = db.begin().await?;
    let now = now();
    let case_id = new_id();
    let profile_id = new_id();
    let case_code = format!("AG-{}", case_id[..8].to_uppercase());

    cases::ActiveModel {
        id: Set(case_id.clone()),
        case_code: Set(case_code),
        status: Set("active".to_owned()),
        created_at: Set(now.clone()),
        updated_at: Set(now.clone()),
    }
    .insert(&transaction)
    .await?;

    elder_profiles::ActiveModel {
        id: Set(profile_id),
        case_id: Set(case_id.clone()),
        display_name: Set(request.display_name.trim().to_owned()),
        age: Set(request.age),
        gender: Set(trim_optional(request.gender)),
        physical_description: Set(trim_optional(request.physical_description)),
        clothing_description: Set(trim_optional(request.clothing_description)),
        health_notes: Set(trim_optional(request.health_notes)),
        last_seen_at: Set(trim_optional(request.last_seen_at)),
        last_seen_location: Set(trim_optional(request.last_seen_location)),
        created_at: Set(now.clone()),
        updated_at: Set(now.clone()),
    }
    .insert(&transaction)
    .await?;

    write_audit(
        &transaction,
        Some(case_id.clone()),
        "demo:family",
        "case.created",
        "case",
        case_id.clone(),
        Some(json!({ "status": "active" })),
    )
    .await?;

    transaction.commit().await?;
    get_case(db, &case_id).await
}

pub async fn update_case_status(
    db: &DatabaseConnection,
    case_id: &str,
    request: UpdateCaseStatusRequest,
) -> Result<CaseDetail, ApiError> {
    let next_status = request.status.trim().to_lowercase();
    if !CASE_STATUSES.contains(&next_status.as_str()) {
        return Err(ApiError::Validation(format!(
            "unsupported case status {next_status:?}"
        )));
    }

    let transaction = db.begin().await?;
    let case_model = cases::Entity::find_by_id(case_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("case {case_id} was not found")))?;

    if !case_transition_allowed(&case_model.status, &next_status) {
        return Err(ApiError::Conflict(format!(
            "case status cannot change from {:?} to {:?}",
            case_model.status, next_status
        )));
    }

    let previous_status = case_model.status.clone();
    let mut active = case_model.into_active_model();
    active.status = Set(next_status.clone());
    active.updated_at = Set(now());
    active.update(&transaction).await?;

    write_audit(
        &transaction,
        Some(case_id.to_owned()),
        "demo:commander",
        "case.status_changed",
        "case",
        case_id.to_owned(),
        Some(json!({ "from": previous_status, "to": next_status })),
    )
    .await?;

    transaction.commit().await?;
    get_case(db, case_id).await
}

pub async fn create_clue(
    db: &DatabaseConnection,
    case_id: &str,
    request: CreateClueRequest,
) -> Result<ClueResponse, ApiError> {
    validate_clue_request(&request)?;
    let transaction = db.begin().await?;
    let case_model = cases::Entity::find_by_id(case_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("case {case_id} was not found")))?;

    if case_model.status == "closed" {
        return Err(ApiError::Conflict(
            "new clues cannot be added to a closed case".to_owned(),
        ));
    }

    let clue_id = new_id();
    let timestamp = now();
    let clue_model = clues::ActiveModel {
        id: Set(clue_id.clone()),
        case_id: Set(case_id.to_owned()),
        status: Set("pending_review".to_owned()),
        source: Set(request.source.trim().to_owned()),
        content: Set(request.content.trim().to_owned()),
        occurred_at: Set(trim_optional(request.occurred_at)),
        location_text: Set(trim_optional(request.location_text)),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp),
    }
    .insert(&transaction)
    .await?;

    write_audit(
        &transaction,
        Some(case_id.to_owned()),
        "demo:family",
        "clue.submitted",
        "clue",
        clue_id,
        Some(json!({ "status": "pending_review" })),
    )
    .await?;

    transaction.commit().await?;
    Ok(clue_model.into())
}

pub async fn review_clue(
    db: &DatabaseConnection,
    clue_id: &str,
    request: ReviewClueRequest,
) -> Result<ClueResponse, ApiError> {
    let next_status = request.status.trim().to_lowercase();
    if !CLUE_REVIEW_STATUSES.contains(&next_status.as_str()) {
        return Err(ApiError::Validation(format!(
            "unsupported reviewed clue status {next_status:?}"
        )));
    }

    let transaction = db.begin().await?;
    let clue_model = clues::Entity::find_by_id(clue_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("clue {clue_id} was not found")))?;
    let previous_status = clue_model.status.clone();
    let case_id = clue_model.case_id.clone();
    let mut active = clue_model.into_active_model();
    active.status = Set(next_status.clone());
    active.updated_at = Set(now());
    let updated = active.update(&transaction).await?;

    write_audit(
        &transaction,
        Some(case_id),
        "demo:commander",
        "clue.reviewed",
        "clue",
        clue_id.to_owned(),
        Some(json!({ "from": previous_status, "to": next_status })),
    )
    .await?;

    transaction.commit().await?;
    Ok(updated.into())
}

async fn write_audit<C: ConnectionTrait>(
    db: &C,
    case_id: Option<String>,
    actor: &str,
    action: &str,
    entity_type: &str,
    entity_id: String,
    metadata: Option<serde_json::Value>,
) -> Result<(), ApiError> {
    audit_events::ActiveModel {
        id: Set(new_id()),
        case_id: Set(case_id),
        actor: Set(actor.to_owned()),
        action: Set(action.to_owned()),
        entity_type: Set(entity_type.to_owned()),
        entity_id: Set(entity_id),
        metadata_json: Set(metadata.map(|value| value.to_string())),
        created_at: Set(now()),
    }
    .insert(db)
    .await?;

    Ok(())
}

fn validate_case_request(request: &CreateCaseRequest) -> Result<(), ApiError> {
    let name = request.display_name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(ApiError::Validation(
            "display_name must contain between 1 and 120 characters".to_owned(),
        ));
    }
    if request.age.is_some_and(|age| !(0..=130).contains(&age)) {
        return Err(ApiError::Validation(
            "age must be between 0 and 130".to_owned(),
        ));
    }
    if trim_optional(request.last_seen_location.clone()).is_none() {
        return Err(ApiError::Validation(
            "last_seen_location is required".to_owned(),
        ));
    }
    Ok(())
}

fn validate_clue_request(request: &CreateClueRequest) -> Result<(), ApiError> {
    if request.source.trim().is_empty() || request.source.chars().count() > 64 {
        return Err(ApiError::Validation(
            "source must contain between 1 and 64 characters".to_owned(),
        ));
    }
    if request.content.trim().is_empty() || request.content.chars().count() > 4000 {
        return Err(ApiError::Validation(
            "content must contain between 1 and 4000 characters".to_owned(),
        ));
    }
    Ok(())
}

fn case_transition_allowed(current: &str, next: &str) -> bool {
    current == next
        || matches!(
            (current, next),
            ("active", "resolved")
                | ("active", "closed")
                | ("resolved", "active")
                | ("resolved", "closed")
        )
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use migration::{Migrator, MigratorTrait};
    use sea_orm::{Database, EntityTrait, PaginatorTrait};

    use super::{create_case, create_clue, list_cases, review_clue, update_case_status};
    use crate::{
        entities::audit_events,
        models::{
            CreateCaseRequest, CreateClueRequest, ReviewClueRequest, UpdateCaseStatusRequest,
        },
    };

    async fn database() -> sea_orm::DatabaseConnection {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&database, None)
            .await
            .expect("migrations should succeed");
        database
    }

    #[actix_web::test]
    async fn case_and_clue_workflow_is_persisted_and_audited() {
        let database = database().await;

        let case = create_case(
            &database,
            CreateCaseRequest {
                display_name: "模拟老人 A".to_owned(),
                age: Some(76),
                gender: Some("female".to_owned()),
                physical_description: Some("短发，行动较慢".to_owned()),
                clothing_description: Some("蓝色外套".to_owned()),
                health_notes: Some("模拟认知障碍信息".to_owned()),
                last_seen_at: Some("2026-07-13T09:00:00Z".to_owned()),
                last_seen_location: Some("模拟公园北门".to_owned()),
            },
        )
        .await
        .expect("case creation should succeed");

        let cases = list_cases(&database)
            .await
            .expect("case listing should succeed");
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].display_name, "模拟老人 A");

        let clue = create_clue(
            &database,
            &case.id,
            CreateClueRequest {
                source: "family".to_owned(),
                content: "模拟线索：曾向市场方向步行".to_owned(),
                occurred_at: Some("2026-07-13T09:10:00Z".to_owned()),
                location_text: Some("模拟公园北门".to_owned()),
            },
        )
        .await
        .expect("clue creation should succeed");
        assert_eq!(clue.status, "pending_review");

        let reviewed = review_clue(
            &database,
            &clue.id,
            ReviewClueRequest {
                status: "confirmed".to_owned(),
            },
        )
        .await
        .expect("clue review should succeed");
        assert_eq!(reviewed.status, "confirmed");

        let resolved = update_case_status(
            &database,
            &case.id,
            UpdateCaseStatusRequest {
                status: "resolved".to_owned(),
            },
        )
        .await
        .expect("case status update should succeed");
        assert_eq!(resolved.status, "resolved");

        let audit_count = audit_events::Entity::find()
            .count(&database)
            .await
            .expect("audit count should succeed");
        assert_eq!(audit_count, 4);
    }
}
