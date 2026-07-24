use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{ADMIN, FAMILY, LEARNER, TestContext, assert_error, create_case_json};

#[actix_web::test]
async fn post_cases_creates_an_active_case_for_a_family_member() {
    let context = TestContext::new().await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/cases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(create_case_json())
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["status"], "active");
    assert_eq!(body["access_role"], "family");
    assert_eq!(body["elder_profile"]["display_name"], "测试老人");
}

#[actix_web::test]
async fn post_cases_allows_operational_members_and_validates_requests() {
    let context = TestContext::new().await;
    let learner_token = context.token(LEARNER).await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let forbidden = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/cases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {learner_token}")))
            .set_json(create_case_json())
            .to_request(),
    )
    .await;
    assert_error(forbidden, StatusCode::FORBIDDEN, "forbidden").await;
    let invalid = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/cases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "display_name": "测试老人" }))
            .to_request(),
    )
    .await;
    assert_error(invalid, StatusCode::BAD_REQUEST, "validation_error").await;
}

#[actix_web::test]
async fn learner_cannot_create_cases_but_admin_capability_does_not_block_member_creation() {
    let context = TestContext::new().await;
    let learner_token = context.token(LEARNER).await;
    let admin_token = context.token(ADMIN).await;
    let app = crate::init_api_app!(&context);

    let learner = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/cases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {learner_token}")))
            .set_json(create_case_json())
            .to_request(),
    )
    .await;
    assert_error(learner, StatusCode::FORBIDDEN, "forbidden").await;

    let admin = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/cases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(create_case_json())
            .to_request(),
    )
    .await;
    assert_eq!(admin.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(admin).await;
    assert_eq!(body["access_role"], "family");
}
