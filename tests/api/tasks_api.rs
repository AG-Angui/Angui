use actix_web::{
    http::{StatusCode, header},
    test,
};
use angui::{
    entities::{
        audit_events, clues, task_location_reports, tasks, user_global_capabilities, users,
    },
    services::task_service,
};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, sea_query::Expr};
use serde_json::{Value, json};

use crate::support::{ADMIN, COMMANDER, FAMILY, LEARNER, TestContext, VOLUNTEER, assert_error};

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

macro_rules! submit_location_report {
    ($app:expr, $task_id:expr, $token:expr, $body:expr) => {{
        test::call_service(
            $app,
            test::TestRequest::post()
                .uri(&format!("/api/tasks/{}/location-reports", $task_id))
                .insert_header((header::AUTHORIZATION, format!("Bearer {}", $token)))
                .insert_header(("Idempotency-Key", uuid::Uuid::new_v4().to_string()))
                .set_json($body)
                .to_request(),
        )
        .await
    }};
}

macro_rules! submit_task_feedback {
    ($app:expr, $task_id:expr, $token:expr, $body:expr) => {{
        test::call_service(
            $app,
            test::TestRequest::post()
                .uri(&format!("/api/tasks/{}/feedback", $task_id))
                .insert_header((header::AUTHORIZATION, format!("Bearer {}", $token)))
                .insert_header(("Idempotency-Key", uuid::Uuid::new_v4().to_string()))
                .set_json($body)
                .to_request(),
        )
        .await
    }};
}

