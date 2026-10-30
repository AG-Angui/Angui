use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, IntoActiveModel,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    ai_gateway::{AiCapability, AiExecutionResult, AiGateway, AiPurpose, AiRequest, DataLevel},
    audio_storage::{AudioStorage, SharedAudioStorage},
    entities::{
        audit_events, clue_drafts, intercom_recordings, voice_clue_candidates, voice_reports,
        voice_transcripts,
    },
    models::ClueDraftCandidate,
};

#[derive(Deserialize)]
struct AsrResult {
    text: String,
    model: String,
}

/// Claims durable jobs before network I/O. A crashed worker's lease expires
/// after five minutes, allowing a later tick to recover it.
pub fn start_worker(
    db: DatabaseConnection,
    storage: SharedAudioStorage,
    asr_url: Option<String>,
    asr_key: Option<String>,
    gateway: AiGateway,
) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                log::error!("voice worker HTTP client: {error}");
                return;
            }
        };
        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        loop {
            ticker.tick().await;
            let stale = (Utc::now() - chrono::Duration::minutes(5))
                .to_rfc3339_opts(SecondsFormat::Millis, true);
            let jobs = match voice_clue_candidates::Entity::find()
                .filter(
                    voice_clue_candidates::Column::Status
                        .eq("queued")
                        .and(voice_clue_candidates::Column::UpdatedAt.lte(now()))
                        .or(voice_clue_candidates::Column::Status
                            .eq("processing")
                            .and(voice_clue_candidates::Column::UpdatedAt.lt(stale))),
                )
                .order_by_asc(voice_clue_candidates::Column::CreatedAt)
                .limit(10)
                .all(&db)
                .await
            {
                Ok(jobs) => jobs,
                Err(error) => {
                    log::warn!("voice queue scan failed: {error}");
                    continue;
                }
            };
            for job in jobs {
                let claimed = voice_clue_candidates::Entity::update_many()
                    .col_expr(voice_clue_candidates::Column::Status, "processing".into())
                    .col_expr(voice_clue_candidates::Column::UpdatedAt, now().into())
                    .filter(voice_clue_candidates::Column::Id.eq(&job.id))
                    .filter(voice_clue_candidates::Column::Status.eq(&job.status))
                    .filter(voice_clue_candidates::Column::UpdatedAt.eq(&job.updated_at))
                    .exec(&db)
                    .await;
                if !matches!(claimed, Ok(result) if result.rows_affected == 1) {
                    continue;
                }
                if let Err(reason) = process(
                    &db,
                    storage.as_ref(),
                    asr_url.as_deref(),
                    asr_key.as_deref(),
                    &client,
                    &gateway,
                    &job,
                )
                .await
                    && let Err(error) = mark_failed(&db, &job, &reason).await
                {
                    log::error!("voice job {} failed to persist failure: {error}", job.id);
                }
            }
        }
    });
}

