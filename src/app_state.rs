use sea_orm::DatabaseConnection;

use crate::rate_limit::LoginRateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub session_ttl_hours: i64,
    pub login_limiter: LoginRateLimiter,
}
