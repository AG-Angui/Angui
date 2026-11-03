//! Canonical positions, written in the owning clue/task transaction.
use crate::{
    entities::{clues, locations, tasks},
    error::ApiError,
    models::{AuthenticatedUser, LocationResponse, TaskResponse},
    roles::CaseRole,
    services::case_service::{new_id, require_case_role},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct LocationInput {
    pub text: Option<String>,
    pub precision: Option<String>,
    pub kind: Option<String>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub radius_meters: Option<f64>,
}

pub(crate) async fn resolve<C: ConnectionTrait>(
    db: &C,
    auth: &AuthenticatedUser,
    case_id: &str,
    location_id: Option<&str>,
    input: LocationInput,
    timestamp: &str,
) -> Result<Option<locations::Model>, ApiError> {
    if let Some(id) = location_id {
        let location = locations::Entity::find_by_id(id)
            .filter(locations::Column::CaseId.eq(case_id))
            .filter(locations::Column::Status.eq("active"))
            .one(db)
            .await?
            .ok_or_else(|| ApiError::NotFound("location was not found".to_owned()))?;
        if !can_reference(db, auth, case_id, id).await? {
            return Err(ApiError::NotFound("location was not found".to_owned()));
        }
        let conflict = input
            .text
            .as_ref()
            .is_some_and(|v| !v.trim().is_empty() && v.trim() != location.location_text)
            || input
                .precision
                .as_ref()
                .is_some_and(|v| v.trim() != location.location_precision)
            || input
                .kind
                .as_ref()
                .is_some_and(|v| Some(v.trim()) != location.location_kind.as_deref())
            || input
                .longitude
                .is_some_and(|v| Some(v) != location.longitude)
            || input.latitude.is_some_and(|v| Some(v) != location.latitude)
            || input
                .radius_meters
                .is_some_and(|v| Some(v) != location.location_radius_meters);
        if conflict {
            return Err(ApiError::Validation(
                "location_id conflicts with inline location fields".to_owned(),
            ));
        }
        return Ok(Some(location));
    }
    if !input
        .text
        .as_ref()
        .is_some_and(|text| !text.trim().is_empty())
        && input.longitude.is_none()
        && input.latitude.is_none()
        && input.kind.is_none()
        && input.radius_meters.is_none()
    {
        return Ok(None);
    }
    Ok(Some(
        locations::ActiveModel {
            id: Set(new_id()),
            case_id: Set(case_id.to_owned()),
            location_text: Set(input.text.unwrap_or_default()),
            location_precision: Set(input.precision.unwrap_or_else(|| "unknown".to_owned())),
            location_kind: Set(input.kind),
            longitude: Set(input.longitude),
            latitude: Set(input.latitude),
            location_radius_meters: Set(input.radius_meters),
            status: Set("active".to_owned()),
            merged_into_id: Set(None),
            created_by_user_id: Set(Some(auth.id.clone())),
            created_at: Set(timestamp.to_owned()),
            updated_at: Set(timestamp.to_owned()),
        }
        .insert(db)
        .await?,
    ))
}

async fn can_reference<C: ConnectionTrait>(
    db: &C,
    auth: &AuthenticatedUser,
    case_id: &str,
    id: &str,
) -> Result<bool, ApiError> {
    let role = require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Commander, CaseRole::Family, CaseRole::Volunteer],
    )
    .await?;
    if role == CaseRole::Commander {
        return Ok(true);
    }
    let linked = clues::Entity::find()
        .filter(clues::Column::CaseId.eq(case_id))
        .filter(clues::Column::LocationId.eq(id))
        .all(db)
        .await?;
    if linked.iter().any(|clue| {
        clue.created_by_user_id.as_deref() == Some(auth.id.as_str())
            || (clue.status == "confirmed" && clue.visibility.as_deref() != Some("internal"))
    }) {
        return Ok(true);
    }
    if role != CaseRole::Volunteer {
        return Ok(false);
    }
    let linked_task = tasks::Entity::find()
        .filter(tasks::Column::CaseId.eq(case_id))
        .filter(tasks::Column::LocationId.eq(id))
        .one(db)
        .await?;
    Ok(linked_task.is_some())
}

