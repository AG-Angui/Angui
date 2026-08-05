use actix_web::{
    http::{StatusCode, header},
    test,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};

use crate::support::{
    ADMIN, COMMANDER, FAMILY, LEARNER, TestContext, VOLUNTEER, assert_error,
    read_sse_completed_json,
};
use angui::entities::{clue_drafts, summary_drafts};

#[actix_web::test]
async fn case_collaboration_endpoints_apply_roles_lifecycle_and_degraded_fallbacks() {
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
    let own_pending_id = context.create_clue(&case_id, FAMILY).await;
    let internal_pending_id = context.create_clue(&case_id, COMMANDER).await;
    let reviewed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{confirmed_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "status": "confirmed", "reason": "fictional source verified" }))
            .to_request(),
    )
    .await;
    assert_eq!(reviewed.status(), StatusCode::OK);

    let public_progress = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/public-progress"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(FAMILY).await),
            ))
            .to_request(),
    )
    .await;
    assert_eq!(public_progress.status(), StatusCode::OK);
    let public_progress: Value = test::read_body_json(public_progress).await;
    assert_eq!(public_progress["publication_status"], "reviewed_public");
    assert_eq!(
        public_progress["confirmed_progress"][0]["clue_id"],
        confirmed_id
    );
    assert_eq!(
        public_progress["confirmed_progress"][0]["progress_type"],
        "confirmed_update"
    );
    assert!(
        public_progress["confirmed_progress"][0]
            .get("content")
            .is_none()
    );
    assert_eq!(
        public_progress["requested_family_information"][0]["clue_id"],
        own_pending_id
    );
    assert_ne!(
        public_progress["requested_family_information"][0]["clue_id"],
        internal_pending_id
    );
    assert_eq!(
        public_progress["requested_family_information"][0]["progress_type"],
        "family_follow_up"
    );
    assert!(
        public_progress["requested_family_information"][0]
            .get("content")
            .is_none()
    );
    assert!(!public_progress.to_string().contains("测试线索"));
    assert_error(
        test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/cases/{case_id}/public-progress"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
                .to_request(),
        )
        .await,
        StatusCode::FORBIDDEN,
        "forbidden",
    )
    .await;

    for email in [VOLUNTEER, LEARNER, ADMIN] {
        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/cases/{case_id}/public-progress"))
                .insert_header((
                    header::AUTHORIZATION,
                    format!("Bearer {}", context.token(email).await),
                ))
                .to_request(),
        )
        .await;
        assert!(matches!(
            response.status(),
            StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
        ));
    }
    assert_error(
        test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/cases/{case_id}/public-progress"))
                .to_request(),
        )
        .await,
        StatusCode::UNAUTHORIZED,
        "unauthorized",
    )
    .await;

    let source = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/source-records"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "record_type": "phone_record", "content": "Fictional caller said to verify the north gate.", "source_reference": "fictional-call-1" }))
            .to_request(),
    ).await;
    assert_eq!(source.status(), StatusCode::CREATED);
    let source: Value = test::read_body_json(source).await;
    let clue_drafts = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clue-drafts"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "source_record_id": source["id"] }))
            .to_request(),
    )
    .await;
    assert_eq!(clue_drafts.status(), StatusCode::CREATED);
    let clue_drafts = read_sse_completed_json(clue_drafts).await;
    assert_eq!(clue_drafts[0]["status"], "draft");
    assert_eq!(clue_drafts[0]["raw_record_reference"], "fictional-call-1");
    assert_eq!(clue_drafts[0]["degradation_status"], "rule_based_fallback");
    assert!(clue_drafts[0]["uncertainty_notice"].as_str().is_some());
    let clue_draft_id = clue_drafts[0]["id"].as_str().expect("draft id");
    assert!(
        clue_drafts::Entity::find()
            .filter(clue_drafts::Column::Id.eq(clue_draft_id))
            .one(&context.database)
            .await
            .expect("clue draft query should succeed")
            .is_some()
    );

    let task = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/tasks"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({
                "source_clue_id": confirmed_id,
                "volunteer_user_id": volunteer.id,
                "title": "Verify fictional north gate",
                "objective": "Check the fictional route.",
                "area_text": "Fictional north gate",
                "latitude": 31.8206,
                "longitude": 117.2272,
                "due_at": "2099-07-27T12:00:00Z",
                "background": "Internal fictional direction.",
                "risk_level": "medium",
                "risk_notes": "Stay in public areas.",
                "safety_briefing": "Keep contact with the commander.",
                "expected_feedback": "Submit factual observations."
            }))
            .to_request(),
    )
    .await;
    assert_eq!(task.status(), StatusCode::CREATED);

    let public_progress_after_task = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/public-progress"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(FAMILY).await),
            ))
            .to_request(),
    )
    .await;
    assert_eq!(public_progress_after_task.status(), StatusCode::OK);
    let public_progress_after_task: Value = test::read_body_json(public_progress_after_task).await;
    assert!(public_progress_after_task.get("task_status").is_none());

    let pois = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/pois?category=hospital"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(VOLUNTEER).await),
            ))
            .to_request(),
    )
    .await;
    assert_eq!(pois.status(), StatusCode::OK);
    let pois: Value = test::read_body_json(pois).await;
    assert_eq!(pois["source"], "fixed_demo_fallback");
    assert_eq!(pois["degradation_status"], "degraded");
    assert!(pois["items"][0]["longitude"].is_null());
    let transit = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/pois?category=transit"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(transit.status(), StatusCode::OK);
    let transit: Value = test::read_body_json(transit).await;
    assert_eq!(transit["items"][0]["category"], "transit");
    assert_eq!(transit["degradation_status"], "degraded");
    assert_error(
        test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/cases/{case_id}/pois?category=unknown"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
                .to_request(),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;

    let draft = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/summary-drafts"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({}))
            .to_request(),
    )
    .await;
    assert_eq!(draft.status(), StatusCode::CREATED);
    let draft = read_sse_completed_json(draft).await;
    assert_eq!(draft["status"], "pending_review");
    assert_eq!(draft["publication_eligible"], true);
    assert!(draft["provider_model"].is_null());
    let draft_id = draft["id"].as_str().expect("draft id");
    assert_error(
        test::call_service(
            &app,
            test::TestRequest::patch()
                .uri(&format!(
                    "/api/cases/{case_id}/summary-drafts/{draft_id}/review"
                ))
                .insert_header((
                    header::AUTHORIZATION,
                    format!("Bearer {}", context.token(VOLUNTEER).await),
                ))
                .set_json(json!({ "action": "publish", "reason": "fictional approval" }))
                .to_request(),
        )
        .await,
        StatusCode::FORBIDDEN,
        "forbidden",
    )
    .await;
    let published = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!(
                "/api/cases/{case_id}/summary-drafts/{draft_id}/review"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "action": "publish", "reason": "fictional human approval" }))
            .to_request(),
    )
    .await;
    assert_eq!(published.status(), StatusCode::OK);
    let published: Value = test::read_body_json(published).await;
    assert_eq!(published["status"], "published");
    assert!(published["reviewed_at"].as_str().is_some());

    let second_draft = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/summary-drafts"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({}))
            .to_request(),
    )
    .await;
    assert_eq!(second_draft.status(), StatusCode::CREATED);
    let second_draft = read_sse_completed_json(second_draft).await;
    let second_draft_id = second_draft["id"].as_str().expect("second draft id");
    let superseding_publish = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!(
                "/api/cases/{case_id}/summary-drafts/{second_draft_id}/review"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "action": "publish", "reason": "fictional revised approval" }))
            .to_request(),
    )
    .await;
    assert_eq!(superseding_publish.status(), StatusCode::OK);
    let first = summary_drafts::Entity::find_by_id(draft_id)
        .one(&context.database)
        .await
        .expect("first summary draft query should succeed")
        .expect("first summary draft should exist");
    assert_eq!(first.status, "superseded");

    let manual_draft = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/summary-drafts"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "content": "Fictional internal direction that must not be public." }))
            .to_request(),
    )
    .await;
    assert_eq!(manual_draft.status(), StatusCode::CREATED);
    let manual_draft: Value = test::read_body_json(manual_draft).await;
    assert_eq!(manual_draft["status"], "draft");
    assert_eq!(manual_draft["publication_eligible"], false);
    let manual_draft_id = manual_draft["id"].as_str().expect("manual draft id");
    let submitted_manual = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!(
                "/api/cases/{case_id}/summary-drafts/{manual_draft_id}/review"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "action": "submit", "reason": "fictional review submission" }))
            .to_request(),
    )
    .await;
    assert_eq!(submitted_manual.status(), StatusCode::OK);
    assert_error(
        test::call_service(
            &app,
            test::TestRequest::patch()
                .uri(&format!(
                    "/api/cases/{case_id}/summary-drafts/{manual_draft_id}/review"
                ))
                .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
                .set_json(json!({ "action": "publish", "reason": "fictional unsafe publication" }))
                .to_request(),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;

    let hidden = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/summary-drafts"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .set_json(json!({}))
            .to_request(),
    )
    .await;
    assert_error(hidden, StatusCode::NOT_FOUND, "not_found").await;
}

