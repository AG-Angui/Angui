use actix_web::{
    http::{StatusCode, header},
    test,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;

use angui::entities::audit_events;

use crate::support::{COMMANDER, FAMILY, TestContext, VOLUNTEER, assert_error, create_clue_json};

#[actix_web::test]
async fn patch_case_status_requires_commander_and_enforces_the_state_machine() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let family_token = context.token(FAMILY).await;
    let commander_token = context.token(COMMANDER).await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let app = crate::init_api_app!(&context);

    let forbidden = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/status"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "status": "resolved" }))
            .to_request(),
    )
    .await;
    assert_error(forbidden, StatusCode::FORBIDDEN, "forbidden").await;
    let volunteer_forbidden = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/status"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .set_json(serde_json::json!({ "status": "resolved" }))
            .to_request(),
    )
    .await;
    assert_error(volunteer_forbidden, StatusCode::FORBIDDEN, "forbidden").await;

    for expected_status in ["resolved", "active", "closed", "closed"] {
        let response = test::call_service(
            &app,
            test::TestRequest::patch()
                .uri(&format!("/api/cases/{case_id}/status"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
                .set_json(serde_json::json!({ "status": expected_status }))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value = test::read_body_json(response).await;
        assert_eq!(body["status"], expected_status);
    }
    let invalid_transition = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/status"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "status": "active" }))
            .to_request(),
    )
    .await;
    assert_error(invalid_transition, StatusCode::CONFLICT, "conflict").await;

    let closed_case_clue = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(create_clue_json())
            .to_request(),
    )
    .await;
    assert_error(closed_case_clue, StatusCode::CONFLICT, "conflict").await;

    let audits = audit_events::Entity::find()
        .filter(audit_events::Column::CaseId.eq(&case_id))
        .filter(audit_events::Column::Action.eq("case.status_changed"))
        .all(&context.database)
        .await
        .expect("status audit query should succeed");
    let transitions: Vec<Value> = audits
        .into_iter()
        .map(|audit| {
            serde_json::from_str(
                audit
                    .metadata_json
                    .as_deref()
                    .expect("status audit metadata"),
            )
            .expect("status audit metadata should be JSON")
        })
        .collect();
    for (from, to) in [
        ("active", "resolved"),
        ("resolved", "active"),
        ("active", "closed"),
        ("closed", "closed"),
    ] {
        assert!(
            transitions
                .iter()
                .any(|metadata| metadata["from"] == from && metadata["to"] == to),
            "missing audit transition from {from} to {to}"
        );
    }
}
