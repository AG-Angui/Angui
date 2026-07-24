use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::{Value, json};

use crate::support::{
    ADMIN, COMMANDER, FAMILY, LEARNER, PASSWORD, TestContext, VOLUNTEER, assert_error,
};

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
    assert_eq!(body["user"]["account_type"], "member");
    assert_eq!(body["user"]["global_capabilities"], json!([]));
}

#[actix_web::test]
async fn every_demo_role_can_log_in_and_restore_its_identity_from_the_session_token() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let accounts = [
        (FAMILY, "member", json!([])),
        (COMMANDER, "member", json!(["commander"])),
        (VOLUNTEER, "member", json!(["volunteer"])),
        (LEARNER, "learner", json!([])),
        (ADMIN, "member", json!(["admin"])),
    ];

    for (email, account_type, global_capabilities) in accounts {
        let login = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/auth/login")
                .set_json(json!({ "email": email, "password": PASSWORD }))
                .to_request(),
        )
        .await;
        assert_eq!(login.status(), StatusCode::OK, "{email} should log in");
        let body: Value = test::read_body_json(login).await;
        let token = body["token"]
            .as_str()
            .expect("login response should contain a token");
        assert_eq!(body["user"]["email"], email);
        assert_eq!(body["user"]["account_type"], account_type);
        assert_eq!(body["user"]["global_capabilities"], global_capabilities);

        let restored = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/auth/me")
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .to_request(),
        )
        .await;
        assert_eq!(
            restored.status(),
            StatusCode::OK,
            "{email} session should restore"
        );
        let current_user: Value = test::read_body_json(restored).await;
        assert_eq!(current_user["email"], email);
        assert_eq!(current_user["account_type"], account_type);
        assert_eq!(current_user["global_capabilities"], global_capabilities);
    }
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
