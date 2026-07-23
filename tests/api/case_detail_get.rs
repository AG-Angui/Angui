use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{COMMANDER, FAMILY, TestContext, VOLUNTEER, assert_error};

#[actix_web::test]
async fn get_case_applies_membership_and_role_based_field_and_clue_cuts() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family_token = context.token(FAMILY).await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let app = crate::init_api_app!(&context);

    let non_member = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    assert_error(non_member, StatusCode::NOT_FOUND, "not_found").await;

    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    context.create_clue(&case_id, FAMILY).await;

    let family = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    let family_body: Value = test::read_body_json(family).await;
    assert_eq!(family_body["access_role"], "family");
    assert_eq!(family_body["clues"].as_array().map(Vec::len), Some(1));

    let volunteer = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    let volunteer_body: Value = test::read_body_json(volunteer).await;
    assert_eq!(volunteer_body["access_role"], "volunteer");
    assert!(volunteer_body["elder_profile"]["health_notes"].is_null());
    assert_eq!(volunteer_body["clues"].as_array().map(Vec::len), Some(0));
}
