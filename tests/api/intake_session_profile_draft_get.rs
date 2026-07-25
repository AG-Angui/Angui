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

async fn create_session(context: &TestContext) -> String {
    let family = context.authenticated(FAMILY).await;
    intake_session_service::create_intake_session(
        &context.database,
        &family,
        CreateIntakeSessionRequest {
            initial_answers: IntakeInitialAnswers {
                basic_information: Some(
                    "Fictional elder uses a cane and wears a blue coat.".to_owned(),
                ),
                health_status: Some(
                    "Fictional health information that requires confirmation.".to_owned(),
                ),
                last_seen: Some("Fictional community gate; time is not yet verified.".to_owned()),
                frequent_locations: Some("Fictional neighborhood park.".to_owned()),
                ..Default::default()
            },
        },
        2_000,
    )
    .await
    .expect("fixture session should be created")
    .id
}

#[actix_web::test]
async fn get_profile_draft_returns_only_unconfirmed_family_source_data() {
    let context = TestContext::new().await;
    let session_id = create_session(&context).await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/intake-sessions/{session_id}/profile-draft"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["status"], "draft");
    assert_eq!(body["requires_human_confirmation"], true);
    assert_eq!(
        body["source_scope"],
        "family_provided intake answers from this session only"
    );
    assert_eq!(
        body["profile"]["health_notes"],
        "Fictional health information that requires confirmation."
    );
    assert_eq!(
        body["profile"]["last_seen_information"],
        "Fictional community gate; time is not yet verified."
    );
    let health_notes_metadata = body["field_metadata"]
        .as_array()
        .expect("profile draft metadata must be an array")
        .iter()
        .find(|metadata| metadata["field"] == "health_notes")
        .expect("health notes must include field-level draft metadata");
    assert_eq!(health_notes_metadata["source_field"], "health_status");
    assert_eq!(health_notes_metadata["source"], "family_provided");
    assert_eq!(health_notes_metadata["status"], "draft");
    assert!(health_notes_metadata["generated_at"].is_string());
    assert!(
        body["missing_fields"]
            .as_array()
            .is_some_and(|fields| { fields.iter().any(|field| field == "behavior_habits") })
    );
    assert_eq!(body["direction_hypotheses"][0]["status"], "hypothesis");
    assert_eq!(
        body["direction_hypotheses"][0]["source_fields"][0],
        "frequent_locations"
    );
}

#[actix_web::test]
async fn get_profile_draft_hides_sensitive_answers_from_non_creators() {
    let context = TestContext::new().await;
    let session_id = create_session(&context).await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/intake-sessions/{session_id}/profile-draft"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;

    assert_error(response, StatusCode::NOT_FOUND, "not_found").await;
}