async fn add_second_case_volunteer(context: &TestContext, case_id: &str) -> String {
    let learner = context.authenticated(LEARNER).await;
    users::Entity::update_many()
        .col_expr(users::Column::AccountType, Expr::value("member"))
        .filter(users::Column::Id.eq(&learner.id))
        .exec(&context.database)
        .await
        .expect("fixture learner should become a member account");
    user_global_capabilities::ActiveModel {
        user_id: Set(learner.id),
        capability: Set("volunteer".to_owned()),
        created_at: Set(Utc::now().to_rfc3339()),
    }
    .insert(&context.database)
    .await
    .expect("fixture should grant a second volunteer capability");
    context
        .add_member(case_id, COMMANDER, LEARNER, "volunteer")
        .await;
    context.token(LEARNER).await
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
    let family_token = context.token(FAMILY).await;
    let admin_token = context.token(ADMIN).await;
    let volunteer = context.authenticated(VOLUNTEER).await;
    let second_volunteer_token = add_second_case_volunteer(&context, &case_id).await;
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
    for token in [&family_token, &second_volunteer_token, &admin_token] {
        let unauthorized = update_status!(&app, &task_id, token, "accepted");
        assert_error(unauthorized, StatusCode::NOT_FOUND, "not_found").await;
    }
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

#[actix_web::test]
async fn task_location_reports_accept_only_recent_simulated_points_from_the_active_assignee() {
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
    let family_token = context.token(FAMILY).await;
    let admin_token = context.token(ADMIN).await;
    let volunteer = context.authenticated(VOLUNTEER).await;
    let second_volunteer_token = add_second_case_volunteer(&context, &case_id).await;
    let source_clue_id = confirmed_clue!(&context, &app, &case_id, &commander_token);
    let task_id = create_task!(
        &app,
        &case_id,
        &commander_token,
        &source_clue_id,
        &volunteer.id
    );

    let before_active = submit_location_report!(
        &app,
        &task_id,
        &volunteer_token,
        location_report_json(Utc::now())
    );
    assert_error(before_active, StatusCode::CONFLICT, "conflict").await;

    let accepted = update_status!(&app, &task_id, &volunteer_token, "accepted");
    assert_eq!(accepted.status(), StatusCode::OK);
    let active = update_status!(&app, &task_id, &volunteer_token, "active");
    assert_eq!(active.status(), StatusCode::OK);

    for token in [
        &family_token,
        &commander_token,
        &second_volunteer_token,
        &admin_token,
    ] {
        let non_assignee =
            submit_location_report!(&app, &task_id, token, location_report_json(Utc::now()));
        assert_eq!(non_assignee.status(), StatusCode::NOT_FOUND);
        let body: Value = test::read_body_json(non_assignee).await;
        assert_eq!(body["error"]["code"], "not_found");
        let serialized = body.to_string();
        assert!(!serialized.contains("31.2"));
        assert!(!serialized.contains("121.5"));
        assert!(!serialized.contains("20"));
    }

    let invalid_source = submit_location_report!(
        &app,
        &task_id,
        &volunteer_token,
        json!({
            "source": "device",
            "latitude": 31.2,
            "longitude": 121.5,
            "accuracy_meters": 20,
            "captured_at": Utc::now().to_rfc3339(),
        })
    );
    assert_error(invalid_source, StatusCode::BAD_REQUEST, "validation_error").await;

    let mut invalid_coordinates = location_report_json(Utc::now());
    invalid_coordinates["latitude"] = json!(91);
    let invalid_coordinates =
        submit_location_report!(&app, &task_id, &volunteer_token, invalid_coordinates);
    assert_error(
        invalid_coordinates,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;

    let mut invalid_accuracy = location_report_json(Utc::now());
    invalid_accuracy["accuracy_meters"] = json!(10_001);
    let invalid_accuracy =
        submit_location_report!(&app, &task_id, &volunteer_token, invalid_accuracy);
    assert_error(
        invalid_accuracy,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;

    for captured_at in [
        Utc::now() - Duration::minutes(16),
        Utc::now() + Duration::minutes(6),
    ] {
        let invalid_time = submit_location_report!(
            &app,
            &task_id,
            &volunteer_token,
            location_report_json(captured_at)
        );
        assert_error(invalid_time, StatusCode::BAD_REQUEST, "validation_error").await;
    }

    let unknown_device_field = submit_location_report!(
        &app,
        &task_id,
        &volunteer_token,
        json!({
            "source": "simulated",
            "latitude": 31.2,
            "longitude": 121.5,
            "accuracy_meters": 20,
            "captured_at": Utc::now().to_rfc3339(),
            "device_id": "forbidden-device-identifier",
        })
    );
    assert_error(
        unknown_device_field,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;

    let report = submit_location_report!(
        &app,
        &task_id,
        &volunteer_token,
        location_report_json(Utc::now())
    );
    assert_eq!(report.status(), StatusCode::CREATED);
    let report: Value = test::read_body_json(report).await;
    assert_eq!(report["source"], "simulated");
    assert!(report.get("latitude").is_none());
    assert!(report.get("longitude").is_none());
    assert!(report.get("accuracy_meters").is_none());
    let captured_at = report["captured_at"]
        .as_str()
        .expect("captured_at should be returned");
    let retention_expires_at = report["retention_expires_at"]
        .as_str()
        .expect("retention_expires_at should be returned");
    let captured_at = chrono::DateTime::parse_from_rfc3339(captured_at).expect("rfc3339");
    let retention_expires_at =
        chrono::DateTime::parse_from_rfc3339(retention_expires_at).expect("rfc3339");
    assert_eq!(retention_expires_at - captured_at, Duration::hours(24));
    let report_id = report["id"].as_str().expect("report id should be returned");

    let audit = audit_events::Entity::find()
        .filter(audit_events::Column::EntityId.eq(report_id))
        .filter(audit_events::Column::Action.eq("task.location_reported"))
        .one(&context.database)
        .await
        .expect("location report audit should be readable")
        .expect("location report should be audited");
    let metadata: Value = serde_json::from_str(
        audit
            .metadata_json
            .as_deref()
            .expect("location report audit should have metadata"),
    )
    .expect("location report audit metadata should be JSON");
    assert_eq!(metadata["source"], "simulated");
    assert!(metadata.get("latitude").is_none());
    assert!(metadata.get("longitude").is_none());
    assert!(metadata.get("accuracy_meters").is_none());

    let reused_key = "e6d449bb-5b77-4378-a7a1-54de941f1bb8";
    let idempotent_payload = location_report_json(Utc::now());
    let first_idempotent_report = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/tasks/{task_id}/location-reports"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .insert_header(("Idempotency-Key", reused_key))
            .set_json(idempotent_payload.clone())
            .to_request(),
    )
    .await;
    assert_eq!(first_idempotent_report.status(), StatusCode::CREATED);
    let replayed_idempotent_report = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/tasks/{task_id}/location-reports"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .insert_header(("Idempotency-Key", reused_key))
            .set_json(idempotent_payload)
            .to_request(),
    )
    .await;
    assert_eq!(replayed_idempotent_report.status(), StatusCode::CREATED);
    let replayed: Value = test::read_body_json(replayed_idempotent_report).await;
    let first: Value = test::read_body_json(first_idempotent_report).await;
    assert_eq!(replayed, first);
    let conflicting_idempotent_report = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/tasks/{task_id}/location-reports"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .insert_header(("Idempotency-Key", reused_key))
            .set_json(json!({
                "source": "simulated",
                "latitude": 31.3,
                "longitude": 121.5,
                "accuracy_meters": 20,
                "captured_at": Utc::now().to_rfc3339(),
            }))
            .to_request(),
    )
    .await;
    assert_error(
        conflicting_idempotent_report,
        StatusCode::CONFLICT,
        "conflict",
    )
    .await;

    task_location_reports::Entity::update_many()
        .col_expr(
            task_location_reports::Column::RetentionExpiresAt,
            Expr::value((Utc::now() - Duration::seconds(1)).to_rfc3339()),
        )
        .filter(task_location_reports::Column::Id.eq(report_id))
        .exec(&context.database)
        .await
        .expect("location report expiration should be configurable for the retention test");
    assert_eq!(
        task_service::purge_expired_location_reports(&context.database)
            .await
            .expect("expired location reports should be purged"),
        1
    );
    assert!(
        task_location_reports::Entity::find_by_id(report_id)
            .one(&context.database)
            .await
            .expect("location report lookup should succeed")
            .is_none()
    );

    let logout = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/logout")
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    let logged_out_report = submit_location_report!(
        &app,
        &task_id,
        &volunteer_token,
        location_report_json(Utc::now())
    );
    assert_error(logged_out_report, StatusCode::UNAUTHORIZED, "unauthorized").await;

    let active_volunteer_token = context.token(VOLUNTEER).await;

    let completed = update_status!(&app, &task_id, &active_volunteer_token, "completed");
    assert_eq!(completed.status(), StatusCode::OK);
    let completed_report = submit_location_report!(
        &app,
        &task_id,
        &active_volunteer_token,
        location_report_json(Utc::now())
    );
    assert_error(completed_report, StatusCode::CONFLICT, "conflict").await;

    let cancelled_task_id = create_task!(
        &app,
        &case_id,
        &commander_token,
        &source_clue_id,
        &volunteer.id
    );
    let cancelled = update_status!(&app, &cancelled_task_id, &commander_token, "cancelled");
    assert_eq!(cancelled.status(), StatusCode::OK);
    let cancelled_report = submit_location_report!(
        &app,
        &cancelled_task_id,
        &active_volunteer_token,
        location_report_json(Utc::now())
    );
    assert_error(cancelled_report, StatusCode::CONFLICT, "conflict").await;
}

#[actix_web::test]
async fn task_feedback_is_an_assignee_only_pending_review_clue_without_task_side_effects() {
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
    let family_token = context.token(FAMILY).await;
    let admin_token = context.token(ADMIN).await;
    let volunteer = context.authenticated(VOLUNTEER).await;
    let second_volunteer_token = add_second_case_volunteer(&context, &case_id).await;
    let source_clue_id = confirmed_clue!(&context, &app, &case_id, &commander_token);
    let task_id = create_task!(
        &app,
        &case_id,
        &commander_token,
        &source_clue_id,
        &volunteer.id
    );

    let before_active = submit_task_feedback!(&app, &task_id, &volunteer_token, feedback_json());
    assert_error(before_active, StatusCode::CONFLICT, "conflict").await;
    assert_eq!(
        update_status!(&app, &task_id, &volunteer_token, "accepted").status(),
        StatusCode::OK
    );
    assert_eq!(
        update_status!(&app, &task_id, &volunteer_token, "active").status(),
        StatusCode::OK
    );

    for token in [
        &family_token,
        &commander_token,
        &second_volunteer_token,
        &admin_token,
    ] {
        let forbidden = submit_task_feedback!(&app, &task_id, token, feedback_json());
        assert_error(forbidden, StatusCode::NOT_FOUND, "not_found").await;
    }

    let invalid_location = submit_task_feedback!(
        &app,
        &task_id,
        &volunteer_token,
        json!({
            "content": "Observed a safe route.",
            "location_precision": "approximate"
        })
    );
    assert_error(
        invalid_location,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;
    let invalid_attachment = submit_task_feedback!(
        &app,
        &task_id,
        &volunteer_token,
        json!({ "content": "Observed a safe route.", "attachment_ids": ["missing"] })
    );
    assert_error(
        invalid_attachment,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;

    let feedback = submit_task_feedback!(&app, &task_id, &volunteer_token, feedback_json());
    assert_eq!(feedback.status(), StatusCode::CREATED);
    let feedback: Value = test::read_body_json(feedback).await;
    assert_eq!(feedback["task_id"], task_id);
    assert_eq!(feedback["status"], "pending_review");
    let feedback_clue_id = feedback["clue_id"]
        .as_str()
        .expect("feedback receipt should identify its clue");

    let feedback_clue = clues::Entity::find_by_id(feedback_clue_id)
        .one(&context.database)
        .await
        .expect("feedback clue lookup should succeed")
        .expect("feedback should create a clue");
    assert_eq!(feedback_clue.status, "pending_review");
    assert_eq!(feedback_clue.source, "task_feedback");
    assert_eq!(feedback_clue.source_type, "field_report");
    assert_eq!(
        feedback_clue.content,
        "Observed a safe route and no immediate hazard."
    );
    assert_eq!(
        feedback_clue.location_text.as_deref(),
        Some("North gate walkway")
    );
    assert_eq!(
        feedback_clue.location_precision.as_deref(),
        Some("approximate")
    );
    assert_eq!(
        feedback_clue.linked_task_reference.as_deref(),
        Some(task_id.as_str())
    );

    let task = tasks::Entity::find_by_id(&task_id)
        .one(&context.database)
        .await
        .expect("task lookup should succeed")
        .expect("task should exist");
    assert_eq!(task.status, "active");
    assert_eq!(task.result_summary, None);

    let audit = audit_events::Entity::find()
        .filter(audit_events::Column::EntityId.eq(&task_id))
        .filter(audit_events::Column::Action.eq("task.feedback_submitted"))
        .one(&context.database)
        .await
        .expect("feedback audit lookup should succeed")
        .expect("feedback should be audited");
    let metadata = audit
        .metadata_json
        .expect("feedback audit should include metadata");
    assert!(metadata.contains(feedback_clue_id));
    assert!(!metadata.contains("Observed a safe route"));
    assert!(!metadata.contains("North gate walkway"));

    let reviewed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{feedback_clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(
                json!({ "status": "confirmed", "reason": "commander verified field feedback" }),
            )
            .to_request(),
    )
    .await;
    assert_eq!(reviewed.status(), StatusCode::OK);

    assert_eq!(
        update_status!(&app, &task_id, &volunteer_token, "completed").status(),
        StatusCode::OK
    );
    let completed_feedback =
        submit_task_feedback!(&app, &task_id, &volunteer_token, feedback_json());
    assert_error(completed_feedback, StatusCode::CONFLICT, "conflict").await;

    let closed_case_id = context.create_case().await;
    context
        .add_member(&closed_case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&closed_case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let closed_case_source = confirmed_clue!(&context, &app, &closed_case_id, &commander_token);
    let closed_case_task_id = create_task!(
        &app,
        &closed_case_id,
        &commander_token,
        &closed_case_source,
        &volunteer.id
    );
    assert_eq!(
        update_status!(&app, &closed_case_task_id, &volunteer_token, "accepted").status(),
        StatusCode::OK
    );
    assert_eq!(
        update_status!(&app, &closed_case_task_id, &volunteer_token, "active").status(),
        StatusCode::OK
    );
    context.close_case(&closed_case_id).await;
    let closed_case_feedback = submit_task_feedback!(
        &app,
        &closed_case_task_id,
        &volunteer_token,
        feedback_json()
    );
    assert_error(closed_case_feedback, StatusCode::CONFLICT, "conflict").await;
}

#[actix_web::test]
async fn task_safety_and_navigation_are_limited_to_the_assignee_and_commander() {
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
    let family_token = context.token(FAMILY).await;
    let admin_token = context.token(ADMIN).await;
    let volunteer = context.authenticated(VOLUNTEER).await;
    let second_volunteer_token = add_second_case_volunteer(&context, &case_id).await;
    let source_clue_id = confirmed_clue!(&context, &app, &case_id, &commander_token);
    let task_id = create_task!(
        &app,
        &case_id,
        &commander_token,
        &source_clue_id,
        &volunteer.id
    );

    for token in [&commander_token, &volunteer_token] {
        let safety = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/tasks/{task_id}/safety-briefing"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .to_request(),
        )
        .await;
        assert_eq!(safety.status(), StatusCode::OK);
        let safety: Value = test::read_body_json(safety).await;
        assert_eq!(safety["task_id"], task_id);
        assert_eq!(safety["risk_level"], "medium");
        assert_eq!(safety["source"], "rule_based");
        assert_eq!(safety["degradation_status"], "rule_based_fallback");
        assert!(safety["emergency_stop_message"].as_str().is_some());
        assert!(
            safety["notices"]
                .as_array()
                .is_some_and(|notices| !notices.is_empty())
        );

        let navigation = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/tasks/{task_id}/navigation"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .to_request(),
        )
        .await;
        assert_eq!(navigation.status(), StatusCode::OK);
        let navigation: Value = test::read_body_json(navigation).await;
        assert_eq!(navigation["task_id"], task_id);
        assert_eq!(navigation["source"], "task_area_text");
        assert_eq!(navigation["degradation_status"], "text_fallback");
        assert!(navigation["navigation_url"].is_null());
        assert!(
            navigation["route_summary"]
                .as_str()
                .is_some_and(|summary| summary.contains("North gate to market"))
        );
    }

    for token in [&family_token, &second_volunteer_token, &admin_token] {
        for endpoint in ["safety-briefing", "navigation"] {
            assert_error(
                test::call_service(
                    &app,
                    test::TestRequest::get()
                        .uri(&format!("/api/tasks/{task_id}/{endpoint}"))
                        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                        .to_request(),
                )
                .await,
                StatusCode::NOT_FOUND,
                "not_found",
            )
            .await;
        }
    }

    let mut no_coordinate_payload = task_json(&source_clue_id, &volunteer.id);
    no_coordinate_payload["latitude"] = Value::Null;
    no_coordinate_payload["longitude"] = Value::Null;
    let no_coordinate_task = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(no_coordinate_payload)
            .to_request(),
    )
    .await;
    assert_eq!(no_coordinate_task.status(), StatusCode::CREATED);
    let no_coordinate_task: Value = test::read_body_json(no_coordinate_task).await;
    let no_coordinate_task_id = no_coordinate_task["id"]
        .as_str()
        .expect("task id should be returned");
    let navigation = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/tasks/{no_coordinate_task_id}/navigation"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(navigation.status(), StatusCode::OK);
    let navigation: Value = test::read_body_json(navigation).await;
    assert!(navigation["navigation_url"].is_null());
    assert!(
        navigation["route_summary"]
            .as_str()
            .is_some_and(|summary| summary.contains("未配置坐标"))
    );
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

fn location_report_json(captured_at: chrono::DateTime<Utc>) -> Value {
    json!({
        "source": "simulated",
        "latitude": 31.2,
        "longitude": 121.5,
        "accuracy_meters": 20,
        "captured_at": captured_at.to_rfc3339(),
    })
}

fn feedback_json() -> Value {
    json!({
        "content": "Observed a safe route and no immediate hazard.",
        "occurred_at": "2026-07-27T09:00:00Z",
        "location_text": "North gate walkway",
        "location_precision": "approximate"
    })
}
