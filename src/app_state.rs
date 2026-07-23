use sea_orm::DatabaseConnection;

use crate::{ai_gateway::AiGateway, rate_limit::LoginRateLimiter};

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub session_ttl_hours: i64,
    pub intake_answer_hard_max: usize,
    pub ai_gateway: AiGateway,
    pub login_limiter: LoginRateLimiter,
}