async fn process(
    db: &DatabaseConnection,
    storage: &dyn AudioStorage,
    asr_url: Option<&str>,
    asr_key: Option<&str>,
    client: &reqwest::Client,
    gateway: &AiGateway,
    job: &voice_clue_candidates::Model,
) -> Result<(), String> {
    let object_key = if let Some(report_id) = &job.voice_report_id {
        let report = voice_reports::Entity::find_by_id(report_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("voice report is missing")?;
        let mut active = report.clone().into_active_model();
        active.status = Set("transcribing".to_owned());
        active.update(db).await.map_err(|e| e.to_string())?;
        report.object_key
    } else if let Some(recording_id) = &job.intercom_recording_id {
        let recording = intercom_recordings::Entity::find_by_id(recording_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("intercom recording is missing")?;
        let mut active = recording.clone().into_active_model();
        active.transcription_status = Set("processing".to_owned());
        active.update(db).await.map_err(|e| e.to_string())?;
        recording.object_key
    } else {
        return Err("voice job has no audio source".to_owned());
    };
    let bytes = storage
        .read(&object_key)
        .await
        .map_err(|_| "audio object is unavailable".to_owned())?;
    let url = asr_url.ok_or("ASR provider is not configured")?;
    let mut request = client
        .post(url)
        .header("content-type", "application/octet-stream")
        .body(bytes);
    if let Some(key) = asr_key {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| format!("ASR request failed: {e}"))?;
    if !response.status().is_success() {
        let kind = if response.status().is_server_error() {
            "ASR transient response failed"
        } else {
            "ASR permanent response failed"
        };
        return Err(format!("{kind}: HTTP {}", response.status().as_u16()));
    }
    let asr: AsrResult = response
        .json()
        .await
        .map_err(|e| format!("invalid ASR response: {e}"))?;
    let transcript = asr.text.trim();
    if transcript.is_empty() || transcript.chars().count() > 10_000 {
        return Err("ASR transcript is empty or too long".to_owned());
    }
    if let Some(report_id) = &job.voice_report_id {
        if voice_transcripts::Entity::find()
            .filter(voice_transcripts::Column::VoiceReportId.eq(report_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .is_none()
        {
            voice_transcripts::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                voice_report_id: Set(report_id.clone()),
                content: Set(transcript.to_owned()),
                provider: Set(asr.model.clone()),
                status: Set("completed".to_owned()),
                created_at: Set(now()),
            }
            .insert(db)
            .await
            .map_err(|e| e.to_string())?;
        }
        if let Some(report) = voice_reports::Entity::find_by_id(report_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
        {
            let mut active = report.into_active_model();
            active.status = Set("transcribed".to_owned());
            active.failed_reason = Set(None);
            active.update(db).await.map_err(|e| e.to_string())?;
        }
    }
    let ai_request = AiRequest {
        capability: AiCapability::StructuredExtraction, data_level: DataLevel::Collaborative,
        purpose: AiPurpose::ClueDraft, data_region: "CN".to_owned(),
        system_instruction: Some("Extract only facts explicitly present in this ASR transcript. Return JSON. Do not infer facts, issue instructions or confirm a clue.".to_owned()),
        output_schema: Some(json!({"type":"object","additionalProperties":false,"required":["content_summary","occurred_at","location_text","source_text","action_candidates","missing_fields","source_excerpt","field_sources"],"properties":{"content_summary":{"type":["string","null"]},"occurred_at":{"type":["string","null"]},"location_text":{"type":["string","null"]},"source_text":{"type":["string","null"]},"action_candidates":{"type":"array","items":{"type":"string"}},"missing_fields":{"type":"array","items":{"type":"string"}},"source_excerpt":{"type":"string"},"field_sources":{"type":"object"}}})),
        output_schema_name: Some("voice_clue_draft".to_owned()), input: transcript.to_owned(),
        requested_output_tokens: 400, template_version: "voice-clue-v1".to_owned(),
        input_scope_reference: format!("voice_candidate:{}", job.id),
        redaction_policy_version: "case-collaboration-v1".to_owned(),
    };
    let execution = gateway.execute(&ai_request).await;
    let audits = crate::ai_gateway::execution_attempt_audits(&ai_request, &execution);
    let (candidate, model) = match execution {
        AiExecutionResult::Completed { route, output, .. } => {
            let candidate = gateway
                .decode_json::<ClueDraftCandidate>(&output)
                .map_err(|_| "AI returned invalid structured output".to_owned())?;
            (candidate, route.model)
        }
        _ => return Err("AI extraction provider is unavailable".to_owned()),
    };
    let candidate_json = serde_json::to_string(&candidate).map_err(|e| e.to_string())?;
    let transaction = db.begin().await.map_err(|e| e.to_string())?;
    let draft_id = if let Some(id) = &job.clue_draft_id {
        id.clone()
    } else {
        let draft = clue_drafts::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            case_id: Set(job.case_id.clone()),
            status: Set("draft".to_owned()),
            content: Set(transcript.to_owned()),
            source_type: Set("field_report".to_owned()),
            raw_record_reference: Set(Some(format!("voice_candidate:{}", job.id))),
            source_record_id: Set(None),
            uncertainty_notice: Set("AI 提取仅供人工审核；请核实时间、地点和发现人。".to_owned()),
            template_version: Set("voice-clue-v1".to_owned()),
            provider_model: Set(Some(model.clone())),
            degradation_status: Set("manual_review_required".to_owned()),
            candidate_json: Set(candidate_json.clone()),
            review_status: Set("pending_review".to_owned()),
            reviewed_by_user_id: Set(None),
            reviewed_at: Set(None),
            review_reason: Set(None),
            version: Set(1),
            promoted_clue_id: Set(None),
            created_by_user_id: Set(job.submitted_by_user_id.clone()),
            created_at: Set(now()),
            updated_at: Set(now()),
        }
        .insert(&transaction)
        .await
        .map_err(|e| e.to_string())?;
        draft.id
    };
    let current = voice_clue_candidates::Entity::find_by_id(&job.id)
        .one(&transaction)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("voice candidate is missing")?;
    let mut active = current.into_active_model();
    active.status = Set("pending_review".to_owned());
    active.transcript_text = Set(Some(transcript.to_owned()));
    active.asr_version = Set(Some(asr.model));
    active.model_version = Set(Some(model));
    active.candidate_json = Set(candidate_json);
    active.clue_draft_id = Set(Some(draft_id));
    active.failure_reason = Set(None);
    active.updated_at = Set(now());
    active
        .update(&transaction)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(report_id) = &job.voice_report_id
        && let Some(report) = voice_reports::Entity::find_by_id(report_id)
            .one(&transaction)
            .await
            .map_err(|e| e.to_string())?
    {
        let mut active = report.into_active_model();
        active.status = Set("draft_ready".to_owned());
        active
            .update(&transaction)
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(recording_id) = &job.intercom_recording_id
        && let Some(recording) = intercom_recordings::Entity::find_by_id(recording_id)
            .one(&transaction)
            .await
            .map_err(|e| e.to_string())?
    {
        let mut active = recording.into_active_model();
        active.transcription_status = Set("completed".to_owned());
        active.failed_reason = Set(None);
        active
            .update(&transaction)
            .await
            .map_err(|e| e.to_string())?;
    }
    crate::ai_gateway::persist_execution_audits(
        &transaction,
        &audits,
        &job.submitted_by_user_id,
        Some(&job.case_id),
    )
    .await
    .map_err(|e| e.to_string())?;
    transaction.commit().await.map_err(|e| e.to_string())
}

async fn mark_failed(
    db: &DatabaseConnection,
    job: &voice_clue_candidates::Model,
    reason: &str,
) -> Result<(), sea_orm::DbErr> {
    let retryable = (reason.starts_with("ASR request failed:")
        || reason.starts_with("ASR transient response failed:"))
        && job.retry_count < 5;
    let next_attempt = if retryable {
        (Utc::now() + chrono::Duration::seconds(15 * (1_i64 << std::cmp::min(job.retry_count, 4))))
            .to_rfc3339_opts(SecondsFormat::Millis, true)
    } else {
        now()
    };
    let current = voice_clue_candidates::Entity::find_by_id(&job.id)
        .one(db)
        .await?;
    if let Some(current) = current {
        let mut active = current.clone().into_active_model();
        active.status = Set(if retryable { "queued" } else { "failed" }.to_owned());
        active.retry_count = Set(current.retry_count + 1);
        active.failure_reason = Set(Some(reason.chars().take(300).collect()));
        active.updated_at = Set(next_attempt);
        active.update(db).await?;
    }
    if let Some(report_id) = &job.voice_report_id
        && let Some(report) = voice_reports::Entity::find_by_id(report_id).one(db).await?
    {
        let mut active = report.into_active_model();
        active.status = Set(if retryable { "transcribing" } else { "failed" }.to_owned());
        active.failed_reason = Set(Some(reason.chars().take(300).collect()));
        active.update(db).await?;
    }
    if let Some(recording_id) = &job.intercom_recording_id
        && let Some(recording) = intercom_recordings::Entity::find_by_id(recording_id)
            .one(db)
            .await?
    {
        let mut active = recording.into_active_model();
        active.transcription_status = Set(if retryable { "queued" } else { "failed" }.to_owned());
        active.failed_reason = Set(Some(reason.chars().take(300).collect()));
        active.update(db).await?;
    }
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Removes expired private audio while retaining the review metadata and an
/// audit record. Missing files are treated as a recoverable prior deletion.
pub fn start_audio_retention(db: DatabaseConnection, storage: SharedAudioStorage, days: u64) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(3600));
        loop {
            ticker.tick().await;
            let cutoff = (Utc::now() - chrono::Duration::days(days as i64))
                .to_rfc3339_opts(SecondsFormat::Millis, true);
            match voice_reports::Entity::find()
                .filter(voice_reports::Column::CreatedAt.lt(&cutoff))
                .filter(voice_reports::Column::AudioDeletedAt.is_null())
                .limit(100)
                .all(&db)
                .await
            {
                Ok(reports) => {
                    for report in reports {
                        if storage.delete(&report.object_key).await.is_err() {
                            continue;
                        }
                        let transaction = match db.begin().await {
                            Ok(value) => value,
                            Err(_) => continue,
                        };
                        let mut active = report.clone().into_active_model();
                        active.audio_deleted_at = Set(Some(now()));
                        if active.update(&transaction).await.is_err() {
                            continue;
                        }
                        if retention_audit(
                            &transaction,
                            &report.case_id,
                            "voice_report",
                            &report.id,
                        )
                        .await
                        .is_err()
                        {
                            continue;
                        }
                        let _ = transaction.commit().await;
                    }
                }
                Err(error) => log::warn!("voice report retention scan failed: {error}"),
            }
            match intercom_recordings::Entity::find()
                .filter(intercom_recordings::Column::CreatedAt.lt(&cutoff))
                .filter(intercom_recordings::Column::AudioDeletedAt.is_null())
                .limit(100)
                .all(&db)
                .await
            {
                Ok(recordings) => {
                    for recording in recordings {
                        if storage.delete(&recording.object_key).await.is_err() {
                            continue;
                        }
                        let transaction = match db.begin().await {
                            Ok(value) => value,
                            Err(_) => continue,
                        };
                        let mut active = recording.clone().into_active_model();
                        active.audio_deleted_at = Set(Some(now()));
                        if active.update(&transaction).await.is_err() {
                            continue;
                        }
                        if retention_audit(
                            &transaction,
                            &recording.case_id,
                            "intercom_recording",
                            &recording.id,
                        )
                        .await
                        .is_err()
                        {
                            continue;
                        }
                        let _ = transaction.commit().await;
                    }
                }
                Err(error) => log::warn!("intercom retention scan failed: {error}"),
            }
        }
    });
}

async fn retention_audit(
    db: &sea_orm::DatabaseTransaction,
    case_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<(), sea_orm::DbErr> {
    audit_events::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        case_id: Set(Some(case_id.to_owned())),
        actor: Set("system".to_owned()),
        action: Set("audio.retention_deleted".to_owned()),
        entity_type: Set(entity_type.to_owned()),
        entity_id: Set(entity_id.to_owned()),
        metadata_json: Set(None),
        created_at: Set(now()),
    }
    .insert(db)
    .await?;
    Ok(())
}
