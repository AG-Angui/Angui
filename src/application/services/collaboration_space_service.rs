use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use actix_web::web;
use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    entities::{
        collaboration_spaces, event_outbox, space_events, space_location_consents,
        space_location_samples, space_member_slots, space_members, space_messages, users,
        voice_reports, voice_transcripts,
    },
    error::ApiError,
    models::{
        AuthenticatedUser, CollaborationSpaceResponse, CollaborationSpaceSnapshotResponse,
        CreateCollaborationSpaceRequest, CreateSpaceMessageRequest, JoinCollaborationSpaceRequest,
        RecordSpaceLocationRequest, SpaceEventResponse, SpaceLocationResponse, SpaceMemberResponse,
        SpaceMessageResponse, VoiceReportResponse, VoiceTranscriptResponse,
    },
    roles::{CaseRole, GlobalCapability},
    services::case_service::{new_id, require_case_role, write_audit},
};

const MAX_ACTIVE_SPACES_PER_VOLUNTEER: i32 = 3;
/// A transport cap, independent of any retention policy. Audio remains private
/// and is never routed through the generic case-attachment endpoint.
pub const MAX_VOICE_REPORT_BYTES: usize = 10 * 1024 * 1024;

pub async fn create_space(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
    request: CreateCollaborationSpaceRequest,
) -> Result<CollaborationSpaceResponse, ApiError> {
    let name = validated_name(request.name)?;
    let transaction = db.begin().await?;
    require_case_role(&transaction, &auth.id, case_id, &[CaseRole::Commander]).await?;
    if collaboration_spaces::Entity::find()
        .filter(collaboration_spaces::Column::CaseId.eq(case_id))
        .filter(collaboration_spaces::Column::Status.eq("active"))
        .one(&transaction)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict(
            "the case already has an active collaboration space".to_owned(),
        ));
    }

    let timestamp = now();
    let space = collaboration_spaces::ActiveModel {
        id: Set(new_id()),
        case_id: Set(case_id.to_owned()),
        name: Set(name),
        status: Set("active".to_owned()),
        created_by_user_id: Set(auth.id.clone()),
        created_at: Set(timestamp.clone()),
        archived_at: Set(None),
        next_event_version: Set(0),
    }
    .insert(&transaction)
    .await?;

    let member = insert_member(&transaction, &space, auth, CaseRole::Commander, &timestamp).await?;
    publish_event(
        &transaction,
        &space,
        "space.member_joined",
        "space_members",
        json!({"user_id": auth.id, "role": "commander"}),
        &timestamp,
    )
    .await?;
    write_audit(
        &transaction,
        Some(case_id.to_owned()),
        auth,
        "collaboration_space.created",
        "collaboration_space",
        space.id.clone(),
        Some(json!({"member_id": member.id, "status": "active"})),
    )
    .await?;
    transaction.commit().await?;
    Ok(space_response(space, Some("active".to_owned())))
}

