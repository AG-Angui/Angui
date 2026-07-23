use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{COMMANDER, FAMILY, TestContext, VOLUNTEER, assert_error, create_clue_json};

#[actix_web::test]
async fn post_case_clues_creates_pending_review_clues_for_case_members() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(create_clue_json())
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["case_id"], case_id);
    assert_eq!(body["status"], "pending_review");
    assert_eq!(body["is_own_submission"], true);
}

#[actix_web::test]
async fn post_case_clues_hides_non_member_cases_and_rejects_closed_cases() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let non_member = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .set_json(create_clue_json())
            .to_request(),
    )
    .await;
    assert_error(non_member, StatusCode::NOT_FOUND, "not_found").await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context.close_case(&case_id).await;
    let closed = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(create_clue_json())
            .to_request(),
    )
    .await;
    assert_error(closed, StatusCode::CONFLICT, "conflict").await;
}
