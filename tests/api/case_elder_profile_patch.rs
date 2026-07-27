use actix_web::{
    http::{StatusCode, header},
    test,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;

use crate::support::{ADMIN, COMMANDER, FAMILY, TestContext, VOLUNTEER, assert_error};
use angui::entities::{audit_events, elder_profile_revisions};

#[actix_web::test]
async fn family_and_commander_revision_updates_while_volunteers_are_denied() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let family = context.token(FAMILY).await;
    let commander = context.token(COMMANDER).await;
    let volunteer = context.token(VOLUNTEER).await;
    let admin = context.token(ADMIN).await;
    let app = crate::init_api_app!(&context);

    let updated = test::call_service(&app, test::TestRequest::patch()
        .uri(&format!("/api/cases/{case_id}/elder-profile"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {family}")))
        .set_json(serde_json::json!({ "health_notes": "fictional changed note", "last_seen_location": "Updated fictional place" })).to_request()).await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated: Value = test::read_body_json(updated).await;
    assert_eq!(
        updated["elder_profile"]["last_seen_location"],
        "Updated fictional place"
    );

    let commander_update = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/elder-profile"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander}")))
            .set_json(
                serde_json::json!({ "physical_description": "updated fictional description" }),
            )
            .to_request(),
    )
    .await;
    assert_eq!(commander_update.status(), StatusCode::OK);

    let volunteer_denied = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/elder-profile"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer}")))
            .set_json(serde_json::json!({ "display_name": "Not allowed" }))
            .to_request(),
    )
    .await;
    assert_error(volunteer_denied, StatusCode::FORBIDDEN, "forbidden").await;
    let admin_hidden = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/elder-profile"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin}")))
            .set_json(serde_json::json!({ "display_name": "Not allowed" }))
            .to_request(),
    )
    .await;
    assert_error(admin_hidden, StatusCode::NOT_FOUND, "not_found").await;
    let blocked_field = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{case_id}/elder-profile"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family}")))
            .set_json(serde_json::json!({ "status": "closed" }))
            .to_request(),
    )
    .await;
    assert_error(blocked_field, StatusCode::BAD_REQUEST, "validation_error").await;

    let revisions = elder_profile_revisions::Entity::find()
        .filter(elder_profile_revisions::Column::CaseId.eq(&case_id))
        .all(&context.database)
        .await
        .expect("revision query should succeed");
    assert_eq!(revisions.len(), 2);
    assert!(revisions.iter().all(
        |revision| revision.previous_profile_json.contains("health_notes")
            && revision.updated_profile_json.contains("display_name")
    ));
    let audit = audit_events::Entity::find()
        .filter(audit_events::Column::Action.eq("elder_profile.updated"))
        .one(&context.database)
        .await
        .expect("audit query should succeed")
        .expect("summary audit should exist");
    let metadata = audit.metadata_json.expect("summary audit metadata");
    assert!(metadata.contains("changed_fields"));
    assert!(!metadata.contains("fictional changed note"));
}
