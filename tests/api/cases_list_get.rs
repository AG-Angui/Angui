use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{ADMIN, COMMANDER, FAMILY, LEARNER, TestContext, assert_error};

#[actix_web::test]
async fn get_cases_only_returns_cases_where_the_user_is_a_member() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family_token = context.token(FAMILY).await;
    let commander_token = context.token(COMMANDER).await;
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

    let absent_member = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/cases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    let absent_cases: Value = test::read_body_json(absent_member).await;
    assert_eq!(absent_cases, Value::Array(vec![]));

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
    let learner_token = context.token(LEARNER).await;
    let admin_token = context.token(ADMIN).await;
    let app = crate::init_api_app!(&context);

    for token in [learner_token, admin_token] {
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
