use actix_web::{
    http::{StatusCode, header},
    test,
};
use angui::entities::tasks;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{Value, json};

use crate::support::{COMMANDER, FAMILY, LEARNER, TestContext, VOLUNTEER, assert_error};

#[actix_web::test]
async fn get_case_map_view_returns_only_role_necessary_layers_with_text_fallbacks() {
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
    let clue_id = context.create_clue(&case_id, FAMILY).await;
    let reviewed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "status": "confirmed", "reason": "fictional source verified" }))
            .to_request(),
    )
    .await;
    assert_eq!(reviewed.status(), StatusCode::OK);
    let task = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({
                "source_clue_id": clue_id.clone(),
                "volunteer_user_id": volunteer.id,
                "title": "Verify fictional north gate",
                "objective": "Check the fictional reported route and submit observations.",
                "area_text": "Fictional north gate",
                "latitude": 31.8206,
                "longitude": 117.2272,
                "due_at": "2099-07-27T12:00:00Z",
                "background": "Fictional confirmed clue.",
                "risk_level": "medium",
                "risk_notes": "Stay in public areas and do not enter restricted property.",
                "safety_briefing": "Keep contact with the commander and stop if conditions change.",
                "expected_feedback": "Submit a factual text report for commander review."
            }))
            .to_request(),
    )
    .await;
    assert_eq!(task.status(), StatusCode::CREATED);
    let task: Value = test::read_body_json(task).await;
    let task_id = task["id"].as_str().expect("task id");
    let commander = context.authenticated(COMMANDER).await;
    for index in 0..100 {
        tasks::ActiveModel {
            id: Set(format!("map-extra-task-{index}")),
            case_id: Set(case_id.clone()),
            source_clue_id: Set(Some(clue_id.clone())),
            title: Set(format!("Fictional extra task {index}")),
            objective: Set("Fictional task objective.".to_owned()),
            area_text: Set("Fictional task area".to_owned()),
            latitude: Set(None),
            longitude: Set(None),
            due_at: Set("2099-07-27T12:00:00Z".to_owned()),
            background: Set("Fictional confirmed clue.".to_owned()),
            risk_level: Set("medium".to_owned()),
            risk_notes: Set("Fictional safety note.".to_owned()),
            safety_briefing: Set("Fictional safety briefing.".to_owned()),
            expected_feedback: Set("Fictional feedback.".to_owned()),
            status: Set("pending_claim".to_owned()),
            result_summary: Set(None),
            created_by_user_id: Set(commander.id.clone()),
            created_at: Set("2026-07-27T00:00:00Z".to_owned()),
            updated_at: Set("2026-07-27T00:00:00Z".to_owned()),
        }
        .insert(&context.database)
        .await
        .expect("fixture task should be created");
    }

    let request_for = |token: String| {
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/map-view"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request()
    };
    let commander_view: Value = test::read_body_json(
        test::call_service(&app, request_for(context.token(COMMANDER).await)).await,
    )
    .await;
    let commander_items = commander_view["items"].as_array().expect("map items");
    assert!(
        commander_items
            .iter()
            .any(|item| item["object_type"] == "last_seen"
                && item["longitude"].is_null()
                && item["review_status"] == "pending_review"
                && item["display_name"].is_null())
    );
    assert!(
        commander_items
            .iter()
            .any(|item| item["object_type"] == "clue" && item["review_status"] == "confirmed")
    );
    assert!(
        commander_items
            .iter()
            .any(|item| item["id"] == task_id && item["latitude"] == 31.8206)
    );
    assert_eq!(
        commander_items
            .iter()
            .filter(|item| item["object_type"] == "task")
            .count(),
        101
    );
    assert!(
        commander_items
            .iter()
            .any(|item| item["id"] == "map-extra-task-99")
    );

    let family_view: Value = test::read_body_json(
        test::call_service(&app, request_for(context.token(FAMILY).await)).await,
    )
    .await;
    let family_items = family_view["items"].as_array().expect("map items");
    assert!(
        family_items
            .iter()
            .any(|item| item["object_type"] == "last_seen")
    );
    assert!(
        !family_items
            .iter()
            .any(|item| item["object_type"] == "clue")
    );
    assert!(
        !family_items
            .iter()
            .any(|item| item["object_type"] == "task")
    );

    let volunteer_view: Value = test::read_body_json(
        test::call_service(&app, request_for(context.token(VOLUNTEER).await)).await,
    )
    .await;
    let volunteer_items = volunteer_view["items"].as_array().expect("map items");
    assert!(volunteer_items.iter().any(|item| item["id"] == task_id));
    assert!(
        !volunteer_items
            .iter()
            .any(|item| item["object_type"] == "last_seen" || item["object_type"] == "clue")
    );

    let hidden = test::call_service(&app, request_for(context.token(LEARNER).await)).await;
    assert_error(hidden, StatusCode::NOT_FOUND, "not_found").await;
}
