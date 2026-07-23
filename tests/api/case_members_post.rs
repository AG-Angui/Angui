use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{COMMANDER, FAMILY, TestContext, VOLUNTEER, assert_error};

#[actix_web::test]
async fn post_case_members_applies_invitation_role_and_duplicate_rules() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let invite_commander = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "email": COMMANDER, "role": "commander" }))
            .to_request(),
    )
    .await;
    assert_eq!(invite_commander.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(invite_commander).await;
    assert_eq!(body["email"], COMMANDER);
    assert_eq!(body["role"], "commander");

    let duplicate = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "email": COMMANDER, "role": "commander" }))
            .to_request(),
    )
    .await;
    assert_error(duplicate, StatusCode::CONFLICT, "conflict").await;
    let family_cannot_invite_volunteer = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "email": VOLUNTEER, "role": "volunteer" }))
            .to_request(),
    )
    .await;
    assert_error(
        family_cannot_invite_volunteer,
        StatusCode::FORBIDDEN,
        "forbidden",
    )
    .await;
}

#[actix_web::test]
async fn post_case_members_allows_a_commander_to_add_a_matching_volunteer() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "email": VOLUNTEER, "role": "volunteer" }))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}
