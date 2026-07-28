use actix_web::{
    http::{StatusCode, header},
    test,
};
use angui::entities::clues;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set};
use serde_json::{Value, json};

use crate::support::{COMMANDER, FAMILY, LEARNER, TestContext, VOLUNTEER, assert_error};

#[actix_web::test]
async fn get_case_summary_classifies_review_states_and_crops_each_role() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let app = crate::init_api_app!(&context);
    let commander_token = context.token(COMMANDER).await;
    let volunteer = context.authenticated(VOLUNTEER).await;

    let confirmed_id = context.create_clue(&case_id, FAMILY).await;
    let latest_confirmed_id = context.create_clue(&case_id, FAMILY).await;
    let own_pending_id = context.create_clue(&case_id, FAMILY).await;
    let internal_pending_id = context.create_clue(&case_id, COMMANDER).await;
    let excluded_id = context.create_clue(&case_id, COMMANDER).await;
    set_reported_at(&context, &confirmed_id, "2026-07-13T09:10:00Z").await;
    set_reported_at(&context, &latest_confirmed_id, "2026-07-13T09:20:00Z").await;
    for (clue_id, status, reason) in [
        (&confirmed_id, "confirmed", "fictional source verified"),
        (
            &latest_confirmed_id,
            "confirmed",
            "fictional later source verified",
        ),
        (&excluded_id, "rejected", "fictional report disproven"),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::patch()
                .uri(&format!("/api/clues/{clue_id}/review"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
                .set_json(json!({ "status": status, "reason": reason }))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    let task_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({
                "source_clue_id": latest_confirmed_id,
                "volunteer_user_id": volunteer.id,
                "title": "Verify fictional north gate",
                "objective": "Check the fictional reported route and submit observations.",
                "area_text": "Fictional north gate",
                "latitude": 31.8206,
                "longitude": 117.2272,
                "due_at": "2099-07-27T12:00:00Z",
                "background": "Commander-only fictional direction.",
                "risk_level": "medium",
                "risk_notes": "Stay in public areas.",
                "safety_briefing": "Keep contact with the commander.",
                "expected_feedback": "Submit a factual text report."
            }))
            .to_request(),
    )
    .await;
    assert_eq!(task_response.status(), StatusCode::CREATED);

    let request_for = |token: String| {
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/summary"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request()
    };

    let commander: Value = test::read_body_json(
        test::call_service(&app, request_for(context.token(COMMANDER).await)).await,
    )
    .await;
    assert_eq!(commander["access_role"], "commander");
    assert!(commander["generated_at"].as_str().is_some());
    assert_eq!(
        commander["last_confirmed_information"]["status"],
        "confirmed"
    );
    assert_eq!(
        commander["last_confirmed_information"]["clue_id"],
        latest_confirmed_id
    );
    assert_eq!(
        commander["confirmed_clues"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(
        commander["pending_verification"].as_array().map(Vec::len),
        Some(2)
    );
    assert!(
        commander["pending_verification"]
            .as_array()
            .expect("pending items")
            .iter()
            .all(|item| item["status"] != "confirmed")
    );
    assert_eq!(commander["excluded_directions"][0]["clue_id"], excluded_id);
    assert_eq!(commander["current_focus"].as_array().map(Vec::len), Some(1));
    assert_eq!(commander["task_status"].as_array().map(Vec::len), Some(1));
    assert!(
        commander["source_scope"]
            .as_array()
            .expect("source scope")
            .iter()
            .any(|scope| scope == "all_case_tasks")
    );

    let family: Value = test::read_body_json(
        test::call_service(&app, request_for(context.token(FAMILY).await)).await,
    )
    .await;
    assert_eq!(family["access_role"], "family");
    assert_eq!(family["confirmed_clues"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        family["last_confirmed_information"]["clue_id"],
        latest_confirmed_id
    );
    assert_eq!(family["pending_verification"][0]["clue_id"], own_pending_id);
    assert_ne!(
        family["pending_verification"][0]["clue_id"],
        internal_pending_id
    );
    assert_eq!(family["excluded_directions"], json!([]));
    assert_eq!(family["current_focus"], json!([]));
    assert_eq!(family["task_status"], json!([]));

    let volunteer: Value = test::read_body_json(
        test::call_service(&app, request_for(context.token(VOLUNTEER).await)).await,
    )
    .await;
    assert_eq!(volunteer["access_role"], "volunteer");
    assert!(volunteer["last_confirmed_information"].is_null());
    assert_eq!(volunteer["confirmed_clues"], json!([]));
    assert_eq!(volunteer["pending_verification"], json!([]));
    assert_eq!(volunteer["excluded_directions"], json!([]));
    assert_eq!(volunteer["current_focus"], json!([]));
    assert_eq!(volunteer["task_status"].as_array().map(Vec::len), Some(1));
    assert!(volunteer["task_status"][0].get("background").is_none());
    assert!(
        volunteer["source_scope"]
            .as_array()
            .expect("source scope")
            .iter()
            .all(|scope| scope == "own_assigned_tasks")
    );

    let hidden = test::call_service(&app, request_for(context.token(LEARNER).await)).await;
    assert_error(hidden, StatusCode::NOT_FOUND, "not_found").await;
}

async fn set_reported_at(context: &TestContext, clue_id: &str, reported_at: &str) {
    let clue = clues::Entity::find_by_id(clue_id)
        .one(&context.database)
        .await
        .expect("fixture clue should load")
        .expect("fixture clue should exist");
    let mut active = clue.into_active_model();
    active.reported_at = Set(reported_at.to_owned());
    active
        .update(&context.database)
        .await
        .expect("fixture clue update");
}

#[actix_web::test]
async fn get_case_summary_returns_empty_deterministic_sections_without_ai() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/summary"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(FAMILY).await),
            ))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = test::read_body_json(response).await;
    assert!(body["generated_at"].as_str().is_some());
    for section in [
        "confirmed_clues",
        "pending_verification",
        "excluded_directions",
        "current_focus",
        "task_status",
    ] {
        assert_eq!(body[section], json!([]));
    }
    assert!(body["last_confirmed_information"].is_null());
    assert!(
        !body["safety_reminders"]
            .as_array()
            .expect("reminders")
            .is_empty()
    );
}
