use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{COMMANDER, FAMILY, TestContext, assert_error};

#[actix_web::test]
async fn patch_clue_review_requires_the_case_commander_and_publishes_review_status() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let clue_id = context.create_clue(&case_id, FAMILY).await;
    let family_token = context.token(FAMILY).await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);
    let forbidden = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "status": "confirmed" }))
            .to_request(),
    )
    .await;
    assert_error(forbidden, StatusCode::FORBIDDEN, "forbidden").await;
    let confirmed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "status": "confirmed" }))
            .to_request(),
    )
    .await;
    assert_eq!(confirmed.status(), StatusCode::OK);
    let body: Value = test::read_body_json(confirmed).await;
    assert_eq!(body["status"], "confirmed");
    assert!(body["reviewed_at"].is_string());
}

#[actix_web::test]
async fn patch_clue_review_rejects_pending_review_as_a_review_target() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let clue_id = context.create_clue(&case_id, FAMILY).await;
    let token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(serde_json::json!({ "status": "pending_review" }))
            .to_request(),
    )
    .await;
    assert_error(response, StatusCode::BAD_REQUEST, "validation_error").await;
}
