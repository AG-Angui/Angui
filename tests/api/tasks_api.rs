use actix_web::{
    http::{StatusCode, header},
    test,
};
use angui::entities::audit_events;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};

use crate::support::{ADMIN, COMMANDER, FAMILY, TestContext, VOLUNTEER, assert_error};

macro_rules! confirmed_clue {
    ($context:expr, $app:expr, $case_id:expr, $commander_token:expr) => {{
        let clue_id = $context.create_clue($case_id, FAMILY).await;
        let reviewed = test::call_service(
            $app,
            test::TestRequest::patch()
                .uri(&format!("/api/clues/{clue_id}/review"))
                .insert_header((
                    header::AUTHORIZATION,
                    format!("Bearer {}", $commander_token),
                ))
                .set_json(json!({ "status": "confirmed", "reason": "source verified" }))
                .to_request(),
        )
        .await;
        assert_eq!(reviewed.status(), StatusCode::OK);
        clue_id
    }};
}

macro_rules! create_task {
    ($app:expr, $case_id:expr, $commander_token:expr, $source_clue_id:expr, $volunteer_user_id:expr) => {{
        let response = test::call_service(
            $app,
            test::TestRequest::post()
                .uri(&format!("/api/cases/{}/tasks", $case_id))
                .insert_header((
                    header::AUTHORIZATION,
                    format!("Bearer {}", $commander_token),
                ))
                .set_json(task_json($source_clue_id, $volunteer_user_id))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        test::read_body_json::<Value, _>(response).await["id"]
            .as_str()
            .expect("task id should be returned")
            .to_owned()
    }};
}

macro_rules! update_status {
    ($app:expr, $task_id:expr, $token:expr, $status:expr) => {{
        test::call_service(
            $app,
            test::TestRequest::patch()
                .uri(&format!("/api/tasks/{}/status", $task_id))
                .insert_header((header::AUTHORIZATION, format!("Bearer {}", $token)))
                .set_json(json!({ "status": $status }))
                .to_request(),
        )
        .await
    }};
}

#[actix_web::test]
async fn post_case_tasks_requires_a_commander_confirmed_source_and_active_case_volunteer() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let commander_token = context.token(COMMANDER).await;
    let family_token = context.token(FAMILY).await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let admin_token = context.token(ADMIN).await;
    let volunteer = context.authenticated(VOLUNTEER).await;
    let source_clue_id = confirmed_clue!(&context, &app, &case_id, &commander_token);

    for (token, expected_status, expected_code) in [
        (&family_token, StatusCode::FORBIDDEN, "forbidden"),
        (&volunteer_token, StatusCode::FORBIDDEN, "forbidden"),
        (&admin_token, StatusCode::NOT_FOUND, "not_found"),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/cases/{case_id}/tasks"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .set_json(task_json(&source_clue_id, &volunteer.id))
                .to_request(),
        )
        .await;
        assert_error(response, expected_status, expected_code).await;
    }

    let mut invalid_coordinates = task_json(&source_clue_id, &volunteer.id);
    invalid_coordinates["latitude"] = json!(91);
    let invalid_coordinates = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(invalid_coordinates)
            .to_request(),
    )
    .await;
    assert_error(
        invalid_coordinates,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;

    let created = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(task_json(&source_clue_id, &volunteer.id))
            .to_request(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: Value = test::read_body_json(created).await;
    assert_eq!(created["status"], "assigned");
    assert_eq!(created["source_clue_id"], source_clue_id);
    assert_eq!(created["assigned_volunteer_user_id"], volunteer.id);
    let task_id = created["id"].as_str().expect("task id should be returned");

    let audits = audit_events::Entity::find()
        .filter(audit_events::Column::EntityId.eq(task_id))
        .all(&context.database)
        .await
        .expect("task audits should be readable");
    assert_eq!(audits.len(), 2);
    assert!(audits.iter().any(|audit| audit.action == "task.created"));
    assert!(audits.iter().any(|audit| audit.action == "task.assigned"));

    let unconfirmed_clue_id = context.create_clue(&case_id, FAMILY).await;
    let unconfirmed = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(task_json(&unconfirmed_clue_id, &volunteer.id))
            .to_request(),
    )
    .await;
    assert_error(unconfirmed, StatusCode::BAD_REQUEST, "validation_error").await;

    let no_volunteer = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(task_json(
                &source_clue_id,
                &context.authenticated(COMMANDER).await.id,
            ))
            .to_request(),
    )
    .await;
    assert_error(no_volunteer, StatusCode::BAD_REQUEST, "validation_error").await;

    context.close_case(&case_id).await;
    let closed_case = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(task_json(&source_clue_id, &volunteer.id))
            .to_request(),
    )
    .await;
    assert_error(closed_case, StatusCode::CONFLICT, "conflict").await;
}

