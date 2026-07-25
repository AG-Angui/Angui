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
            .set_json(serde_json::json!({ "status": "confirmed" }))
            .to_request(),
    )
    .await;
    assert_error(forbidden, StatusCode::FORBIDDEN, "forbidden").await;
    let confirmed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "status": "confirmed" }))
            .to_request(),
    )
    .await;
    assert_eq!(confirmed.status(), StatusCode::OK);
    let body: Value = test::read_body_json(confirmed).await;
    assert_eq!(body["status"], "confirmed");
    assert!(body["reviewed_at"].is_string());

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
            .set_json(serde_json::json!({ "status": "pending_review" }))
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
            .set_json(serde_json::json!({ "status": "confirmed" }))
            .to_request(),
    )
    .await;
    assert_error(forbidden, StatusCode::FORBIDDEN, "forbidden").await;

    for status in ["needs_verification", "duplicate"] {
        let response = test::call_service(
            &app,
            test::TestRequest::patch()
                .uri(&format!("/api/clues/{clue_id}/review"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
                .set_json(serde_json::json!({ "status": status }))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    let audits = angui::entities::audit_events::Entity::find()
        .filter(angui::entities::audit_events::Column::EntityId.eq(&clue_id))
        .filter(angui::entities::audit_events::Column::Action.eq("clue.reviewed"))
        .all(&context.database)
        .await
        .expect("audit lookup should succeed");
    assert_eq!(audits.len(), 2);
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
            .is_some_and(|metadata| metadata.contains("duplicate"))
    }));
}
