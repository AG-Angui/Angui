use actix_web::{
    http::{StatusCode, header},
    test,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;

use crate::support::{COMMANDER, FAMILY, TestContext, VOLUNTEER, assert_error, create_clue_json};

#[actix_web::test]
async fn post_case_clues_creates_pending_review_clues_for_case_members() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(create_clue_json())
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["case_id"], case_id);
    assert_eq!(body["status"], "pending_review");
    assert_eq!(body["is_own_submission"], true);
    assert!(body["reviewed_at"].is_null());

    let audit = angui::entities::audit_events::Entity::find()
        .filter(angui::entities::audit_events::Column::EntityId.eq(body["id"].as_str()))
        .one(&context.database)
        .await
        .expect("audit lookup should succeed")
        .expect("clue submission should be audited");
    let metadata = audit
        .metadata_json
        .expect("audit metadata should be present");
    assert!(metadata.contains("pending_review"));
    assert!(
        !metadata.contains(
            create_clue_json()["content"]
                .as_str()
                .expect("content is text")
        )
    );
}

#[actix_web::test]
async fn post_case_clues_hides_non_member_cases_and_rejects_non_active_cases() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let non_member = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .set_json(create_clue_json())
            .to_request(),
    )
    .await;
    assert_error(non_member, StatusCode::NOT_FOUND, "not_found").await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context.close_case(&case_id).await;
    let closed = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(create_clue_json())
            .to_request(),
    )
    .await;
    assert_error(closed, StatusCode::CONFLICT, "conflict").await;

    let resolved_case_id = context.create_case().await;
    context
        .add_member(&resolved_case_id, FAMILY, COMMANDER, "commander")
        .await;
    angui::services::case_service::update_case_status(
        &context.database,
        &context.authenticated(COMMANDER).await,
        &resolved_case_id,
        angui::models::UpdateCaseStatusRequest {
            status: "resolved".to_owned(),
        },
    )
    .await
    .expect("fixture case should resolve");
    let resolved = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{resolved_case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(create_clue_json())
            .to_request(),
    )
    .await;
    assert_error(resolved, StatusCode::CONFLICT, "conflict").await;
}

#[actix_web::test]
async fn post_case_clues_rejects_client_controlled_status_and_unknown_fields() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);

    for payload in [
        serde_json::json!({
            "source": "family",
            "content": "untrusted status",
            "status": "confirmed"
        }),
        serde_json::json!({
            "source": "family",
            "content": "unknown field",
            "reviewed_by_user_id": "forged"
        }),
        serde_json::json!({
            "source": "family",
            "content": "forged lifecycle time",
            "reported_at": "2026-07-13T09:10:00Z"
        }),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/cases/{case_id}/clues"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .set_json(payload)
                .to_request(),
        )
        .await;
        assert_error(response, StatusCode::BAD_REQUEST, "validation_error").await;
    }
}

#[actix_web::test]
async fn post_case_clues_keeps_source_provenance_and_leaves_missing_draft_fields_empty() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "source": "family chat",
                "source_type": "field_report",
                "content": "Original quoted message retained for review.",
                "raw_record_reference": "controlled://chat/record-16",
                "occurred_at": null,
                "location_text": "near the fictional park",
                "location_precision": " Approximate ",
                "next_action": "ask the reporter for a time window"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["status"], "pending_review");
    assert_eq!(body["source_type"], "field_report");
    assert_eq!(body["raw_record_reference"], "controlled://chat/record-16");
    assert_eq!(body["occurred_at"], Value::Null);
    assert_eq!(body["confirmed_at"], Value::Null);
    assert!(body["reported_at"].is_string());
    assert_eq!(body["location_precision"], "approximate");
}

#[actix_web::test]
async fn post_case_clues_rejects_client_claimed_ai_and_chat_draft_sources() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);

    for source_type in ["ai_draft", "chat_draft"] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/cases/{case_id}/clues"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .set_json(serde_json::json!({
                    "source": "untrusted client",
                    "source_type": source_type,
                    "content": "client must not claim an automated source"
                }))
                .to_request(),
        )
        .await;
        assert_error(response, StatusCode::BAD_REQUEST, "validation_error").await;
    }
}

#[actix_web::test]
async fn post_case_clues_rejects_ambiguous_location_precision() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(serde_json::json!({
                "source": "family",
                "content": "precision without a location is ambiguous",
                "location_precision": "exact"
            }))
            .to_request(),
    )
    .await;
    assert_error(response, StatusCode::BAD_REQUEST, "validation_error").await;
}
