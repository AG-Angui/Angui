use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{COMMANDER, FAMILY, TestContext, assert_error};

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
