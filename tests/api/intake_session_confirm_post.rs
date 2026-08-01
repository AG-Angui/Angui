use actix_web::{
    http::{StatusCode, header},
    test,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::{Value, json};

use angui::{
    entities::{audit_events, cases, intake_sessions},
    models::{CreateIntakeSessionRequest, IntakeInitialAnswers},
    services::intake_session_service,
};

use crate::support::{COMMANDER, FAMILY, TestContext, assert_error};

async fn ready_session(context: &TestContext) -> String {
    let family = context.authenticated(FAMILY).await;
    let session = intake_session_service::create_intake_session(
        &context.database,
        &family,
        CreateIntakeSessionRequest {
            initial_answers: IntakeInitialAnswers {
                basic_information: Some("Fictional basic information.".to_owned()),
                last_seen: Some("Fictional last seen location.".to_owned()),
                ..Default::default()
            },
        },
        2_000,
    )
    .await
    .expect("fixture session should be created");
    intake_session_service::submit_intake_answer(
        &context.database,
        &family,
        &session.id,
        angui::models::SubmitIntakeAnswerRequest {
            field: "health_status".to_owned(),
            answer: "Draft health note that the family will correct.".to_owned(),
            replace: false,
            structured: None,
        },
        2_000,
    )
    .await
    .expect("required answers should make the session ready");
    session.id
}

#[actix_web::test]
async fn post_confirm_rejects_blocking_intake_assessments_without_creating_a_case() {
    let context = TestContext::new().await;
    let family = context.authenticated(FAMILY).await;
    let session = intake_session_service::create_intake_session(
        &context.database,
        &family,
        CreateIntakeSessionRequest {
            initial_answers: IntakeInitialAnswers {
                basic_information: Some("Fictional basic information.".to_owned()),
                last_seen: Some("Fictional last seen location.".to_owned()),
                ..Default::default()
            },
        },
        2_000,
    )
    .await
    .expect("fixture session should be created");
    intake_session_service::submit_intake_answer(
        &context.database,
        &family,
        &session.id,
        angui::models::SubmitIntakeAnswerRequest {
            field: "health_status".to_owned(),
            answer: "Family-provided draft health note.".to_owned(),
            replace: false,
            structured: Some(angui::models::IntakeStructuredFacts {
                last_seen_at: Some("2026-07-25T15:00:00+08:00".to_owned()),
                follow_up_at: Some("2026-07-25T14:30:00+08:00".to_owned()),
                last_seen_location: Some(angui::models::IntakeLocation {
                    name: "Fictional origin".to_owned(),
                    longitude: 114.48,
                    latitude: 36.61,
                    coordinate_system: "gcj02".to_owned(),
                }),
                follow_up_location: Some(angui::models::IntakeLocation {
                    name: "Fictional destination".to_owned(),
                    longitude: 114.58,
                    latitude: 36.61,
                    coordinate_system: "gcj02".to_owned(),
                }),
                transport_modes: vec!["walking".to_owned()],
                ..Default::default()
            }),
        },
        2_000,
    )
    .await
    .expect("structured draft answer should be recorded");

    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let blocked = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/intake-sessions/{}/confirm", session.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(confirmation_json())
            .to_request(),
    )
    .await;
    assert_error(blocked, StatusCode::CONFLICT, "conflict").await;
    assert_eq!(
        cases::Entity::find()
            .count(&context.database)
            .await
            .unwrap(),
        0
    );

    let draft = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/intake-sessions/{}/profile-draft",
                session.id
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;
    assert_eq!(draft.status(), StatusCode::OK);
    let draft_body: Value = test::read_body_json(draft).await;
    assert_eq!(draft_body["assessments"][0]["severity"], "blocking");
    assert!(
        draft_body["confirmation_blocked_reasons"]
            .as_array()
            .is_some_and(|reasons| !reasons.is_empty())
    );
}

fn confirmation_json() -> Value {
    json!({
        "human_confirmed": true,
        "profile": {
            "display_name": "Fictional confirmed elder",
            "age": 76,
            "gender": "female",
            "physical_description": "Family-corrected physical description",
            "clothing_description": "Family-corrected blue coat",
            "health_notes": "Family-corrected health note",
            "last_seen_at": "2026-07-25T09:00:00Z",
            "last_seen_location": "Fictional confirmed community gate"
        }
    })
}

