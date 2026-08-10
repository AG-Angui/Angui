use crate::support::{ADMIN, TestContext, assert_error};
use actix_web::{http::StatusCode, test};
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};

#[actix_web::test]
async fn access_request_verification_is_public_and_admin_review_is_protected() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(&app, test::TestRequest::post().uri("/api/auth/access-requests").set_json(json!({"email":"new-user@example.invalid","display_name":"New user","requested_role":"volunteer"})).to_request()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["status"], "pending_verification");
    let token = context
        .database
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Sqlite,
            "SELECT token_hash FROM auth_email_tokens LIMIT 1",
        ))
        .await
        .expect("token query should succeed");
    assert!(token.is_some());
    let admin_token = context.token(ADMIN).await;
    let forbidden = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/admin/access-requests")
            .to_request(),
    )
    .await;
    assert_error(forbidden, StatusCode::UNAUTHORIZED, "unauthorized").await;
    let listed = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/admin/access-requests")
            .insert_header(("Authorization", format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
}
