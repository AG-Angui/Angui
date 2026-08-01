use actix_web::{
    http::{StatusCode, header},
    test,
};
use angui::entities::{archive_drafts, audit_events};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;

use crate::support::{COMMANDER, FAMILY, LEARNER, TestContext, VOLUNTEER, assert_error};

#[actix_web::test]
async fn post_archive_drafts_requires_finished_commander_case_and_keeps_raw_material_out() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let clue_id = context.create_clue(&case_id, FAMILY).await;
    let app = crate::init_api_app!(&context);
    let commander_token = context.token(COMMANDER).await;

    let active_case = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/archive-drafts"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_error(active_case, StatusCode::CONFLICT, "conflict").await;

    let reviewed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "status": "confirmed", "reason": "fictional source verification" }))
            .to_request(),
    )
    .await;
    assert_eq!(reviewed.status(), StatusCode::OK);

    let resolved = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/status"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "status": "resolved" }))
            .to_request(),
    )
    .await;
    assert_eq!(resolved.status(), StatusCode::OK);

    let family = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/archive-drafts"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(FAMILY).await),
            ))
            .to_request(),
    )
    .await;
    assert_error(family, StatusCode::FORBIDDEN, "forbidden").await;

    let created = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/archive-drafts"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: serde_json::Value = test::read_body_json(created).await;
    let draft_id = created["id"].as_str().expect("archive draft id");
    assert_eq!(created["status"], "draft");
    assert_eq!(created["deidentification_status"], "manual_review_required");
    assert_eq!(
        created["source_scope"],
        json!([
            "confirmed_clue_review_material",
            "completed_task_review_material"
        ])
    );
    assert!(created["provider_model"].is_null());
    assert!(!created.to_string().contains("测试线索"));

    let stored = archive_drafts::Entity::find_by_id(draft_id)
        .one(&context.database)
        .await
        .expect("archive draft query should succeed")
        .expect("archive draft should be persisted");
    assert_eq!(stored.case_id, case_id);
    assert!(!stored.content.contains("测试线索"));
    assert!(!stored.content.contains("健康"));
    let audit = audit_events::Entity::find()
        .filter(audit_events::Column::EntityId.eq(draft_id))
        .filter(audit_events::Column::Action.eq("archive_draft.created"))
        .one(&context.database)
        .await
        .expect("archive audit query should succeed")
        .expect("archive audit should be written");
    assert!(!audit.metadata_json.unwrap_or_default().contains("测试线索"));

    let hidden = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/archive-drafts"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .to_request(),
    )
    .await;
    assert_error(hidden, StatusCode::NOT_FOUND, "not_found").await;

    let closed_case_id = context.create_case().await;
    context
        .add_member(&closed_case_id, FAMILY, COMMANDER, "commander")
        .await;
    context.close_case(&closed_case_id).await;
    let closed = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{closed_case_id}/archive-drafts"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(closed.status(), StatusCode::CREATED);
}
