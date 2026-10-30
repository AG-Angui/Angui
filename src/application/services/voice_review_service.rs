use chrono::{SecondsFormat, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set, TransactionTrait,
};
use serde_json::json;

use crate::{
    entities::{
        clue_drafts, clues, collaboration_spaces, intercom_recordings, voice_clue_candidates,
        voice_reports,
    },
    error::ApiError,
    models::{AuthenticatedUser, ClueDraftCandidate},
    roles::CaseRole,
    services::{
        case_collaboration_service::normalized_candidate,
        case_service::{require_case_role, write_audit},
        collaboration_space_service,
    },
};

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

async fn source(
    db: &DatabaseConnection,
    space_id: &str,
    id: &str,
) -> Result<(String, voice_clue_candidates::Model, clue_drafts::Model), ApiError> {
    let space = collaboration_spaces::Entity::find_by_id(space_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("collaboration space was not found".to_owned()))?;
    let candidate = voice_clue_candidates::Entity::find_by_id(id)
        .one(db)
        .await?
        .filter(|item| item.case_id == space.case_id)
        .ok_or_else(|| ApiError::NotFound("voice candidate was not found".to_owned()))?;
    let belongs = if let Some(id) = &candidate.voice_report_id {
        voice_reports::Entity::find_by_id(id)
            .one(db)
            .await?
            .is_some_and(|item| item.space_id == space_id)
    } else if let Some(id) = &candidate.intercom_recording_id {
        intercom_recordings::Entity::find_by_id(id)
            .one(db)
            .await?
            .is_some_and(|item| item.space_id == space_id)
    } else {
        false
    };
    if !belongs {
        return Err(ApiError::NotFound(
            "voice candidate was not found".to_owned(),
        ));
    }
    let draft_id = candidate
        .clue_draft_id
        .as_ref()
        .ok_or_else(|| ApiError::Conflict("candidate has no draft".to_owned()))?;
    let draft = clue_drafts::Entity::find_by_id(draft_id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("clue draft was not found".to_owned()))?;
    if draft.case_id != space.case_id || draft.review_status != "pending_review" {
        return Err(ApiError::Conflict(
            "clue draft is no longer pending review".to_owned(),
        ));
    }
    Ok((space.case_id, candidate, draft))
}

