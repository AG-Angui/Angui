use actix_web::{
    http::{StatusCode, header},
    test,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;

use crate::support::{COMMANDER, FAMILY, TestContext, VOLUNTEER, assert_error};

#[actix_web::test]
async fn patch_clue_review_requires_the_case_commander_and_publishes_review_status() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let clue_id = context.create_clue(&case_id, FAMILY).await;
    let family_token = context.token(FAMILY).await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);
    let forbidden = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(
                serde_json::json!({ "status": "confirmed", "reason": "family cannot review" }),
            )
            .to_request(),
    )
    .await;
    assert_error(forbidden, StatusCode::FORBIDDEN, "forbidden").await;
    let confirmed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "status": "confirmed", "reason": "verified against the original report" }))
            .to_request(),
    )
    .await;
    assert_eq!(confirmed.status(), StatusCode::OK);
    let body: Value = test::read_body_json(confirmed).await;
    assert_eq!(body["status"], "confirmed");
    assert!(body["reviewed_at"].is_string());
    assert!(body["confirmed_at"].is_string());

    let audit = angui::entities::audit_events::Entity::find()
        .filter(angui::entities::audit_events::Column::EntityId.eq(&clue_id))
        .filter(angui::entities::audit_events::Column::Action.eq("clue.reviewed"))
        .one(&context.database)
        .await
        .expect("audit lookup should succeed")
        .expect("clue review should be audited");
    let metadata = audit
        .metadata_json
        .expect("audit metadata should be present");
    assert!(metadata.contains("confirmed"));
    assert!(!metadata.contains("测试线索"));
}

#[actix_web::test]
async fn patch_clue_review_rejects_pending_review_as_a_review_target() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let clue_id = context.create_clue(&case_id, FAMILY).await;
    let token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(serde_json::json!({ "status": "pending_review", "reason": "invalid target status" }))
            .to_request(),
    )
    .await;
    assert_error(response, StatusCode::BAD_REQUEST, "validation_error").await;
}

#[actix_web::test]
async fn patch_clue_review_rejects_volunteers_and_records_each_review_transition() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let clue_id = context.create_clue(&case_id, FAMILY).await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);

    let forbidden = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .set_json(
                serde_json::json!({ "status": "confirmed", "reason": "volunteer cannot review" }),
            )
            .to_request(),
    )
    .await;
    assert_error(forbidden, StatusCode::FORBIDDEN, "forbidden").await;

    for status in ["confirmed", "needs_verification", "rejected"] {
        let response = test::call_service(
            &app,
            test::TestRequest::patch()
                .uri(&format!("/api/clues/{clue_id}/review"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
                .set_json(serde_json::json!({ "status": status, "reason": "commander review transition" }))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value = test::read_body_json(response).await;
        assert_eq!(body["occurred_at"], "2026-07-13T09:10:00Z");
        assert!(body["reported_at"].is_string());
        if status == "confirmed" {
            assert!(body["confirmed_at"].is_string());
        } else {
            assert_eq!(body["confirmed_at"], Value::Null);
        }
    }

    let audits = angui::entities::audit_events::Entity::find()
        .filter(angui::entities::audit_events::Column::EntityId.eq(&clue_id))
        .filter(angui::entities::audit_events::Column::Action.eq("clue.reviewed"))
        .all(&context.database)
        .await
        .expect("audit lookup should succeed");
    assert_eq!(audits.len(), 3);
    assert!(audits.iter().any(|audit| {
        audit
            .metadata_json
            .as_deref()
            .is_some_and(|metadata| metadata.contains("needs_verification"))
    }));
    assert!(audits.iter().any(|audit| {
        audit
            .metadata_json
            .as_deref()
            .is_some_and(|metadata| metadata.contains("rejected"))
    }));
}

#[actix_web::test]
async fn patch_clue_review_requires_a_reason_and_traces_duplicate_relationships() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let original_id = context.create_clue(&case_id, FAMILY).await;
    let duplicate_id = context.create_clue(&case_id, FAMILY).await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);

    let missing_reason = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{duplicate_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "status": "confirmed" }))
            .to_request(),
    )
    .await;
    assert_error(missing_reason, StatusCode::BAD_REQUEST, "validation_error").await;

    let missing_relationship = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{duplicate_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(
                serde_json::json!({ "status": "duplicate", "reason": "same original record" }),
            )
            .to_request(),
    )
    .await;
    assert_error(
        missing_relationship,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;

    let response = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{duplicate_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({
                "status": "duplicate",
                "reason": "same original record",
                "related_clue_id": original_id,
                "relationship_type": "duplicate_of",
                "next_action": "retain the earlier report"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["status"], "duplicate");
    assert_eq!(body["related_clue_id"], original_id);
    assert_eq!(body["relationship_type"], "duplicate_of");
    assert_eq!(body["review_reason"], "same original record");
    assert_eq!(body["next_action"], "retain the earlier report");

    let conflict_id = context.create_clue(&case_id, FAMILY).await;
    let conflict = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{conflict_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({
                "status": "conflicting",
                "reason": "reports give incompatible directions",
                "related_clue_id": original_id,
                "relationship_type": "conflicts_with"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::OK);
    let conflict_body: Value = test::read_body_json(conflict).await;
    assert_eq!(conflict_body["status"], "conflicting");
    assert_eq!(conflict_body["relationship_type"], "conflicts_with");
}

#[actix_web::test]
async fn patch_clue_review_records_a_confirm_then_retraction_without_silent_overwrite() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let family_token = context.token(FAMILY).await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);

    let created = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({
                "source": "family",
                "content": "Original clue requiring a follow-up.",
                "next_action": "contact the original reporter",
                "linked_task_reference": "task://follow-up/reporter"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: Value = test::read_body_json(created).await;
    let clue_id = created["id"].as_str().expect("clue id");

    let confirmed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({
                "status": "confirmed",
                "reason": "first review corroborated the original source"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(confirmed.status(), StatusCode::OK);
    let confirmed: Value = test::read_body_json(confirmed).await;
    assert!(confirmed["confirmed_at"].is_string());
    assert_eq!(confirmed["next_action"], "contact the original reporter");
    assert_eq!(
        confirmed["linked_task_reference"],
        "task://follow-up/reporter"
    );

    let retracted = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({
                "status": "needs_verification",
                "reason": "later feedback invalidated the corroboration"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(retracted.status(), StatusCode::OK);
    let retracted: Value = test::read_body_json(retracted).await;
    assert_eq!(retracted["confirmed_at"], Value::Null);
    assert_eq!(retracted["next_action"], "contact the original reporter");
    assert_eq!(
        retracted["linked_task_reference"],
        "task://follow-up/reporter"
    );

    let audits = angui::entities::audit_events::Entity::find()
        .filter(angui::entities::audit_events::Column::EntityId.eq(clue_id))
        .filter(angui::entities::audit_events::Column::Action.eq("clue.reviewed"))
        .all(&context.database)
        .await
        .expect("audit lookup should succeed");
    assert_eq!(audits.len(), 2);
    assert!(audits.iter().any(|audit| {
        audit.metadata_json.as_deref().is_some_and(|metadata| {
            metadata.contains("\"from\":\"confirmed\"")
                && metadata.contains("\"to\":\"needs_verification\"")
        })
    }));
}
