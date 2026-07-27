use actix_web::{HttpResponse, Responder, get};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

#[get("/health")]
pub async fn get_health() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_owned(),
        service: "angui-api".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}
