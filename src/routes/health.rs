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

#[cfg(test)]
mod tests {
    use actix_web::{App, http::StatusCode, test};

    use super::HealthResponse;
    use crate::routes;

    #[actix_web::test]
    async fn health_endpoint_returns_service_metadata() {
        let app = test::init_service(App::new().configure(routes::configure)).await;
        let request = test::TestRequest::get().uri("/api/health").to_request();

        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: HealthResponse = test::read_body_json(response).await;
        assert_eq!(body.status, "ok");
        assert_eq!(body.service, "angui-api");
        assert_eq!(body.version, env!("CARGO_PKG_VERSION"));
    }
}
