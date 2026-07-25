use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{ADMIN, COMMANDER, FAMILY, LEARNER, TestContext, VOLUNTEER, assert_error};

#[actix_web::test]
async fn get_cases_only_returns_cases_where_the_user_is_a_member() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let family_token = context.token(FAMILY).await;
    let commander_token = context.token(COMMANDER).await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let app = crate::init_api_app!(&context);

    let family_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/cases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(family_response.status(), StatusCode::OK);
    let family_cases: Value = test::read_body_json(family_response).await;
    assert_eq!(family_cases[0]["id"], case_id);
    assert_eq!(family_cases[0]["access_role"], "family");
    let family_case = family_cases[0].as_object().expect("case list item object");
    for private_field in ["health_notes", "clues", "members", "attachments", "places"] {
        assert!(
            !family_case.contains_key(private_field),
            "case list must not expose {private_field}"
        );
    }

    let commander_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/cases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    let commander_cases: Value = test::read_body_json(commander_response).await;
    assert_eq!(commander_cases[0]["id"], case_id);
    assert_eq!(commander_cases[0]["access_role"], "commander");

    let volunteer_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/cases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    let volunteer_cases: Value = test::read_body_json(volunteer_response).await;
    assert_eq!(volunteer_cases[0]["id"], case_id);
    assert_eq!(volunteer_cases[0]["access_role"], "volunteer");

    let missing = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/cases").to_request(),
    )
    .await;
    assert_error(missing, StatusCode::UNAUTHORIZED, "unauthorized").await;
}

#[actix_web::test]
async fn learner_and_admin_capability_do_not_gain_case_access_without_membership() {
    let context = TestContext::new().await;
    context.create_case().await;
    let commander_token = context.token(COMMANDER).await;
    let learner_token = context.token(LEARNER).await;
    let admin_token = context.token(ADMIN).await;
    let app = crate::init_api_app!(&context);

    for token in [commander_token, learner_token, admin_token] {
        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/cases")
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let cases: Value = test::read_body_json(response).await;
        assert_eq!(cases, Value::Array(vec![]));
    }
}
