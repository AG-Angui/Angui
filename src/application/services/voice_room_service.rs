use std::time::{Duration, Instant};

use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sea_orm::EntityTrait;
use serde::Serialize;
use serde_json::{Value, json};
use sha1::Sha1;
use sha2::Sha256;
use uuid::Uuid;

use crate::{
    app_state::{AppState, VoiceSession},
    entities::{auth_sessions, users},
    error::ApiError,
    models::AuthenticatedUser,
    services::collaboration_space_service,
};

#[derive(Serialize)]
pub struct VoiceRoomTicket {
    pub url: String,
    pub token: String,
    pub participant_identity: String,
    pub expires_at: i64,
    pub ice_servers: Vec<Value>,
}

fn token(
    key: &str,
    secret: &str,
    identity: &str,
    name: &str,
    grant: Value,
) -> Result<String, ApiError> {
    let now = Utc::now().timestamp();
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&json!({
            "iss": key, "sub": identity, "name": name, "nbf": now - 5,
            "exp": now + 300, "video": grant
        }))
        .map_err(|_| ApiError::Internal)?,
    );
    let signed = format!("{header}.{payload}");
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| ApiError::Internal)?;
    mac.update(signed.as_bytes());
    Ok(format!(
        "{signed}.{}",
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    ))
}

pub async fn room_ticket(
    state: &AppState,
    auth: &AuthenticatedUser,
    space_id: &str,
) -> Result<VoiceRoomTicket, ApiError> {
    collaboration_space_service::authorize_voice(&state.db, auth, space_id).await?;
    let (url, key, secret) = configuration(state)?;
    let room = format!("space-{space_id}");
    let identity = format!("{}:{}", auth.id, Uuid::new_v4());
    let grant = json!({"roomJoin":true,"room":room,"canPublish":false,"canSubscribe":true,"canPublishData":false});
    let room_token = token(key, secret, &identity, &auth.display_name, grant)?;
    let ice_servers = match (&state.turn_url, &state.turn_secret) {
        (Some(url), Some(secret)) => {
            let username = format!("{}:{}", Utc::now().timestamp() + 600, auth.id);
            let mut mac =
                Hmac::<Sha1>::new_from_slice(secret.as_bytes()).map_err(|_| ApiError::Internal)?;
            mac.update(username.as_bytes());
            vec![
                json!({"urls":url.split(',').collect::<Vec<_>>(),"username":username,"credential":STANDARD.encode(mac.finalize().into_bytes())}),
            ]
        }
        _ => Vec::new(),
    };
    state
        .voice_sessions
        .lock()
        .map_err(|_| ApiError::Internal)?
        .insert(
            identity.clone(),
            VoiceSession {
                auth: auth.clone(),
                space_id: space_id.to_owned(),
            },
        );
    Ok(VoiceRoomTicket {
        url: url.to_owned(),
        token: room_token,
        participant_identity: identity,
        expires_at: Utc::now().timestamp() + 300,
        ice_servers,
    })
}

pub async fn set_floor(
    state: &AppState,
    auth: &AuthenticatedUser,
    space_id: &str,
    identity: &str,
    active: bool,
) -> Result<(), ApiError> {
    let role = collaboration_space_service::authorize_voice(&state.db, auth, space_id).await?;
    // Keep identity validation and the remote permission change in one critical section.
    let _gate = state.voice_floor_gate.lock().await;
    let registered = state
        .voice_sessions
        .lock()
        .map_err(|_| ApiError::Internal)?
        .get(identity)
        .is_some_and(|session| {
            session.space_id == space_id
                && session.auth.id == auth.id
                && session.auth.session_id == auth.session_id
        });
    if !registered {
        return Err(ApiError::Conflict(
            "voice room session is no longer active".to_owned(),
        ));
    }
    let room = format!("space-{space_id}");
    let previous = state
        .voice_floors
        .lock()
        .map_err(|_| ApiError::Internal)?
        .get(space_id)
        .cloned();
    if active {
        if previous.as_ref().is_some_and(|(owner, at)| {
            owner != identity && at.elapsed() < Duration::from_secs(15) && role != "commander"
        }) {
            return Err(ApiError::Conflict("another member is speaking".to_owned()));
        }
        if let Some((owner, _)) = previous.as_ref().filter(|(owner, _)| owner != identity) {
            update_participant(state, &room, owner, false).await?;
            state
                .voice_floors
                .lock()
                .map_err(|_| ApiError::Internal)?
                .remove(space_id);
        }
        update_participant(state, &room, identity, true).await?;
        state
            .voice_floors
            .lock()
            .map_err(|_| ApiError::Internal)?
            .insert(space_id.to_owned(), (identity.to_owned(), Instant::now()));
    } else {
        // An old window must not revoke a newer window's publish permission.
        if !previous
            .as_ref()
            .is_some_and(|(owner, _)| owner == identity)
        {
            return Ok(());
        }
        update_participant(state, &room, identity, false).await?;
        state
            .voice_floors
            .lock()
            .map_err(|_| ApiError::Internal)?
            .remove(space_id);
    }
    Ok(())
}

pub async fn leave_room(
    state: &AppState,
    auth: &AuthenticatedUser,
    space_id: &str,
    identity: &str,
) -> Result<(), ApiError> {
    let owned = state
        .voice_sessions
        .lock()
        .map_err(|_| ApiError::Internal)?
        .get(identity)
        .is_some_and(|session| {
            session.space_id == space_id
                && session.auth.id == auth.id
                && session.auth.session_id == auth.session_id
        });
    if owned {
        disconnect_identity(state, space_id, identity).await;
    }
    Ok(())
}

