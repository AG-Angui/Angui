use std::time::Duration;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};

use crate::entities::{voice_clue_candidates, voice_reports};

/// Consumes durable voice candidates without blocking HTTP uploads. A real ASR
/// adapter can replace `process_candidate`; until configured, failure is
/// explicit and retryable rather than an invented transcript.
pub fn start_worker(db: DatabaseConnection) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(15));
        loop {
            ticker.tick().await;
            let candidates = match voice_clue_candidates::Entity::find()
                .filter(voice_clue_candidates::Column::Status.eq("queued"))
                .all(&db)
                .await
            {
                Ok(items) => items,
                Err(_) => continue,
            };
            for candidate in candidates.into_iter().take(10) {
                let _ = process_candidate(&db, candidate).await;
            }
        }
    });
}

async fn process_candidate(
    db: &DatabaseConnection,
    candidate: voice_clue_candidates::Model,
) -> Result<(), sea_orm::DbErr> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut active = candidate.clone().into_active_model();
    active.status = Set("failed".to_owned());
    active.retry_count = Set(candidate.retry_count + 1);
    active.failure_reason = Set(Some(
        "ASR provider is not configured; retry after configuring an approved adapter".to_owned(),
    ));
    active.updated_at = Set(now.clone());
    active.update(db).await?;
    if let Some(report_id) = candidate.voice_report_id
        && let Some(report) = voice_reports::Entity::find_by_id(report_id).one(db).await?
    {
        let mut report = report.into_active_model();
        report.status = Set("failed".to_owned());
        report.failed_reason = Set(Some("ASR provider is not configured".to_owned()));
        report.update(db).await?;
    }
    Ok(())
}
