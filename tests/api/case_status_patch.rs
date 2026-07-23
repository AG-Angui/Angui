use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{COMMANDER, FAMILY, TestContext, assert_error};

#[actix_web::test]
async fn patch_case_status_requires_commander_and_enforces_the_state_machine() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let family_token = context.token(FAMILY).await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);

    let forbidden = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/status"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "status": "resolved" }))
            .to_request(),
    )
    .await;
    assert_error(forbidden, StatusCode::FORBIDDEN, "forbidden").await;
    let resolved = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/status"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "status": "resolved" }))
            .to_request(),
    )
    .await;
    assert_eq!(resolved.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resolved).await;
    assert_eq!(body["status"], "resolved");
    let closed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/status"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "status": "closed" }))
            .to_request(),
    )
    .await;
    assert_eq!(closed.status(), StatusCode::OK);
    let invalid_transition = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/status"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "status": "active" }))
            .to_request(),
    )
    .await;
    assert_error(invalid_transition, StatusCode::CONFLICT, "conflict").await;
}