#[actix_web::test]
async fn task_lists_are_server_filtered_by_case_role_and_personal_queue_requires_volunteer() {
    let context = TestContext::new().await;
    let empty_app = crate::init_api_app!(&context);
    let empty_volunteer_token = context.token(VOLUNTEER).await;
    let empty_queue = test::call_service(
        &empty_app,
        test::TestRequest::get()
            .uri("/api/tasks/mine")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {empty_volunteer_token}"),
            ))
            .to_request(),
    )
    .await;
    assert_eq!(empty_queue.status(), StatusCode::OK);
    assert_eq!(
        test::read_body_json::<Value, _>(empty_queue).await,
        json!([])
    );

    let app = crate::init_api_app!(&context);
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let commander_token = context.token(COMMANDER).await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let family_token = context.token(FAMILY).await;
    let admin_token = context.token(ADMIN).await;
    let volunteer = context.authenticated(VOLUNTEER).await;
    let source_clue_id = confirmed_clue!(&context, &app, &case_id, &commander_token);
    let created = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(task_json(&source_clue_id, &volunteer.id))
            .to_request(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);

    let commander_tasks = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/tasks?page=1&page_size=1"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(commander_tasks.status(), StatusCode::OK);
    let commander_tasks: Value = test::read_body_json(commander_tasks).await;
    assert_eq!(commander_tasks["total"], 1);
    assert_eq!(
        commander_tasks["items"][0]["assigned_volunteer_user_id"],
        volunteer.id
    );

    let volunteer_tasks = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(volunteer_tasks.status(), StatusCode::OK);
    let volunteer_tasks: Value = test::read_body_json(volunteer_tasks).await;
    assert_eq!(volunteer_tasks["total"], 1);
    assert_eq!(
        volunteer_tasks["items"][0]["assigned_volunteer_user_id"],
        Value::Null
    );
    assert_eq!(volunteer_tasks["items"][0]["source_clue_id"], Value::Null);
    assert_eq!(volunteer_tasks["items"][0]["background"], Value::Null);

    let family_tasks = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(family_tasks.status(), StatusCode::OK);
    let family_tasks: Value = test::read_body_json(family_tasks).await;
    assert_eq!(family_tasks["items"], json!([]));

    let personal_queue = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/tasks/mine")
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(personal_queue.status(), StatusCode::OK);
    let personal_queue: Value = test::read_body_json(personal_queue).await;
    assert_eq!(personal_queue.as_array().map(Vec::len), Some(1));
    assert_eq!(personal_queue[0]["assigned_volunteer_user_id"], Value::Null);
    assert_eq!(personal_queue[0]["source_clue_id"], Value::Null);
    assert_eq!(personal_queue[0]["background"], Value::Null);

    for token in [&family_token, &commander_token, &admin_token] {
        let queue = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/tasks/mine")
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .to_request(),
        )
        .await;
        assert_error(queue, StatusCode::FORBIDDEN, "forbidden").await;
    }

    let unrelated = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_error(unrelated, StatusCode::NOT_FOUND, "not_found").await;
}

#[actix_web::test]
async fn task_status_state_machine_is_limited_to_the_assignee_or_commander_cancellation_and_audited()
 {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let commander_token = context.token(COMMANDER).await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let volunteer = context.authenticated(VOLUNTEER).await;
    let source_clue_id = confirmed_clue!(&context, &app, &case_id, &commander_token);
    let task_id = create_task!(
        &app,
        &case_id,
        &commander_token,
        &source_clue_id,
        &volunteer.id
    );

    let illegal = update_status!(&app, &task_id, &volunteer_token, "completed");
    assert_eq!(illegal.status(), StatusCode::CONFLICT);
    let illegal: Value = test::read_body_json(illegal).await;
    assert_eq!(illegal["error"]["code"], "conflict");
    assert_eq!(
        illegal["error"]["message"],
        "task status cannot change from assigned to completed"
    );
    for status in ["accepted", "active", "blocked", "active", "completed"] {
        let response = update_status!(&app, &task_id, &volunteer_token, status);
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value = test::read_body_json(response).await;
        assert_eq!(body["status"], status);
    }
    let completed_cancel = update_status!(&app, &task_id, &commander_token, "cancelled");
    assert_error(completed_cancel, StatusCode::CONFLICT, "conflict").await;

    let cancellable_task_id = create_task!(
        &app,
        &case_id,
        &commander_token,
        &source_clue_id,
        &volunteer.id
    );
    let cancelled = update_status!(&app, &cancellable_task_id, &commander_token, "cancelled");
    assert_eq!(cancelled.status(), StatusCode::OK);
    let cancelled: Value = test::read_body_json(cancelled).await;
    assert_eq!(cancelled["status"], "cancelled");

    let audits = audit_events::Entity::find()
        .filter(audit_events::Column::EntityId.eq(&task_id))
        .filter(audit_events::Column::Action.eq("task.status_changed"))
        .all(&context.database)
        .await
        .expect("state transition audits should be readable");
    assert_eq!(audits.len(), 5);
    assert!(audits.iter().all(|audit| {
        audit.metadata_json.as_deref().is_some_and(|metadata| {
            let metadata: Value = serde_json::from_str(metadata).expect("audit metadata is JSON");
            metadata["from"].is_string() && metadata["to"].is_string()
        })
    }));
}

fn task_json(source_clue_id: &str, volunteer_user_id: &str) -> Value {
    json!({
        "source_clue_id": source_clue_id,
        "volunteer_user_id": volunteer_user_id,
        "title": "Verify north gate",
        "objective": "Check the reported route and submit observations.",
        "area_text": "North gate to market",
        "latitude": 31.2,
        "longitude": 121.5,
        "due_at": "2099-07-27T12:00:00Z",
        "background": "A commander-reviewed report needs field verification.",
        "risk_level": "medium",
        "risk_notes": "Stay in public areas and do not enter restricted property.",
        "safety_briefing": "Keep contact with the commander and stop if conditions change.",
        "expected_feedback": "Submit a factual text report for commander review."
    })
}
