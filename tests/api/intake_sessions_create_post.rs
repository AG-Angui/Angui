use actix_web::{
    http::{StatusCode, header},
    test,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set};
use serde_json::Value;

use angui::{
    entities::{audit_events, intake_question_definitions, intake_sessions},
    models::{CreateIntakeSessionRequest, IntakeInitialAnswers},
    services::intake_session_service,
};

use crate::support::{FAMILY, LEARNER, TestContext, assert_error};

#[actix_web::test]
async fn post_intake_sessions_creates_a_family_owned_rule_guided_draft() {
    let context = TestContext::new().await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/intake-sessions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "initial_answers": {
                    "basic_information": "  Fictional elder profile  ",
                    "last_seen": "Fictional community gate, approximate time"
                }
            }))
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["status"], "ready_for_confirmation");
    assert_eq!(body["question_set_version"], 2);
    assert_eq!(body["guidance_mode"], "rule_based");
    assert_eq!(
        body["initial_answers"]["basic_information"],
        "Fictional elder profile"
    );
    assert_eq!(body["next_question"]["field"], "frequent_locations");
    assert!(
        body["missing_fields"]
            .as_array()
            .unwrap()
            .contains(&Value::String("health_status".to_owned()))
    );

    let session_id = body["id"].as_str().unwrap();
    let stored = intake_sessions::Entity::find_by_id(session_id)
        .one(&context.database)
        .await
        .unwrap()
        .expect("session should be stored");
    assert_eq!(stored.status, "ready_for_confirmation");
    assert!(stored.answers_json.contains("Fictional elder profile"));

    let audit = audit_events::Entity::find()
        .filter(audit_events::Column::EntityId.eq(session_id))
        .one(&context.database)
        .await
        .unwrap()
        .expect("creation should be audited");
    assert_eq!(audit.action, "intake_session.created");
    assert!(
        !audit
            .metadata_json
            .unwrap_or_default()
            .contains("Fictional elder profile")
    );
}

#[actix_web::test]
async fn post_intake_sessions_reads_question_order_prompt_and_limit_from_database() {
    let context = TestContext::new().await;
    let health_question = intake_question_definitions::Entity::find()
        .filter(intake_question_definitions::Column::FieldCode.eq("health_status"))
        .filter(intake_question_definitions::Column::Status.eq("active"))
        .one(&context.database)
        .await
        .unwrap()
        .expect("seeded health question should exist");
    let mut configured_question = health_question.into_active_model();
    configured_question.prompt = Set("Configured health question".to_owned());
    configured_question.max_answer_chars = Set(5);
    configured_question.update(&context.database).await.unwrap();

    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let configured_prompt = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/intake-sessions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "initial_answers": { "basic_information": "Fictional elder" }
            }))
            .to_request(),
    )
    .await;
    assert_eq!(configured_prompt.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(configured_prompt).await;
    assert_eq!(body["next_question"]["field"], "health_status");
    assert_eq!(
        body["next_question"]["prompt"],
        "Configured health question"
    );

    let limit_from_database = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/intake-sessions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "initial_answers": { "health_status": "123456" }
            }))
            .to_request(),
    )
    .await;
    assert_error(
        limit_from_database,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;
}

#[actix_web::test]
async fn intake_answer_hard_max_caps_the_database_question_limit() {
    let context = TestContext::new().await;
    let family = context.authenticated(FAMILY).await;
    let result = intake_session_service::create_intake_session(
        &context.database,
        &family,
        CreateIntakeSessionRequest {
            initial_answers: IntakeInitialAnswers {
                basic_information: Some("123456".to_owned()),
                ..Default::default()
            },
        },
        5,
    )
    .await;
    assert!(matches!(result, Err(angui::error::ApiError::Validation(_))));
}

#[actix_web::test]
async fn post_intake_sessions_rejects_learner_accounts() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);

    for email in [LEARNER] {
        let token = context.token(email).await;
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/intake-sessions")
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .set_json(serde_json::json!({}))
                .to_request(),
        )
        .await;
        assert_error(response, StatusCode::FORBIDDEN, "forbidden").await;
    }
}

#[actix_web::test]
async fn post_intake_sessions_validates_answers_and_refuses_client_confirmed_facts() {
    let context = TestContext::new().await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);

    for payload in [
        serde_json::json!({ "initial_answers": { "health_status": "   " } }),
        serde_json::json!({ "initial_answers": { "basic_information": "x".repeat(1001) } }),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/intake-sessions")
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .set_json(payload)
                .to_request(),
        )
        .await;
        assert_error(response, StatusCode::BAD_REQUEST, "validation_error").await;
    }

    let rejected_confirmed_fact = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/intake-sessions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "initial_answers": { "basic_information": "Fictional elder", "confirmed": true }
            }))
            .to_request(),
    )
    .await;
    assert_eq!(rejected_confirmed_fact.status(), StatusCode::BAD_REQUEST);
}
