use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{ADMIN, COMMANDER, FAMILY, LEARNER, TestContext, VOLUNTEER, assert_error};

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
            .set_json(serde_json::json!({ "email": COMMANDER, "case_role": "commander" }))
            .to_request(),
    )
    .await;
    assert_eq!(invite_commander.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(invite_commander).await;
    assert_eq!(body["email"], COMMANDER);
    assert_eq!(body["global_role"], "commander");
    assert_eq!(body["case_role"], "commander");

    let duplicate = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "email": COMMANDER, "case_role": "commander" }))
            .to_request(),
    )
    .await;
    assert_error(duplicate, StatusCode::CONFLICT, "conflict").await;
    let family_cannot_invite_volunteer = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "email": VOLUNTEER, "case_role": "volunteer" }))
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
async fn post_case_members_allows_a_commander_to_assign_an_operational_account() {
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
            .set_json(serde_json::json!({ "email": VOLUNTEER, "case_role": "volunteer" }))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[actix_web::test]
async fn post_case_members_separates_global_identity_from_case_role() {
    let context = TestContext::new().await;
    let commander = context.authenticated(COMMANDER).await;
    let commander_case = angui::services::case_service::create_case(
        &context.database,
        &commander,
        crate::support::create_case_request(),
    )
    .await
    .expect("commander should create a case")
    .id;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);

    let assigned = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{commander_case}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "email": FAMILY, "case_role": "volunteer" }))
            .to_request(),
    )
    .await;
    assert_eq!(assigned.status(), StatusCode::CREATED);
    let assigned_body: Value = test::read_body_json(assigned).await;
    assert_eq!(assigned_body["global_role"], "family");
    assert_eq!(assigned_body["case_role"], "volunteer");

    let family_token = context.token(FAMILY).await;
    let detail = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{commander_case}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body: Value = test::read_body_json(detail).await;
    assert_eq!(detail_body["access_role"], "volunteer");

    for email in [LEARNER, ADMIN] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/cases/{commander_case}/members"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
                .set_json(serde_json::json!({ "email": email, "case_role": "volunteer" }))
                .to_request(),
        )
        .await;
        assert_error(response, StatusCode::FORBIDDEN, "forbidden").await;
    }
}
