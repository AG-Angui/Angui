use actix_web::{
    http::{StatusCode, header},
    test,
};
use angui::entities::{audit_events, auth_sessions};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};

use crate::support::{ADMIN, FAMILY, TestContext, VOLUNTEER, assert_error};

#[actix_web::test]
async fn admin_endpoints_enforce_capability_redact_data_and_revoke_disabled_sessions() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family = context.authenticated(FAMILY).await;
    let family_token = context.token(FAMILY).await;
    let admin_token = context.token(ADMIN).await;
    let app = crate::init_api_app!(&context);

    let denied = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/admin/users")
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    assert_error(denied, StatusCode::FORBIDDEN, "forbidden").await;

    let users = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/admin/users?status=active&sort=email&order=asc&page=1&page_size=3")
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(users.status(), StatusCode::OK);
    let users: Value = test::read_body_json(users).await;
    assert_eq!(users["page"], 1);
    assert_eq!(users["page_size"], 3);
    assert!(
        users["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(users.get("password_hash").is_none());
    assert!(!users.to_string().contains("token_hash"));
    let family_entry = users["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["id"] == family.id))
        .expect("family user should be in the first page of demo accounts");
    assert_eq!(family_entry["status"], "active");
    assert!(family_entry["last_session_at"].is_string());
    assert!(
        audit_events::Entity::find()
            .filter(audit_events::Column::Action.eq("admin.users_listed"))
            .one(&context.database)
            .await
            .expect("admin users list audit query should succeed")
            .is_some()
    );

    let audit_events_page = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/admin/audit-events?action=admin.users_listed&sort=created_at&order=desc")
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(audit_events_page.status(), StatusCode::OK);
    let audit_events_page: Value = test::read_body_json(audit_events_page).await;
    assert!(audit_events_page["items"].as_array().is_some_and(|items| {
        items
            .iter()
            .all(|item| item["action"] == "admin.users_listed")
    }));
    assert!(
        audit_events_page["items"]
            .as_array()
            .is_some_and(|items| items.iter().all(|item| item.get("metadata_json").is_none()))
    );
    assert!(
        audit_events::Entity::find()
            .filter(audit_events::Column::Action.eq("admin.audit_events_listed"))
            .one(&context.database)
            .await
            .expect("admin audit event list access should be audited")
            .is_some()
    );

    let admin_cannot_read_case = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_error(admin_cannot_read_case, StatusCode::NOT_FOUND, "not_found").await;

    let invalid_status = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/admin/users/{}/status", family.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({ "status": "deleted", "reason": "fictional invalid transition" }))
            .to_request(),
    )
    .await;
    assert_error(invalid_status, StatusCode::BAD_REQUEST, "validation_error").await;

    let disabled = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/admin/users/{}/status", family.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({ "status": "disabled", "reason": "fictional account review" }))
            .to_request(),
    )
    .await;
    assert_eq!(disabled.status(), StatusCode::OK);
    let disabled: Value = test::read_body_json(disabled).await;
    assert_eq!(disabled["status"], "disabled");
    assert!(
        auth_sessions::Entity::find()
            .filter(auth_sessions::Column::UserId.eq(&family.id))
            .filter(auth_sessions::Column::RevokedAt.is_null())
            .one(&context.database)
            .await
            .expect("revoked family sessions query should succeed")
            .is_none()
    );
    let disabled_token = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/auth/me")
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    assert_error(disabled_token, StatusCode::UNAUTHORIZED, "unauthorized").await;
    let status_audit = audit_events::Entity::find()
        .filter(audit_events::Column::EntityId.eq(&family.id))
        .filter(audit_events::Column::Action.eq("admin.user_status_changed"))
        .one(&context.database)
        .await
        .expect("status audit query should succeed")
        .expect("status update should be audited");
    let metadata = status_audit.metadata_json.expect("status audit metadata");
    assert!(metadata.contains("previous_status"));
    assert!(metadata.contains("reason_length"));
    assert!(!metadata.contains("fictional account review"));

    let reactivated = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/admin/users/{}/status", family.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({ "status": "active", "reason": "fictional reactivation" }))
            .to_request(),
    )
    .await;
    assert_eq!(reactivated.status(), StatusCode::OK);
    let restored_token = context.token(FAMILY).await;
    let locked = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/admin/users/{}/status", family.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({ "status": "locked", "reason": "fictional lock review" }))
            .to_request(),
    )
    .await;
    assert_eq!(locked.status(), StatusCode::OK);
    let locked_token = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/cases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {restored_token}")))
            .to_request(),
    )
    .await;
    assert_error(locked_token, StatusCode::UNAUTHORIZED, "unauthorized").await;

    let volunteer_token = context.token(VOLUNTEER).await;
    let non_admin_audit = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/admin/audit-events")
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    assert_error(non_admin_audit, StatusCode::FORBIDDEN, "forbidden").await;
}