pub async fn list_case_spaces(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<Vec<CollaborationSpaceResponse>, ApiError> {
    let role = require_case_role(
        db,
        &auth.id,
        case_id,
        &[CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let spaces = collaboration_spaces::Entity::find()
        .filter(collaboration_spaces::Column::CaseId.eq(case_id))
        .order_by_desc(collaboration_spaces::Column::CreatedAt)
        .all(db)
        .await?;
    let memberships: HashMap<String, String> = if role == CaseRole::Volunteer {
        space_members::Entity::find()
            .filter(space_members::Column::UserId.eq(&auth.id))
            .all(db)
            .await?
            .into_iter()
            .map(|member| (member.space_id, member.status))
            .collect()
    } else {
        Default::default()
    };
    Ok(spaces
        .into_iter()
        .filter_map(|space| {
            let status = memberships.get(&space.id).cloned();
            (role == CaseRole::Commander || space.status == "active")
                .then(|| space_response(space, status))
        })
        .collect())
}

/// Archives a collaboration space without deleting its operational history.
/// Case commanders may archive spaces in their case; administrators may do so
/// for any space. Archived spaces remain readable to authorized commanders so
/// messages, locations, events, and audit records remain reviewable.
pub async fn archive_space(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
) -> Result<CollaborationSpaceResponse, ApiError> {
    let transaction = db.begin().await?;
    let space = collaboration_spaces::Entity::find_by_id(space_id)
        .one(&transaction)
        .await?
        .ok_or_else(|| ApiError::NotFound("collaboration space was not found".to_owned()))?;
    if !auth.global_capabilities.contains(&GlobalCapability::Admin) {
        require_case_role(
            &transaction,
            &auth.id,
            &space.case_id,
            &[CaseRole::Commander],
        )
        .await?;
    }
    if space.status == "archived" {
        transaction.commit().await?;
        return Ok(space_response(space, None));
    }
    let timestamp = now();
    let mut archived = space.clone().into_active_model();
    archived.status = Set("archived".to_owned());
    archived.archived_at = Set(Some(timestamp.clone()));
    let archived = archived.update(&transaction).await?;
    publish_event(
        &transaction,
        &archived,
        "space.archived",
        "space_members",
        json!({"archived_by_user_id": auth.id}),
        &timestamp,
    )
    .await?;
    write_audit(
        &transaction,
        Some(space.case_id.clone()),
        auth,
        "collaboration_space.archived",
        "collaboration_space",
        space.id.clone(),
        Some(json!({"status": "archived", "archived_at": timestamp})),
    )
    .await?;
    transaction.commit().await?;
    Ok(space_response(archived, None))
}

pub async fn get_snapshot(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
) -> Result<CollaborationSpaceSnapshotResponse, ApiError> {
    let (space, own_member) = require_space_access(db, auth, space_id).await?;
    let members = space_members::Entity::find()
        .filter(space_members::Column::SpaceId.eq(space_id))
        .order_by_asc(space_members::Column::JoinedAt)
        .all(db)
        .await?;
    let mut responses = Vec::with_capacity(members.len());
    for member in members {
        let user = users::Entity::find_by_id(&member.user_id)
            .one(db)
            .await?
            .ok_or(ApiError::Internal)?;
        let consent = space_location_consents::Entity::find()
            .filter(space_location_consents::Column::MemberId.eq(&member.id))
            .one(db)
            .await?;
        responses.push(SpaceMemberResponse {
            id: member.id,
            user_id: member.user_id,
            display_name: user.display_name,
            role: member.role,
            status: member.status,
            joined_at: member.joined_at,
            left_at: member.left_at,
            location_consent_granted: consent.is_some_and(|item| item.revoked_at.is_none()),
        });
    }
    Ok(CollaborationSpaceSnapshotResponse {
        space: space_response(space.clone(), own_member.map(|member| member.status)),
        members: responses,
        version: space.next_event_version,
    })
}

pub async fn join_space(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
    request: JoinCollaborationSpaceRequest,
) -> Result<CollaborationSpaceResponse, ApiError> {
    let transaction = db.begin().await?;
    let space = active_space(&transaction, space_id).await?;
    let role = require_case_role(
        &transaction,
        &auth.id,
        &space.case_id,
        &[CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    if role == CaseRole::Volunteer
        && (!request.location_consent || request.consent_version.is_none())
    {
        return Err(ApiError::Validation(
            "volunteers must explicitly grant the current space location consent before joining"
                .to_owned(),
        ));
    }
    let timestamp = now();
    let member = match space_members::Entity::find()
        .filter(space_members::Column::SpaceId.eq(space_id))
        .filter(space_members::Column::UserId.eq(&auth.id))
        .one(&transaction)
        .await?
    {
        Some(member) if member.status == "active" => member,
        Some(member) => {
            let mut active = member.into_active_model();
            active.status = Set("active".to_owned());
            active.joined_at = Set(timestamp.clone());
            active.left_at = Set(None);
            let member = active.update(&transaction).await?;
            if role == CaseRole::Volunteer {
                allocate_slot(&transaction, &member).await?;
            }
            member
        }
        None => insert_member(&transaction, &space, auth, role, &timestamp).await?,
    };
    if role == CaseRole::Volunteer {
        grant_consent(
            &transaction,
            &space,
            &member,
            request.consent_version.as_deref().unwrap_or_default(),
            &timestamp,
        )
        .await?;
    }
    publish_event(
        &transaction,
        &space,
        "space.member_joined",
        "space_members",
        json!({"user_id": auth.id, "role": role.as_str()}),
        &timestamp,
    )
    .await?;
    write_audit(
        &transaction,
        Some(space.case_id.clone()),
        auth,
        "collaboration_space.joined",
        "collaboration_space",
        space.id.clone(),
        None,
    )
    .await?;
    transaction.commit().await?;
    Ok(space_response(space, Some("active".to_owned())))
}

pub async fn leave_space(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
) -> Result<(), ApiError> {
    let transaction = db.begin().await?;
    let space = active_space(&transaction, space_id).await?;
    let member = active_member(&transaction, space_id, &auth.id).await?;
    let timestamp = now();
    let mut updated = member.clone().into_active_model();
    updated.status = Set("left".to_owned());
    updated.left_at = Set(Some(timestamp.clone()));
    updated.update(&transaction).await?;
    space_member_slots::Entity::delete_many()
        .filter(space_member_slots::Column::MemberId.eq(&member.id))
        .exec(&transaction)
        .await?;
    revoke_consent(&transaction, &member.id, &timestamp).await?;
    publish_event(
        &transaction,
        &space,
        "space.member_left",
        "space_members",
        json!({"user_id": auth.id}),
        &timestamp,
    )
    .await?;
    write_audit(
        &transaction,
        Some(space.case_id.clone()),
        auth,
        "collaboration_space.left",
        "collaboration_space",
        space.id.clone(),
        None,
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn grant_location_consent(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
    consent_version: String,
) -> Result<(), ApiError> {
    let consent_version = validated_consent_version(consent_version)?;
    let transaction = db.begin().await?;
    let space = active_space(&transaction, space_id).await?;
    require_case_role(
        &transaction,
        &auth.id,
        &space.case_id,
        &[CaseRole::Volunteer],
    )
    .await?;
    let member = active_member(&transaction, space_id, &auth.id).await?;
    let timestamp = now();
    grant_consent(&transaction, &space, &member, &consent_version, &timestamp).await?;
    publish_event(
        &transaction,
        &space,
        "space.location_consent_granted",
        "space_members",
        json!({"user_id": auth.id}),
        &timestamp,
    )
    .await?;
    write_audit(
        &transaction,
        Some(space.case_id.clone()),
        auth,
        "collaboration_space.location_consent_granted",
        "collaboration_space",
        space.id.clone(),
        None,
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn revoke_location_consent(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
) -> Result<(), ApiError> {
    let transaction = db.begin().await?;
    let space = active_space(&transaction, space_id).await?;
    require_case_role(
        &transaction,
        &auth.id,
        &space.case_id,
        &[CaseRole::Volunteer],
    )
    .await?;
    let member = active_member(&transaction, space_id, &auth.id).await?;
    let timestamp = now();
    revoke_consent(&transaction, &member.id, &timestamp).await?;
    publish_event(
        &transaction,
        &space,
        "space.location_consent_revoked",
        "space_members",
        json!({"user_id": auth.id}),
        &timestamp,
    )
    .await?;
    write_audit(
        &transaction,
        Some(space.case_id.clone()),
        auth,
        "collaboration_space.location_consent_revoked",
        "collaboration_space",
        space.id.clone(),
        None,
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn list_events(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
    after_version: i32,
) -> Result<Vec<SpaceEventResponse>, ApiError> {
    if after_version < 0 {
        return Err(ApiError::Validation(
            "after_version cannot be negative".to_owned(),
        ));
    }
    let (space, own_member) = require_space_access(db, auth, space_id).await?;
    let is_commander = own_member
        .as_ref()
        .is_some_and(|member| member.role == "commander")
        || require_case_role(db, &auth.id, &space.case_id, &[CaseRole::Commander])
            .await
            .is_ok();
    let own_member_id = own_member.map(|member| member.user_id);
    space_events::Entity::find()
        .filter(space_events::Column::SpaceId.eq(space_id))
        .filter(space_events::Column::Version.gt(after_version))
        .order_by_asc(space_events::Column::Version)
        .all(db)
        .await?
        .into_iter()
        .filter(|event| {
            event.visibility_scope == "space_members"
                || (event.visibility_scope == "commanders" && is_commander)
                || (event.visibility_scope == "self"
                    && own_member_id
                        .as_ref()
                        .is_some_and(|id| event_targets_user(event, id)))
        })
        .map(event_response)
        .collect()
}

/// Persists only an authorized volunteer's latest sample. Callers must provide
/// an operation id so a reconnect cannot create a second sample or event.
pub async fn record_location(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
    request: RecordSpaceLocationRequest,
) -> Result<SpaceLocationResponse, ApiError> {
    validate_location(&request)?;
    if !location_retention_is_configured() {
        return Err(ApiError::Conflict(
            "location storage is disabled until an operator configures retention".to_owned(),
        ));
    }
    let transaction = db.begin().await?;
    let space = active_space(&transaction, space_id).await?;
    let role = require_case_role(
        &transaction,
        &auth.id,
        &space.case_id,
        &[CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    let member = active_member(&transaction, space_id, &auth.id).await?;
    let consent = space_location_consents::Entity::find()
        .filter(space_location_consents::Column::MemberId.eq(&member.id))
        .one(&transaction)
        .await?;
    if role == CaseRole::Volunteer && !consent.is_some_and(|item| item.revoked_at.is_none()) {
        return Err(ApiError::Forbidden(
            "an active location-sharing consent is required".to_owned(),
        ));
    }
    if let Some(existing) = space_location_samples::Entity::find()
        .filter(space_location_samples::Column::OperationId.eq(&request.operation_id))
        .one(&transaction)
        .await?
    {
        if existing.space_id == space.id && existing.user_id == auth.id {
            return location_response(existing);
        }
        return Err(ApiError::Conflict(
            "location operation id was already used".to_owned(),
        ));
    }
    let timestamp = now();
    let sample = space_location_samples::ActiveModel {
        id: Set(new_id()),
        space_id: Set(space.id.clone()),
        user_id: Set(auth.id.clone()),
        latitude: Set(request.latitude),
        longitude: Set(request.longitude),
        accuracy_meters: Set(request.accuracy_meters),
        captured_at: Set(request.captured_at),
        operation_id: Set(request.operation_id),
        created_at: Set(timestamp.clone()),
    }
    .insert(&transaction)
    .await?;
    publish_event(
        &transaction,
        &space,
        "member.location_updated",
        "space_members",
        json!({
            "user_id": auth.id, "latitude": sample.latitude, "longitude": sample.longitude,
            "accuracy_meters": sample.accuracy_meters, "captured_at": sample.captured_at,
        }),
        &timestamp,
    )
    .await?;
    write_audit(
        &transaction,
        Some(space.case_id.clone()),
        auth,
        "collaboration_space.location_recorded",
        "collaboration_space",
        space.id.clone(),
        Some(json!({"sample_id": sample.id})),
    )
    .await?;
    transaction.commit().await?;
    location_response(sample)
}

pub async fn list_member_locations(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
    user_id: &str,
) -> Result<Vec<SpaceLocationResponse>, ApiError> {
    let (_space, own_member) = require_space_access(db, auth, space_id).await?;
    if own_member
        .as_ref()
        .is_some_and(|member| member.role == "volunteer")
        && auth.id != user_id
    {
        // A volunteer can read current room locations, but historical tracks are
        // intentionally commander-only until an explicit retention policy exists.
        return Err(ApiError::Forbidden(
            "trajectory history is commander-only".to_owned(),
        ));
    }
    space_location_samples::Entity::find()
        .filter(space_location_samples::Column::SpaceId.eq(space_id))
        .filter(space_location_samples::Column::UserId.eq(user_id))
        .order_by_desc(space_location_samples::Column::CapturedAt)
        .limit(200)
        .all(db)
        .await?
        .into_iter()
        .map(location_response)
        .collect::<Result<Vec<_>, _>>()
}

/// Returns the most recent location sample for each member. This endpoint is
/// intentionally limited to one sample per user so volunteers can see the
/// current team picture without receiving historical trajectories.
pub async fn list_latest_locations(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
) -> Result<Vec<SpaceLocationResponse>, ApiError> {
    let (_space, _member) = require_space_access(db, auth, space_id).await?;
    let samples = space_location_samples::Entity::find()
        .filter(space_location_samples::Column::SpaceId.eq(space_id))
        .order_by_desc(space_location_samples::Column::CapturedAt)
        .limit(500)
        .all(db)
        .await?;
    let mut latest = HashMap::new();
    for sample in samples {
        latest.entry(sample.user_id.clone()).or_insert(sample);
    }
    latest.into_values().map(location_response).collect()
}

pub async fn create_message(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
    request: CreateSpaceMessageRequest,
) -> Result<SpaceMessageResponse, ApiError> {
    let content = request.content.trim();
    if content.is_empty() || content.chars().count() > 2_000 {
        return Err(ApiError::Validation(
            "message content must contain between 1 and 2000 characters".to_owned(),
        ));
    }
    let transaction = db.begin().await?;
    let space = active_space(&transaction, space_id).await?;
    let member = active_member(&transaction, space_id, &auth.id).await?;
    let message_type = request.message_type.unwrap_or_else(|| "text".to_owned());
    if !matches!(message_type.as_str(), "text" | "broadcast") {
        return Err(ApiError::Validation(
            "message_type must be text or broadcast".to_owned(),
        ));
    }
    if message_type == "broadcast" && member.role != "commander" {
        return Err(ApiError::Forbidden(
            "only commanders may send broadcasts".to_owned(),
        ));
    }
    let timestamp = now();
    let message = space_messages::ActiveModel {
        id: Set(new_id()),
        space_id: Set(space.id.clone()),
        sender_id: Set(auth.id.clone()),
        message_type: Set(message_type),
        content: Set(content.to_owned()),
        sent_at: Set(timestamp.clone()),
        recalled_at: Set(None),
    }
    .insert(&transaction)
    .await?;
    publish_event(&transaction, &space, "message.sent", "space_members", json!({"message_id": message.id, "sender_id": auth.id, "sender_display_name": auth.display_name, "message_type": message.message_type, "content": message.content, "sent_at": message.sent_at}), &timestamp).await?;
    write_audit(
        &transaction,
        Some(space.case_id.clone()),
        auth,
        "collaboration_space.message_sent",
        "space_message",
        message.id.clone(),
        None,
    )
    .await?;
    transaction.commit().await?;
    message_response(db, message).await
}

pub async fn list_messages(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
) -> Result<Vec<SpaceMessageResponse>, ApiError> {
    require_space_access(db, auth, space_id).await?;
    let messages = space_messages::Entity::find()
        .filter(space_messages::Column::SpaceId.eq(space_id))
        .order_by_desc(space_messages::Column::SentAt)
        .limit(100)
        .all(db)
        .await?;
    let sender_ids: Vec<String> = messages
        .iter()
        .map(|message| message.sender_id.clone())
        .collect();
    let senders = users::Entity::find()
        .filter(users::Column::Id.is_in(sender_ids))
        .all(db)
        .await?;
    let names: HashMap<String, String> = senders
        .into_iter()
        .map(|user| (user.id, user.display_name))
        .collect();
    Ok(messages
        .into_iter()
        .map(|message| {
            let sender_display_name = names
                .get(&message.sender_id)
                .cloned()
                .unwrap_or_else(|| "未知用户".to_owned());
            message_response_with_name(message, sender_display_name)
        })
        .collect())
}

/// Stores an authorized member's audio report in private storage. There is no
/// configured ASR adapter in this deployment, so the report is deliberately
/// finalized as failed instead of inventing a transcript or leaving it queued.
pub async fn store_voice_report(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
    filename: &str,
    content_type: &str,
    bytes: &[u8],
    directory: &Path,
) -> Result<VoiceReportResponse, ApiError> {
    let extension = voice_extension(content_type)?;
    let transaction = db.begin().await?;
    let space = active_space(&transaction, space_id).await?;
    active_member(&transaction, space_id, &auth.id).await?;

    let id = Uuid::new_v4().to_string();
    let object_key = format!("voice/{id}.{extension}");
    let storage_path = directory.join(&object_key);
    write_private_audio(&storage_path, bytes.to_vec()).await?;
    let timestamp = now();
    let report = voice_reports::ActiveModel {
        id: Set(id),
        space_id: Set(space.id.clone()),
        case_id: Set(space.case_id.clone()),
        reporter_id: Set(auth.id.clone()),
        object_key: Set(object_key),
        content_type: Set(content_type.to_owned()),
        byte_size: Set(i64::try_from(bytes.len()).map_err(|_| ApiError::Internal)?),
        status: Set("failed".to_owned()),
        created_at: Set(timestamp.clone()),
        failed_reason: Set(Some("ASR provider is not configured".to_owned())),
    }
    .insert(&transaction)
    .await;
    let report = match report {
        Ok(value) => value,
        Err(error) => {
            remove_file_best_effort(storage_path).await;
            return Err(ApiError::Database(error));
        }
    };
    if let Err(error) = publish_event(
        &transaction,
        &space,
        "voice_report.processing_failed",
        "commanders",
        json!({"voice_report_id": report.id, "reporter_id": auth.id, "status": report.status}),
        &timestamp,
    )
    .await
    {
        remove_file_best_effort(storage_path).await;
        return Err(error);
    }
    if let Err(error) = write_audit(
        &transaction,
        Some(space.case_id.clone()),
        auth,
        "collaboration_space.voice_report_uploaded",
        "voice_report",
        report.id.clone(),
        Some(json!({
            "content_type": report.content_type,
            "byte_size": report.byte_size,
            "status": report.status,
            "filename_present": !filename.trim().is_empty(),
        })),
    )
    .await
    {
        remove_file_best_effort(storage_path).await;
        return Err(error);
    }
    if let Err(error) = transaction.commit().await {
        remove_file_best_effort(storage_path).await;
        return Err(ApiError::Database(error));
    }
    Ok(voice_report_response(report, None))
}

/// Commanders see reports and any transcript generated by a future approved
/// provider. Members may only see their own report state, never transcript text.
pub async fn list_voice_reports(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
) -> Result<Vec<VoiceReportResponse>, ApiError> {
    let (_space, own_member) = require_space_access(db, auth, space_id).await?;
    let is_commander = own_member
        .as_ref()
        .is_some_and(|member| member.role == "commander");
    let mut query = voice_reports::Entity::find()
        .filter(voice_reports::Column::SpaceId.eq(space_id))
        .order_by_desc(voice_reports::Column::CreatedAt)
        .limit(100);
    if !is_commander {
        query = query.filter(voice_reports::Column::ReporterId.eq(&auth.id));
    }
    let reports = query.all(db).await?;
    let mut responses = Vec::with_capacity(reports.len());
    for report in reports {
        let transcript = if is_commander {
            voice_transcripts::Entity::find()
                .filter(voice_transcripts::Column::VoiceReportId.eq(&report.id))
                .one(db)
                .await?
                .map(transcript_response)
        } else {
            None
        };
        responses.push(voice_report_response(report, transcript));
    }
    Ok(responses)
}

fn event_targets_user(event: &space_events::Model, user_id: &str) -> bool {
    serde_json::from_str::<Value>(&event.payload_json)
        .ok()
        .and_then(|payload| {
            payload
                .get("user_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .is_some_and(|target| target == user_id)
}

async fn require_space_access(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
) -> Result<(collaboration_spaces::Model, Option<space_members::Model>), ApiError> {
    let space = collaboration_spaces::Entity::find_by_id(space_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("collaboration space was not found".to_owned()))?;
    let role = require_case_role(
        db,
        &auth.id,
        &space.case_id,
        &[CaseRole::Commander, CaseRole::Volunteer],
    )
    .await?;
    if space.status == "archived" && role == CaseRole::Volunteer {
        return Err(ApiError::NotFound(
            "collaboration space was not found".to_owned(),
        ));
    }
    let member = space_members::Entity::find()
        .filter(space_members::Column::SpaceId.eq(space_id))
        .filter(space_members::Column::UserId.eq(&auth.id))
        .one(db)
        .await?;
    if role == CaseRole::Volunteer && !member.as_ref().is_some_and(|item| item.status == "active") {
        return Err(ApiError::NotFound(
            "collaboration space was not found".to_owned(),
        ));
    }
    Ok((space, member))
}

async fn active_space<C: sea_orm::ConnectionTrait>(
    db: &C,
    space_id: &str,
) -> Result<collaboration_spaces::Model, ApiError> {
    collaboration_spaces::Entity::find_by_id(space_id)
        .filter(collaboration_spaces::Column::Status.eq("active"))
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("active collaboration space was not found".to_owned()))
}

async fn active_member<C: sea_orm::ConnectionTrait>(
    db: &C,
    space_id: &str,
    user_id: &str,
) -> Result<space_members::Model, ApiError> {
    space_members::Entity::find()
        .filter(space_members::Column::SpaceId.eq(space_id))
        .filter(space_members::Column::UserId.eq(user_id))
        .filter(space_members::Column::Status.eq("active"))
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("collaboration space was not found".to_owned()))
}

async fn insert_member<C: sea_orm::ConnectionTrait>(
    db: &C,
    space: &collaboration_spaces::Model,
    auth: &AuthenticatedUser,
    role: CaseRole,
    timestamp: &str,
) -> Result<space_members::Model, ApiError> {
    let member = space_members::ActiveModel {
        id: Set(new_id()),
        space_id: Set(space.id.clone()),
        user_id: Set(auth.id.clone()),
        role: Set(role.as_str().to_owned()),
        status: Set("active".to_owned()),
        joined_at: Set(timestamp.to_owned()),
        left_at: Set(None),
    }
    .insert(db)
    .await?;
    if role == CaseRole::Volunteer {
        allocate_slot(db, &member).await?;
    }
    Ok(member)
}

async fn allocate_slot<C: sea_orm::ConnectionTrait>(
    db: &C,
    member: &space_members::Model,
) -> Result<(), ApiError> {
    let occupied = space_member_slots::Entity::find()
        .filter(space_member_slots::Column::UserId.eq(&member.user_id))
        .all(db)
        .await?;
    let slot = (1..=MAX_ACTIVE_SPACES_PER_VOLUNTEER)
        .find(|candidate| !occupied.iter().any(|item| item.slot == *candidate))
        .ok_or_else(|| {
            ApiError::Conflict("a volunteer may join at most three active spaces".to_owned())
        })?;
    space_member_slots::ActiveModel {
        user_id: Set(member.user_id.clone()),
        slot: Set(slot),
        member_id: Set(member.id.clone()),
    }
    .insert(db)
    .await
    .map_err(|_| {
        ApiError::Conflict("a volunteer may join at most three active spaces".to_owned())
    })?;
    Ok(())
}

async fn grant_consent<C: sea_orm::ConnectionTrait>(
    db: &C,
    space: &collaboration_spaces::Model,
    member: &space_members::Model,
    consent_version: &str,
    timestamp: &str,
) -> Result<(), ApiError> {
    let consent_version = validated_consent_version(consent_version.to_owned())?;
    match space_location_consents::Entity::find()
        .filter(space_location_consents::Column::MemberId.eq(&member.id))
        .one(db)
        .await?
    {
        Some(model) => {
            let mut active = model.into_active_model();
            active.consent_version = Set(consent_version);
            active.granted_at = Set(timestamp.to_owned());
            active.revoked_at = Set(None);
            active.update(db).await?;
        }
        None => {
            space_location_consents::ActiveModel {
                id: Set(new_id()),
                space_id: Set(space.id.clone()),
                user_id: Set(member.user_id.clone()),
                member_id: Set(member.id.clone()),
                consent_version: Set(consent_version),
                granted_at: Set(timestamp.to_owned()),
                revoked_at: Set(None),
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}

async fn revoke_consent<C: sea_orm::ConnectionTrait>(
    db: &C,
    member_id: &str,
    timestamp: &str,
) -> Result<(), ApiError> {
    if let Some(model) = space_location_consents::Entity::find()
        .filter(space_location_consents::Column::MemberId.eq(member_id))
        .one(db)
        .await?
    {
        let mut active = model.into_active_model();
        active.revoked_at = Set(Some(timestamp.to_owned()));
        active.update(db).await?;
    }
    Ok(())
}

async fn publish_event<C: sea_orm::ConnectionTrait>(
    db: &C,
    space: &collaboration_spaces::Model,
    event_type: &str,
    visibility_scope: &str,
    payload: Value,
    timestamp: &str,
) -> Result<(), ApiError> {
    let version = space.next_event_version + 1;
    let mut active = space.clone().into_active_model();
    active.next_event_version = Set(version);
    active.update(db).await?;
    let event = space_events::ActiveModel {
        id: Set(new_id()),
        space_id: Set(space.id.clone()),
        case_id: Set(space.case_id.clone()),
        event_type: Set(event_type.to_owned()),
        version: Set(version),
        visibility_scope: Set(visibility_scope.to_owned()),
        payload_json: Set(payload.to_string()),
        occurred_at: Set(timestamp.to_owned()),
    }
    .insert(db)
    .await?;
    event_outbox::ActiveModel {
        id: Set(new_id()),
        event_id: Set(event.id),
        topic: Set("collaboration_space".to_owned()),
        status: Set("pending".to_owned()),
        attempt_count: Set(0),
        available_at: Set(timestamp.to_owned()),
        delivered_at: Set(None),
        created_at: Set(timestamp.to_owned()),
    }
    .insert(db)
    .await?;
    Ok(())
}

fn space_response(
    space: collaboration_spaces::Model,
    member_status: Option<String>,
) -> CollaborationSpaceResponse {
    CollaborationSpaceResponse {
        id: space.id,
        case_id: space.case_id,
        name: space.name,
        status: space.status,
        created_by_user_id: space.created_by_user_id,
        created_at: space.created_at,
        archived_at: space.archived_at,
        current_version: space.next_event_version,
        member_status,
    }
}

fn event_response(event: space_events::Model) -> Result<SpaceEventResponse, ApiError> {
    Ok(SpaceEventResponse {
        event_id: event.id,
        space_id: event.space_id,
        case_id: event.case_id,
        event_type: event.event_type,
        version: event.version,
        occurred_at: event.occurred_at,
        visibility_scope: event.visibility_scope,
        payload: serde_json::from_str(&event.payload_json).map_err(|_| ApiError::Internal)?,
    })
}

fn location_response(
    sample: space_location_samples::Model,
) -> Result<SpaceLocationResponse, ApiError> {
    Ok(SpaceLocationResponse {
        id: sample.id,
        user_id: sample.user_id,
        latitude: sample.latitude,
        longitude: sample.longitude,
        accuracy_meters: sample.accuracy_meters,
        captured_at: sample.captured_at,
    })
}

async fn message_response(
    db: &DatabaseConnection,
    message: space_messages::Model,
) -> Result<SpaceMessageResponse, ApiError> {
    let sender_display_name = users::Entity::find_by_id(&message.sender_id)
        .one(db)
        .await?
        .map(|user| user.display_name)
        .unwrap_or_else(|| "未知用户".to_owned());
    Ok(message_response_with_name(message, sender_display_name))
}

fn message_response_with_name(
    message: space_messages::Model,
    sender_display_name: String,
) -> SpaceMessageResponse {
    SpaceMessageResponse {
        id: message.id,
        sender_id: message.sender_id,
        sender_display_name,
        message_type: message.message_type,
        content: message.content,
        sent_at: message.sent_at,
        recalled_at: message.recalled_at,
    }
}

fn voice_report_response(
    report: voice_reports::Model,
    transcript: Option<VoiceTranscriptResponse>,
) -> VoiceReportResponse {
    VoiceReportResponse {
        id: report.id,
        reporter_id: report.reporter_id,
        content_type: report.content_type,
        byte_size: report.byte_size,
        status: report.status,
        created_at: report.created_at,
        failed_reason: report.failed_reason,
        transcript,
    }
}

fn transcript_response(transcript: voice_transcripts::Model) -> VoiceTranscriptResponse {
    VoiceTranscriptResponse {
        content: transcript.content,
        provider: transcript.provider,
        status: transcript.status,
        created_at: transcript.created_at,
    }
}

fn voice_extension(content_type: &str) -> Result<&'static str, ApiError> {
    match content_type {
        "audio/mpeg" => Ok("mp3"),
        "audio/ogg" => Ok("ogg"),
        "audio/wav" => Ok("wav"),
        "audio/webm" => Ok("webm"),
        _ => Err(ApiError::Validation(
            "unsupported voice report content type".to_owned(),
        )),
    }
}

async fn write_private_audio(path: &Path, bytes: Vec<u8>) -> Result<(), ApiError> {
    let parent = path.parent().ok_or(ApiError::Internal)?.to_path_buf();
    let path = path.to_path_buf();
    web::block(move || {
        fs::create_dir_all(parent)?;
        fs::write(path, bytes)
    })
    .await
    .map_err(|_| ApiError::Internal)?
    .map_err(|_| ApiError::Internal)
}

async fn remove_file_best_effort(path: PathBuf) {
    let _ = web::block(move || fs::remove_file(path)).await;
}

fn validate_location(request: &RecordSpaceLocationRequest) -> Result<(), ApiError> {
    if !request.latitude.is_finite()
        || !(-90.0..=90.0).contains(&request.latitude)
        || !request.longitude.is_finite()
        || !(-180.0..=180.0).contains(&request.longitude)
        || !request.accuracy_meters.is_finite()
        || !(0.0..=10_000.0).contains(&request.accuracy_meters)
    {
        return Err(ApiError::Validation(
            "location coordinates or accuracy are invalid".to_owned(),
        ));
    }
    if request.operation_id.trim().is_empty()
        || request.operation_id.len() > 128
        || request.captured_at.trim().is_empty()
        || request.captured_at.len() > 64
    {
        return Err(ApiError::Validation(
            "captured_at and operation_id are required".to_owned(),
        ));
    }
    Ok(())
}

/// A missing policy intentionally disables sensitive trajectory collection.
/// Operations must configure a bounded retention window before enabling it.
fn location_retention_is_configured() -> bool {
    env::var("ANGUI_COLLABORATION_LOCATION_RETENTION_HOURS")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .is_some_and(|hours| (1..=8_760).contains(&hours))
}

fn validated_name(value: String) -> Result<String, ApiError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 {
        return Err(ApiError::Validation(
            "name must contain between 1 and 120 characters".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

fn validated_consent_version(value: String) -> Result<String, ApiError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 64 {
        return Err(ApiError::Validation(
            "consent_version must contain between 1 and 64 characters".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