async fn positions<C: ConnectionTrait>(
    db: &C,
    ids: Vec<String>,
) -> Result<HashMap<String, locations::Model>, ApiError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(locations::Entity::find()
        .filter(locations::Column::Id.is_in(ids))
        .all(db)
        .await?
        .into_iter()
        .map(|position| (position.id.clone(), position))
        .collect())
}

pub(crate) async fn hydrate_clues<C: ConnectionTrait>(
    db: &C,
    models: &mut [clues::Model],
) -> Result<(), ApiError> {
    let positions = positions(
        db,
        models
            .iter()
            .filter_map(|clue| clue.location_id.clone())
            .collect(),
    )
    .await?;
    for clue in models {
        if let Some(id) = clue.location_id.as_ref() {
            let position = positions
                .get(id)
                .filter(|position| position.case_id == clue.case_id)
                .ok_or(ApiError::Internal)?;
            clue.location_text =
                (!position.location_text.is_empty()).then(|| position.location_text.clone());
            clue.location_precision = Some(position.location_precision.clone());
            clue.location_kind = position.location_kind.clone();
            clue.longitude = position.longitude;
            clue.latitude = position.latitude;
            clue.location_radius_meters = position.location_radius_meters;
        }
    }
    Ok(())
}

pub(crate) async fn hydrate_tasks<C: ConnectionTrait>(
    db: &C,
    models: &mut [tasks::Model],
) -> Result<(), ApiError> {
    let positions = positions(
        db,
        models
            .iter()
            .filter_map(|task| task.location_id.clone())
            .collect(),
    )
    .await?;
    for task in models {
        if let Some(id) = task.location_id.as_ref() {
            let position = positions
                .get(id)
                .filter(|position| position.case_id == task.case_id)
                .ok_or(ApiError::Internal)?;
            task.area_text = position.location_text.clone();
            task.longitude = position.longitude;
            task.latitude = position.latitude;
        }
    }
    Ok(())
}

pub(crate) async fn hydrate_task_responses<C: ConnectionTrait>(
    db: &C,
    responses: &mut [TaskResponse],
) -> Result<(), ApiError> {
    let positions = positions(
        db,
        responses
            .iter()
            .filter_map(|task| task.location_id.clone())
            .collect(),
    )
    .await?;
    for task in responses {
        if let Some(id) = task.location_id.as_ref() {
            let position = positions
                .get(id)
                .filter(|position| position.case_id == task.case_id)
                .ok_or(ApiError::Internal)?;
            task.area_text = position.location_text.clone();
            task.longitude = position.longitude;
            task.latitude = position.latitude;
            task.location = Some(LocationResponse {
                id: position.id.clone(),
                case_id: position.case_id.clone(),
                location_text: (!position.location_text.is_empty())
                    .then(|| position.location_text.clone()),
                location_precision: Some(position.location_precision.clone()),
                location_kind: position.location_kind.clone(),
                longitude: position.longitude,
                latitude: position.latitude,
                location_radius_meters: position.location_radius_meters,
            });
        }
    }
    Ok(())
}

/// Project through existing authorized responses, never raw case locations.
pub async fn list(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<Vec<LocationResponse>, ApiError> {
    let detail = crate::services::case_service::get_case(db, auth, case_id).await?;
    let role = detail.access_role;
    let mut visible: Vec<_> = detail
        .clues
        .into_iter()
        .filter_map(|clue| clue.location)
        .collect();
    if role != CaseRole::Family {
        visible.extend(
            crate::services::task_service::list_all_visible_tasks(db, auth, case_id)
                .await?
                .into_iter()
                .filter_map(|task| task.location),
        );
    }
    let mut seen = HashSet::new();
    visible.retain(|location| seen.insert(location.id.clone()));
    if role == CaseRole::Family {
        for location in &mut visible {
            location.longitude = None;
            location.latitude = None;
            location.location_radius_meters = None;
            location.location_precision = Some("unknown".to_owned());
        }
    }
    visible.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(visible)
}
