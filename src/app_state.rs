use sea_orm::DatabaseConnection;

use crate::{ai_gateway::AiGateway, amap_service::AmapService, rate_limit::LoginRateLimiter};

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub session_ttl_hours: i64,
    pub intake_answer_hard_max: usize,
    pub amap_service: AmapService,
    pub ai_gateway: AiGateway,
    pub login_limiter: LoginRateLimiter,
}