/// Sends an unreviewed voice draft back to its submitter for a manual revision.
pub async fn return_candidate(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
    id: &str,
    reason: &str,
) -> Result<(), ApiError> {
    let role = collaboration_space_service::authorize_voice(db, auth, space_id).await?;
    if role != "commander" {
        return Err(ApiError::Forbidden("commander role required".to_owned()));
    }
    let (case_id, candidate, draft) = source(db, space_id, id).await?;
    if candidate.returned_for_revision {
        return Err(ApiError::Conflict(
            "draft has already been returned for revision".to_owned(),
        ));
    }
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 1000 {
        return Err(ApiError::Validation(
            "return reason is required and must be at most 1000 characters".to_owned(),
        ));
    }
    let transaction = db.begin().await?;
    let affected = clue_drafts::Entity::update_many()
        .col_expr(
            clue_drafts::Column::ReviewReason,
            Expr::value(Some(reason.to_owned())),
        )
        .col_expr(clue_drafts::Column::Version, Expr::value(draft.version + 1))
        .col_expr(clue_drafts::Column::UpdatedAt, Expr::value(now()))
        .filter(clue_drafts::Column::Id.eq(&draft.id))
        .filter(clue_drafts::Column::Version.eq(draft.version))
        .filter(clue_drafts::Column::ReviewStatus.eq("pending_review"))
        .exec(&transaction)
        .await?;
    if affected.rows_affected != 1 {
        return Err(ApiError::Conflict("draft changed during review".to_owned()));
    }
    voice_clue_candidates::Entity::update_many()
        .col_expr(
            voice_clue_candidates::Column::ReturnedForRevision,
            Expr::value(true),
        )
        .filter(voice_clue_candidates::Column::Id.eq(id))
        .exec(&transaction)
        .await?;
    write_audit(
        &transaction,
        Some(case_id),
        auth,
        "voice_candidate.returned",
        "voice_clue_candidate",
        id.to_owned(),
        Some(json!({"reason_length":reason.chars().count()})),
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

/// The submitter edits only a returned candidate; the original transcript and
/// audio remain immutable and linked for the next reviewer.
pub async fn resubmit_candidate(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
    id: &str,
    candidate: ClueDraftCandidate,
) -> Result<(), ApiError> {
    collaboration_space_service::authorize_voice(db, auth, space_id).await?;
    let (case_id, source, draft) = source(db, space_id, id).await?;
    if source.submitted_by_user_id != auth.id {
        return Err(ApiError::NotFound(
            "voice candidate was not found".to_owned(),
        ));
    }
    if !source.returned_for_revision {
        return Err(ApiError::Conflict(
            "draft has not been returned for revision".to_owned(),
        ));
    }
    let normalized = normalized_candidate(candidate, &draft.content);
    let candidate_json = serde_json::to_string(&normalized).map_err(|_| ApiError::Internal)?;
    if candidate_json.len() > 8_000 {
        return Err(ApiError::Validation("candidate is too long".to_owned()));
    }
    let transaction = db.begin().await?;
    let affected = clue_drafts::Entity::update_many()
        .col_expr(
            clue_drafts::Column::CandidateJson,
            Expr::value(candidate_json.clone()),
        )
        .col_expr(
            clue_drafts::Column::ReviewReason,
            Expr::value(Option::<String>::None),
        )
        .col_expr(clue_drafts::Column::Version, Expr::value(draft.version + 1))
        .col_expr(clue_drafts::Column::UpdatedAt, Expr::value(now()))
        .filter(clue_drafts::Column::Id.eq(&draft.id))
        .filter(clue_drafts::Column::Version.eq(draft.version))
        .filter(clue_drafts::Column::ReviewStatus.eq("pending_review"))
        .exec(&transaction)
        .await?;
    if affected.rows_affected != 1 {
        return Err(ApiError::Conflict(
            "draft changed during revision".to_owned(),
        ));
    }
    let mut active = source.into_active_model();
    active.candidate_json = Set(candidate_json);
    active.returned_for_revision = Set(false);
    active.updated_at = Set(now());
    active.update(&transaction).await?;
    write_audit(
        &transaction,
        Some(case_id),
        auth,
        "voice_candidate.resubmitted",
        "voice_clue_candidate",
        id.to_owned(),
        None,
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

/// Merges a reviewed source into an existing clue's provenance without
/// silently rewriting the target clue's facts or review status.
pub async fn merge_candidate(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    space_id: &str,
    id: &str,
    target_clue_id: &str,
    reason: &str,
) -> Result<(), ApiError> {
    let role = collaboration_space_service::authorize_voice(db, auth, space_id).await?;
    if role != "commander" {
        return Err(ApiError::Forbidden("commander role required".to_owned()));
    }
    let (case_id, source, draft) = source(db, space_id, id).await?;
    if source.returned_for_revision {
        return Err(ApiError::Conflict(
            "the submitter must resubmit the returned draft before merge".to_owned(),
        ));
    }
    if source.discoverer_user_id.is_none() {
        return Err(ApiError::Conflict(
            "the submitter must confirm the discoverer before merge".to_owned(),
        ));
    }
    let target = clues::Entity::find_by_id(target_clue_id)
        .one(db)
        .await?
        .filter(|item| {
            item.case_id == case_id
                && matches!(item.status.as_str(), "pending_review" | "confirmed")
        })
        .ok_or_else(|| ApiError::NotFound("target clue was not found".to_owned()))?;
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 1000 {
        return Err(ApiError::Validation(
            "merge reason is required and must be at most 1000 characters".to_owned(),
        ));
    }
    require_case_role(db, &auth.id, &case_id, &[CaseRole::Commander]).await?;
    let transaction = db.begin().await?;
    let affected = clue_drafts::Entity::update_many()
        .col_expr(clue_drafts::Column::ReviewStatus, Expr::value("accepted"))
        .col_expr(
            clue_drafts::Column::PromotedClueId,
            Expr::value(Some(target.id.clone())),
        )
        .col_expr(
            clue_drafts::Column::ReviewedByUserId,
            Expr::value(Some(auth.id.clone())),
        )
        .col_expr(clue_drafts::Column::ReviewedAt, Expr::value(Some(now())))
        .col_expr(
            clue_drafts::Column::ReviewReason,
            Expr::value(Some(reason.to_owned())),
        )
        .col_expr(clue_drafts::Column::Version, Expr::value(draft.version + 1))
        .col_expr(clue_drafts::Column::UpdatedAt, Expr::value(now()))
        .filter(clue_drafts::Column::Id.eq(&draft.id))
        .filter(clue_drafts::Column::Version.eq(draft.version))
        .filter(clue_drafts::Column::ReviewStatus.eq("pending_review"))
        .exec(&transaction)
        .await?;
    if affected.rows_affected != 1 {
        return Err(ApiError::Conflict("draft changed during merge".to_owned()));
    }
    let mut active = source.into_active_model();
    active.status = Set("confirmed".to_owned());
    active.promoted_clue_id = Set(Some(target.id.clone()));
    active.updated_at = Set(now());
    active.update(&transaction).await?;
    write_audit(
        &transaction,
        Some(case_id),
        auth,
        "voice_candidate.merged",
        "voice_clue_candidate",
        id.to_owned(),
        Some(json!({"target_clue_id":target.id,"reason_length":reason.chars().count()})),
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}
