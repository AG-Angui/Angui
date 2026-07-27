use actix_web::{
    http::{StatusCode, header},
    test,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;

use crate::support::{FAMILY, TestContext, assert_error};
use angui::entities::audit_events;

#[actix_web::test]
async fn current_user_can_read_and_update_only_their_profile() {
    let context = TestContext::new().await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let get = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/users/me/profile")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;
    assert_eq!(get.status(), StatusCode::OK);
    let initial: Value = test::read_body_json(get).await;
    assert_eq!(initial["email"], FAMILY);
    assert_eq!(initial["preferences"]["locale"], "zh-CN");

    let update = test::call_service(&app, test::TestRequest::patch()
        .uri("/api/users/me/profile").insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "display_name": "Updated family", "avatar_reference": "avatar-demo-1", "preferences": { "locale": "en-US", "reduced_motion": true } })).to_request()).await;
    assert_eq!(update.status(), StatusCode::OK);
    let body: Value = test::read_body_json(update).await;
    assert_eq!(body["display_name"], "Updated family");
    assert_eq!(body["avatar_reference"], "avatar-demo-1");
    assert_eq!(body["preferences"]["reduced_motion"], true);

    let me = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/auth/me")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;
    let me: Value = test::read_body_json(me).await;
    assert_eq!(me["display_name"], "Updated family");

    let audit = audit_events::Entity::find()
        .filter(audit_events::Column::Action.eq("user.profile_updated"))
        .one(&context.database)
        .await
        .expect("profile audit query should succeed")
        .expect("profile audit should exist");
    let metadata = audit.metadata_json.expect("profile audit metadata");
    assert!(metadata.contains("changed_fields"));
    assert!(!metadata.contains("Updated family"));
    assert!(!metadata.contains("avatar-demo-1"));

    let forbidden_fields = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri("/api/users/me/profile")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(serde_json::json!({ "account_type": "learner" }))
            .to_request(),
    )
    .await;
    assert_error(
        forbidden_fields,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;
}
