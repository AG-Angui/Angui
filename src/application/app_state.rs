use std::path::PathBuf;

use sea_orm::DatabaseConnection;

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
    pub attachment_max_image_bytes: usize,
    pub attachment_max_per_case: u64,
    pub case_place_types: Vec<String>,
    pub poi_selection_token_secret: String,
    pub amap_service: AmapService,
    pub ai_gateway: AiGateway,
    pub login_limiter: LoginRateLimiter,
    pub message_delivery: MessageDelivery,
}
