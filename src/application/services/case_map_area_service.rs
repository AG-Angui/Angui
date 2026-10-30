use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde_json::json;

use crate::{
    entities::{case_map_area_vertices, case_map_areas},
    error::ApiError,
    models::{
        AuthenticatedUser, CaseMapAreaResponse, CreateCaseMapAreaRequest, MapAreaVertexResponse,
        ReviewCaseMapAreaRequest, UpdateCaseMapAreaRequest,
    },
    roles::CaseRole,
    services::case_service::{new_id, require_case_role, write_audit},
};

const TYPES: &[&str] = &["search", "searched", "risk", "rally"];

pub async fn list(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<Vec<CaseMapAreaResponse>, ApiError> {
    let role = require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Commander, CaseRole::Volunteer, CaseRole::Family],
    )
    .await?;
    let mut query =
        case_map_areas::Entity::find().filter(case_map_areas::Column::CaseId.eq(case_id));
    if role != CaseRole::Commander {
        query = query.filter(case_map_areas::Column::Status.eq("approved"));
    }
    let areas = query
        .order_by_desc(case_map_areas::Column::UpdatedAt)
        .all(db)
        .await?;
    let mut result = Vec::with_capacity(areas.len());
    for area in areas {
        let vertices = case_map_area_vertices::Entity::find()
            .filter(case_map_area_vertices::Column::AreaId.eq(&area.id))
            .order_by_asc(case_map_area_vertices::Column::VertexOrder)
            .all(db)
            .await?
            .into_iter()
            .map(|v| MapAreaVertexResponse {
                vertex_order: v.vertex_order,
                latitude: v.latitude,
                longitude: v.longitude,
            })
            .collect();
        result.push(response(
            area,
            if role == CaseRole::Family {
                Vec::new()
            } else {
                vertices
            },
        ));
    }
    Ok(result)
}