pub async fn disconnect_user(state: &AppState, space_id: &str, user_id: &str) {
    let identities: Vec<_> = match state.voice_sessions.lock() {
        Ok(sessions) => sessions
            .iter()
            .filter(|(_, session)| session.space_id == space_id && session.auth.id == user_id)
            .map(|(identity, _)| identity.clone())
            .collect(),
        Err(_) => return,
    };
    for identity in identities {
        disconnect_identity(state, space_id, &identity).await;
    }
}

async fn disconnect_identity(state: &AppState, space_id: &str, identity: &str) {
    let _gate = state.voice_floor_gate.lock().await;
    if let Ok(mut sessions) = state.voice_sessions.lock() {
        sessions.remove(identity);
    }
    if let Ok(mut floors) = state.voice_floors.lock()
        && floors
            .get(space_id)
            .is_some_and(|(owner, _)| owner == identity)
    {
        floors.remove(space_id);
    }
    if let Err(error) = room_rpc(
        state,
        "RemoveParticipant",
        json!({"room":format!("space-{space_id}"),"identity":identity}),
    )
    .await
    {
        log::warn!("could not disconnect media participant: {error}");
    }
}

pub async fn close_room(state: &AppState, space_id: &str) {
    let _ = room_rpc(
        state,
        "DeleteRoom",
        json!({"room":format!("space-{space_id}")}),
    )
    .await;
    if let Ok(mut floors) = state.voice_floors.lock() {
        floors.remove(space_id);
    }
    if let Ok(mut sessions) = state.voice_sessions.lock() {
        sessions.retain(|_, session| session.space_id != space_id);
    }
}

/// Revoke connected media shortly after a role, account, session or membership
/// is removed. LiveKit room tokens cannot revoke an established connection.
pub fn start_membership_reaper(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        loop {
            ticker.tick().await;
            let sessions: Vec<_> = match state.voice_sessions.lock() {
                Ok(map) => map
                    .iter()
                    .map(|(identity, session)| (identity.clone(), session.clone()))
                    .collect(),
                Err(_) => continue,
            };
            for (identity, session) in sessions {
                let auth = &session.auth;
                let session_ok = match auth_sessions::Entity::find_by_id(&auth.session_id)
                    .one(&state.db)
                    .await
                {
                    Ok(Some(session)) => {
                        session.revoked_at.is_none()
                            && DateTime::parse_from_rfc3339(&session.expires_at)
                                .is_ok_and(|expires| expires > Utc::now())
                            && session.user_id == auth.id
                    }
                    _ => false,
                };
                let user_ok = users::Entity::find_by_id(&auth.id)
                    .one(&state.db)
                    .await
                    .ok()
                    .flatten()
                    .is_some_and(|user| user.status == "active");
                if !session_ok
                    || !user_ok
                    || collaboration_space_service::authorize_voice(
                        &state.db,
                        auth,
                        &session.space_id,
                    )
                    .await
                    .is_err()
                {
                    disconnect_identity(&state, &session.space_id, &identity).await;
                }
            }
        }
    });
}

async fn update_participant(
    state: &AppState,
    room: &str,
    identity: &str,
    can_publish: bool,
) -> Result<(), ApiError> {
    room_rpc(state, "UpdateParticipant", json!({
        "room":room,"identity":identity,
        "permission":{"canSubscribe":true,"canPublish":can_publish,"canPublishData":false,"canUpdateMetadata":false}
    })).await
}

async fn room_rpc(state: &AppState, method: &str, body: Value) -> Result<(), ApiError> {
    let (_, key, secret) = configuration(state)?;
    let url = state
        .livekit_admin_url
        .as_deref()
        .ok_or_else(|| ApiError::Conflict("LiveKit admin URL is not configured".to_owned()))?;
    let room = body
        .get("room")
        .and_then(Value::as_str)
        .ok_or(ApiError::Internal)?;
    let jwt = token(
        key,
        secret,
        "angui-api",
        "Angui API",
        json!({"roomAdmin":true,"room":room}),
    )?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| ApiError::Internal)?;
    let response = client
        .post(format!(
            "{}/twirp/livekit.RoomService/{method}",
            url.trim_end_matches('/')
        ))
        .bearer_auth(jwt)
        .json(&body)
        .send()
        .await
        .map_err(|_| ApiError::Internal)?;
    if !response.status().is_success() {
        log::warn!(
            "LiveKit RoomService {method} returned HTTP {}",
            response.status()
        );
    }
    if !response.status().is_success()
        && !(matches!(method, "RemoveParticipant" | "DeleteRoom")
            && response.status() == reqwest::StatusCode::NOT_FOUND)
    {
        if method == "UpdateParticipant" && response.status() == reqwest::StatusCode::NOT_FOUND {
            if body["permission"]["canPublish"] == false {
                return Ok(());
            }
            return Err(ApiError::Conflict(
                "media participant is not connected".to_owned(),
            ));
        }
        return Err(ApiError::Conflict(
            "LiveKit room operation failed".to_owned(),
        ));
    }
    Ok(())
}

fn configuration(state: &AppState) -> Result<(&str, &str, &str), ApiError> {
    match (
        &state.livekit_url,
        &state.livekit_api_key,
        &state.livekit_api_secret,
    ) {
        (Some(url), Some(key), Some(secret)) => Ok((url, key, secret)),
        _ => Err(ApiError::Conflict("LiveKit is not configured".to_owned())),
    }
}
