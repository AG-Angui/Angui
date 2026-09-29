use std::path::PathBuf;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::audio_storage::SharedAudioStorage;
use crate::models::AuthenticatedUser;
use sea_orm::DatabaseConnection;

pub struct SocketTicket {
    pub auth: AuthenticatedUser,
    pub space_id: String,
    pub created: std::time::Instant,
}

use crate::{
    ai_gateway::AiGateway, amap_service::AmapService, message_delivery::MessageDelivery,
    rate_limit::LoginRateLimiter,
};

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub frontend_origin: String,
    pub session_ttl_hours: i64,
    pub intake_answer_hard_max: usize,
    pub attachment_storage_directory: PathBuf,
    pub audio_storage: SharedAudioStorage,
    pub attachment_max_image_bytes: usize,
    pub attachment_max_per_case: u64,
    pub case_place_types: Vec<String>,
    pub poi_selection_token_secret: String,
    pub amap_service: AmapService,
    pub ai_gateway: AiGateway,
    pub livekit_url: Option<String>,
    pub livekit_api_key: Option<String>,
    pub livekit_api_secret: Option<String>,
    pub livekit_admin_url: Option<String>,
    pub turn_url: Option<String>,
    pub turn_secret: Option<String>,
    pub voice_floors: Arc<Mutex<HashMap<String, (String, std::time::Instant)>>>,
    pub voice_floor_gate: Arc<tokio::sync::Mutex<()>>,
    pub ws_tickets: Arc<Mutex<HashMap<String, SocketTicket>>>,
    pub voice_sessions: Arc<Mutex<HashMap<(String, String), AuthenticatedUser>>>,
    pub login_limiter: LoginRateLimiter,
    pub message_delivery: MessageDelivery,
}
