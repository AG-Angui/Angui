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
        },
        2_000,
    )
    .await
    .expect("required answers should make the session ready");
    session.id
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
    assert_eq!(first_body["confirmation_status"], "human_confirmed");
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

    let invalid = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/intake-sessions/{session_id}/confirm"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({ "human_confirmed": true, "profile": { "display_name": "Fictional", "last_seen_location": "" } }))
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