#[actix_web::test]
async fn clue_draft_queue_is_commander_only_and_survives_the_creation_response() {
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

    let source = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/source-records"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "record_type": "phone_record", "content": "Fictional caller asks the team to verify the north gate.", "source_reference": "fictional-call-record-2" }))
            .to_request(),
    ).await;
    assert_eq!(source.status(), StatusCode::CREATED);
    let source: Value = test::read_body_json(source).await;
    let created = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clue-drafts"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "source_record_id": source["id"] }))
            .to_request(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);

    let listed = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/clue-drafts"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    let listed: Value = test::read_body_json(listed).await;
    assert_eq!(listed.as_array().map(Vec::len), Some(1));
    assert_eq!(listed[0]["raw_record_reference"], "fictional-call-record-2");
    let draft_id = listed[0]["id"].as_str().expect("draft id");

    let reviewed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/clue-drafts/{draft_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({
                "action": "accept",
                "reason": "fictional commander review",
                "candidate": {
                    "content_summary": "Verify the fictional north gate.",
                    "occurred_at": null,
                    "location_text": "Fictional north gate",
                    "source_text": "Fictional caller",
                    "action_candidates": ["Verify with the commander"],
                    "missing_fields": ["occurred_at"],
                    "source_excerpt": "Fictional caller asks the team to verify the north gate.",
                    "field_sources": {}
                },
                "field_decisions": {
                    "location_text": { "action": "edit", "value": "Fictional north gate", "reason": "normalized by commander" },
                    "occurred_at": { "action": "clear", "reason": "no time in source" }
                }
            }))
            .to_request(),
    )
    .await;
    assert_eq!(reviewed.status(), StatusCode::OK);
    let reviewed: Value = test::read_body_json(reviewed).await;
    assert_eq!(reviewed["review_status"], "accepted");
    assert!(reviewed["promoted_clue_id"].as_str().is_some());

    let forbidden = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/clue-drafts"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(VOLUNTEER).await),
            ))
            .to_request(),
    )
    .await;
    assert_error(forbidden, StatusCode::FORBIDDEN, "forbidden").await;
}