#[actix_web::test]
async fn post_confirm_creates_one_active_case_from_human_confirmed_overrides() {
    let context = TestContext::new().await;
    let session_id = ready_session(&context).await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);

    let initial_review = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!(
                "/api/intake-sessions/{session_id}/ai-initial-review"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(json!({ "profile": confirmation_json()["profile"].clone() }))
            .to_request(),
    )
    .await;
    assert_eq!(initial_review.status(), StatusCode::CREATED);
    let initial_body: Value = test::read_body_json(initial_review).await;
    assert_eq!(initial_body["status"], "awaiting_family_review");
    assert_eq!(initial_body["degradation_status"], "rule_based_fallback");

    let acknowledgement = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!(
                "/api/intake-sessions/{session_id}/ai-initial-review/acknowledge"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(json!({ "human_confirmed": true, "confirmed_issue_ids": [] }))
            .to_request(),
    )
    .await;
    assert_eq!(acknowledgement.status(), StatusCode::OK);
    let acknowledgement_body: Value = test::read_body_json(acknowledgement).await;
    assert_eq!(
        acknowledgement_body["status"],
        "ready_for_second_confirmation"
    );

    let first = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/intake-sessions/{session_id}/confirm"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(confirmation_json())
            .to_request(),
    )
    .await;
    assert_eq!(first.status(), StatusCode::CREATED);
    let first_body: Value = test::read_body_json(first).await;
    assert_eq!(first_body["status"], "active");
    assert_eq!(
        first_body["confirmation_status"],
        "human_confirmed_after_ai_initial_review"
    );
    let case_id = first_body["case_id"]
        .as_str()
        .expect("response includes case id")
        .to_owned();

    let second = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/intake-sessions/{session_id}/confirm"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(confirmation_json())
            .to_request(),
    )
    .await;
    assert_eq!(second.status(), StatusCode::CREATED);
    let second_body: Value = test::read_body_json(second).await;
    assert_eq!(second_body["case_id"], case_id);

    let case_count = cases::Entity::find()
        .count(&context.database)
        .await
        .unwrap();
    assert_eq!(case_count, 1);
    let session = intake_sessions::Entity::find_by_id(&session_id)
        .one(&context.database)
        .await
        .unwrap()
        .expect("session exists");
    assert_eq!(session.status, "confirmed");
    assert_eq!(session.case_id.as_deref(), Some(case_id.as_str()));
    assert!(session.confirmed_by_user_id.is_some());

    let detail = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body: Value = test::read_body_json(detail).await;
    assert_eq!(
        detail_body["elder_profile"]["health_notes"],
        "Family-corrected health note"
    );

    let created_audit = audit_events::Entity::find()
        .filter(audit_events::Column::CaseId.eq(&case_id))
        .filter(audit_events::Column::Action.eq("case.created"))
        .one(&context.database)
        .await
        .unwrap();
    assert!(created_audit.is_some());
}

#[actix_web::test]
async fn post_confirm_accepts_the_two_runtime_required_profile_fields() {
    let context = TestContext::new().await;
    let session_id = ready_session(&context).await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let profile = json!({
        "display_name": "Minimal fictional elder",
        "last_seen_location": "Fictional community gate"
    });

    let initial_review = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!(
                "/api/intake-sessions/{session_id}/ai-initial-review"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(json!({ "profile": profile.clone() }))
            .to_request(),
    )
    .await;
    assert_eq!(initial_review.status(), StatusCode::CREATED);
    let acknowledgement = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!(
                "/api/intake-sessions/{session_id}/ai-initial-review/acknowledge"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(json!({ "human_confirmed": true, "confirmed_issue_ids": [] }))
            .to_request(),
    )
    .await;
    assert_eq!(acknowledgement.status(), StatusCode::OK);

    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/intake-sessions/{session_id}/confirm"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(json!({ "human_confirmed": true, "profile": profile }))
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["status"], "active");
}

#[actix_web::test]
async fn post_confirm_requires_creator_human_confirmation_and_valid_profile_without_partial_case() {
    let context = TestContext::new().await;
    let session_id = ready_session(&context).await;
    let family_token = context.token(FAMILY).await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);

    let non_creator = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/intake-sessions/{session_id}/confirm"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(confirmation_json())
            .to_request(),
    )
    .await;
    assert_error(non_creator, StatusCode::NOT_FOUND, "not_found").await;

    let not_confirmed = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/intake-sessions/{session_id}/confirm"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({ "human_confirmed": false, "profile": confirmation_json()["profile"].clone() }))
            .to_request(),
    )
    .await;
    assert_error(not_confirmed, StatusCode::BAD_REQUEST, "validation_error").await;

    let invalid_profile = json!({
        "display_name": "Fictional",
        "last_seen_location": ""
    });
    let initial_review = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!(
                "/api/intake-sessions/{session_id}/ai-initial-review"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({ "profile": invalid_profile.clone() }))
            .to_request(),
    )
    .await;
    assert_eq!(initial_review.status(), StatusCode::CREATED);
    let acknowledgement = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!(
                "/api/intake-sessions/{session_id}/ai-initial-review/acknowledge"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({ "human_confirmed": true, "confirmed_issue_ids": [] }))
            .to_request(),
    )
    .await;
    assert_eq!(acknowledgement.status(), StatusCode::OK);

    let invalid = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/intake-sessions/{session_id}/confirm"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({ "human_confirmed": true, "profile": invalid_profile }))
            .to_request(),
    )
    .await;
    assert_error(invalid, StatusCode::BAD_REQUEST, "validation_error").await;
    assert_eq!(
        cases::Entity::find()
            .count(&context.database)
            .await
            .unwrap(),
        0
    );
}
