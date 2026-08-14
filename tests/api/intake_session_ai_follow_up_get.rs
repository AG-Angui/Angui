use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use angui::{
    models::{CreateIntakeSessionRequest, IntakeInitialAnswers},
    services::intake_session_service,
};

use crate::support::{COMMANDER, FAMILY, TestContext, assert_error};

async fn create_session(context: &TestContext, answers: IntakeInitialAnswers) -> String {
    let family = context.authenticated(FAMILY).await;
    intake_session_service::create_intake_session(
        &context.database,
        &family,
        CreateIntakeSessionRequest {
            initial_answers: answers,
        },
        2_000,
    )
    .await
    .expect("fixture intake session should be created")
    .id
}

#[actix_web::test]
async fn get_ai_follow_up_returns_a_static_optional_phase_two_question_when_ai_is_unavailable() {
    let context = TestContext::new().await;
    let session_id = create_session(
        &context,
        IntakeInitialAnswers {
            basic_information: Some("Fictional elder profile".to_owned()),
            health_status: Some("No known health notes".to_owned()),
            behavior_habits: Some("Enjoys a daily walk".to_owned()),
            last_seen: Some("Fictional community gate".to_owned()),
            ..Default::default()
        },
    )
    .await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/intake-sessions/{session_id}/ai-follow-up"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["degradation_status"], "rule_based_fallback");
    assert_eq!(body["question"]["field"], "frequent_locations");
    assert_eq!(body["question"]["skippable"], true);
    assert!(
        body["question"]["missing_fields"]
            .as_array()
            .is_some_and(|fields| fields.iter().any(|field| field == "frequent_locations"))
    );
    assert!(body["generated_at"].as_str().is_some());
}

#[actix_web::test]
async fn get_ai_follow_up_returns_a_static_current_phase_question_before_phase_two() {
    let context = TestContext::new().await;
    let session_id = create_session(
        &context,
        IntakeInitialAnswers {
            basic_information: Some("Fictional elder profile".to_owned()),
            ..Default::default()
        },
    )
    .await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/intake-sessions/{session_id}/ai-follow-up"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["degradation_status"], "rule_based_fallback");
    assert_eq!(body["question"]["field"], "health_status");
    assert_eq!(body["question"]["skippable"], true);
}

#[actix_web::test]
async fn get_ai_follow_up_hides_another_members_session() {
    let context = TestContext::new().await;
    let session_id = create_session(
        &context,
        IntakeInitialAnswers {
            basic_information: Some("Fictional elder profile".to_owned()),
            ..Default::default()
        },
    )
    .await;
    let token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/intake-sessions/{session_id}/ai-follow-up"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;

    assert_error(response, StatusCode::NOT_FOUND, "not_found").await;
}