pub async fn create(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: CreateCaseMapAreaRequest,
) -> Result<CaseMapAreaResponse, ApiError> {
    validate(&request.area_type, &request.title, &request.vertices)?;
    let role = require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let area = case_map_areas::ActiveModel {
        id: Set(new_id()),
        case_id: Set(case_id.to_owned()),
        area_type: Set(request.area_type),
        status: Set(if role == CaseRole::Commander {
            "approved"
        } else {
            "candidate"
        }
        .to_owned()),
        title: Set(request.title.trim().to_owned()),
        description: Set(request.description),
        task_id: Set(request.task_id),
        clue_id: Set(request.clue_id),
        version: Set(1),
        created_by_user_id: Set(auth.id.clone()),
        reviewed_by_user_id: Set((role == CaseRole::Commander).then(|| auth.id.clone())),
        reviewed_at: Set((role == CaseRole::Commander).then(|| now.clone())),
        starts_at: Set(request.starts_at),
        ends_at: Set(request.ends_at),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;
    for (order, vertex) in request.vertices.into_iter().enumerate() {
        case_map_area_vertices::ActiveModel {
            area_id: Set(area.id.clone()),
            vertex_order: Set(order as i32),
            latitude: Set(vertex.latitude),
            longitude: Set(vertex.longitude),
        }
        .insert(db)
        .await?;
    }
    let vertices = case_map_area_vertices::Entity::find()
        .filter(case_map_area_vertices::Column::AreaId.eq(&area.id))
        .order_by_asc(case_map_area_vertices::Column::VertexOrder)
        .all(db)
        .await?
        .into_iter()
        .map(|v| MapAreaVertexResponse {
            vertex_order: v.vertex_order,
            latitude: v.latitude,
            longitude: v.longitude,
        })
        .collect();
    Ok(response(area, vertices))
}

pub async fn update(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    area_id: &str,
    request: UpdateCaseMapAreaRequest,
) -> Result<CaseMapAreaResponse, ApiError> {
    validate(&request.area_type, &request.title, &request.vertices)?;
    require_case_role(db, &auth.id, case_id, &[CaseRole::Commander]).await?;
    let tx = db.begin().await?;
    let area = case_map_areas::Entity::find_by_id(area_id)
        .one(&tx)
        .await?
        .ok_or_else(|| ApiError::NotFound("map area was not found".to_owned()))?;
    if area.case_id != case_id {
        return Err(ApiError::NotFound("map area was not found".to_owned()));
    }
    if area.version != request.expected_version {
        return Err(ApiError::Conflict(format!(
            "map area version is {}, expected {}",
            area.version, request.expected_version
        )));
    }
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut active = area.into_active_model();
    active.area_type = Set(request.area_type);
    active.title = Set(request.title.trim().to_owned());
    active.description = Set(request.description);
    active.task_id = Set(request.task_id);
    active.clue_id = Set(request.clue_id);
    active.version = Set(request.expected_version + 1);
    active.updated_at = Set(now);
    let area = active.update(&tx).await?;
    case_map_area_vertices::Entity::delete_many()
        .filter(case_map_area_vertices::Column::AreaId.eq(&area.id))
        .exec(&tx)
        .await?;
    for (order, vertex) in request.vertices.into_iter().enumerate() {
        case_map_area_vertices::ActiveModel {
            area_id: Set(area.id.clone()),
            vertex_order: Set(order as i32),
            latitude: Set(vertex.latitude),
            longitude: Set(vertex.longitude),
        }
        .insert(&tx)
        .await?;
    }
    tx.commit().await?;
    let vertices = case_map_area_vertices::Entity::find()
        .filter(case_map_area_vertices::Column::AreaId.eq(&area.id))
        .order_by_asc(case_map_area_vertices::Column::VertexOrder)
        .all(db)
        .await?
        .into_iter()
        .map(|v| MapAreaVertexResponse {
            vertex_order: v.vertex_order,
            latitude: v.latitude,
            longitude: v.longitude,
        })
        .collect();
    Ok(response(area, vertices))
}

pub async fn review(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    area_id: &str,
    request: ReviewCaseMapAreaRequest,
) -> Result<CaseMapAreaResponse, ApiError> {
    require_case_role(db, &auth.id, case_id, &[CaseRole::Commander]).await?;
    if !matches!(request.action.as_str(), "approve" | "reject") || request.reason.trim().is_empty()
    {
        return Err(ApiError::Validation(
            "action must be approve or reject and reason is required".to_owned(),
        ));
    }
    let tx = db.begin().await?;
    let area = case_map_areas::Entity::find_by_id(area_id)
        .one(&tx)
        .await?
        .ok_or_else(|| ApiError::NotFound("map area was not found".to_owned()))?;
    if area.case_id != case_id {
        return Err(ApiError::NotFound("map area was not found".to_owned()));
    }
    if area.version != request.expected_version {
        return Err(ApiError::Conflict(format!(
            "map area version is {}, expected {}",
            area.version, request.expected_version
        )));
    }
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut active = area.into_active_model();
    active.status = Set(if request.action == "approve" {
        "approved"
    } else {
        "rejected"
    }
    .to_owned());
    active.version = Set(request.expected_version + 1);
    active.reviewed_by_user_id = Set(Some(auth.id.clone()));
    active.reviewed_at = Set(Some(now.clone()));
    active.updated_at = Set(now.clone());
    let area = active.update(&tx).await?;
    write_audit(
        &tx,
        Some(case_id.to_owned()),
        auth,
        "case_map_area.reviewed",
        "case_map_area",
        area.id.clone(),
        Some(json!({"action": request.action, "reason": request.reason})),
    )
    .await?;
    tx.commit().await?;
    let vertices = case_map_area_vertices::Entity::find()
        .filter(case_map_area_vertices::Column::AreaId.eq(&area.id))
        .order_by_asc(case_map_area_vertices::Column::VertexOrder)
        .all(db)
        .await?
        .into_iter()
        .map(|v| MapAreaVertexResponse {
            vertex_order: v.vertex_order,
            latitude: v.latitude,
            longitude: v.longitude,
        })
        .collect();
    Ok(response(area, vertices))
}

fn validate(
    area_type: &str,
    title: &str,
    vertices: &[crate::models::MapAreaVertexInput],
) -> Result<(), ApiError> {
    if !TYPES.contains(&area_type) {
        return Err(ApiError::Validation(
            "area_type must be search, searched, risk, or rally".to_owned(),
        ));
    }
    if title.trim().is_empty() || title.len() > 200 {
        return Err(ApiError::Validation(
            "title must be between 1 and 200 characters".to_owned(),
        ));
    }
    if vertices.len() < 3 {
        return Err(ApiError::Validation(
            "a polygon requires at least three vertices".to_owned(),
        ));
    }
    if vertices.len() > 200 {
        return Err(ApiError::Validation(
            "a polygon cannot contain more than 200 vertices".to_owned(),
        ));
    }
    if vertices
        .iter()
        .any(|v| !(-90.0..=90.0).contains(&v.latitude) || !(-180.0..=180.0).contains(&v.longitude))
    {
        return Err(ApiError::Validation(
            "polygon coordinates are invalid".to_owned(),
        ));
    }
    Ok(())
}

fn response(
    area: case_map_areas::Model,
    vertices: Vec<MapAreaVertexResponse>,
) -> CaseMapAreaResponse {
    CaseMapAreaResponse {
        id: area.id,
        case_id: area.case_id,
        area_type: area.area_type,
        status: area.status,
        title: area.title,
        description: area.description,
        task_id: area.task_id,
        clue_id: area.clue_id,
        version: area.version,
        vertices,
        starts_at: area.starts_at,
        ends_at: area.ends_at,
        created_at: area.created_at,
        updated_at: area.updated_at,
    }
}
