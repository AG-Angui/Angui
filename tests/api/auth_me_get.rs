use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{FAMILY, TestContext, assert_error};

#[actix_web::test]
async fn get_auth_me_returns_the_authenticated_identity_and_rejects_bad_tokens() {
    let context = TestContext::new().await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let authorized = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/auth/me")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;
    assert_eq!(authorized.status(), StatusCode::OK);
    let body: Value = test::read_body_json(authorized).await;
    assert_eq!(body["email"], FAMILY);

    let missing = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/auth/me").to_request(),
    )
    .await;
    assert_error(missing, StatusCode::UNAUTHORIZED, "unauthorized").await;
    let invalid = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/auth/me")
            .insert_header((header::AUTHORIZATION, "Bearer invalid-token"))
            .to_request(),
    )
    .await;
    assert_error(invalid, StatusCode::UNAUTHORIZED, "unauthorized").await;
}
