use actix_web::{http::StatusCode, test};
use serde_json::Value;

#[actix_web::test]
async fn get_health_returns_documented_service_metadata() {
    let context = crate::support::TestContext::new().await;
    let app = crate::init_api_app!(&context);

    let response = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/health").to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "angui-api");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}
