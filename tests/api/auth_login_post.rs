use actix_web::{http::StatusCode, test};
use serde_json::{Value, json};

use crate::support::{FAMILY, PASSWORD, TestContext, assert_error};

#[actix_web::test]
async fn login_returns_a_session_for_valid_demo_credentials() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(json!({ "email": FAMILY, "password": PASSWORD }))
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = test::read_body_json(response).await;
    assert!(
        body["token"]
            .as_str()
            .is_some_and(|token| token.starts_with("angui_"))
    );
    assert_eq!(body["user"]["email"], FAMILY);
    assert_eq!(body["user"]["role"], "family");
}

#[actix_web::test]
async fn login_hides_account_existence_and_rate_limits_repeated_failures() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let wrong_password = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(json!({ "email": FAMILY, "password": "wrong-password" }))
            .to_request(),
    )
    .await;
    let unknown_account = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(json!({ "email": "unknown@demo.invalid", "password": "wrong-password" }))
            .to_request(),
    )
    .await;
    assert_eq!(wrong_password.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(unknown_account.status(), StatusCode::UNAUTHORIZED);
    let wrong_body: Value = test::read_body_json(wrong_password).await;
    let unknown_body: Value = test::read_body_json(unknown_account).await;
    assert_eq!(wrong_body, unknown_body);

    for _ in 0..3 {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/auth/login")
                .set_json(json!({ "email": FAMILY, "password": "wrong-password" }))
                .to_request(),
        )
        .await;
        assert_error(response, StatusCode::UNAUTHORIZED, "unauthorized").await;
    }
    let limited = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(json!({ "email": FAMILY, "password": "wrong-password" }))
            .to_request(),
    )
    .await;
    assert_error(limited, StatusCode::TOO_MANY_REQUESTS, "rate_limited").await;
}
