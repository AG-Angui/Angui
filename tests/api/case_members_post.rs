use actix_web::{
    http::{StatusCode, header},
    test,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;

use angui::entities::{audit_events, user_global_capabilities, users};

use crate::support::{ADMIN, COMMANDER, FAMILY, LEARNER, TestContext, VOLUNTEER, assert_error};

#[actix_web::test]
async fn post_case_members_applies_invitation_role_and_duplicate_rules() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let invite_commander = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "email": COMMANDER, "case_role": "commander" }))
            .to_request(),
    )
    .await;
    assert_eq!(invite_commander.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(invite_commander).await;
    assert_eq!(body["email"], COMMANDER);
    assert_eq!(body["account_type"], "member");
    assert_eq!(
        body["global_capabilities"],
        serde_json::json!(["commander"])
    );
    assert_eq!(body["case_role"], "commander");

    let family = context.authenticated(FAMILY).await;
    let audit = audit_events::Entity::find()
        .filter(audit_events::Column::CaseId.eq(&case_id))
        .filter(audit_events::Column::Action.eq("case.member_added"))
        .one(&context.database)
        .await
        .expect("member invitation audit should be readable")
        .expect("successful invitation should be audited");
    assert_eq!(audit.actor, family.id);
    assert_eq!(audit.entity_type, "user");
    let metadata: Value =
        serde_json::from_str(audit.metadata_json.as_deref().expect("audit metadata"))
            .expect("audit metadata should be JSON");
    assert_eq!(metadata["case_role"], "commander");

    let duplicate = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "email": COMMANDER, "case_role": "commander" }))
            .to_request(),
    )
    .await;
    assert_error(duplicate, StatusCode::CONFLICT, "conflict").await;
    let family_cannot_invite_volunteer = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "email": VOLUNTEER, "case_role": "volunteer" }))
            .to_request(),
    )
    .await;
    assert_error(
        family_cannot_invite_volunteer,
        StatusCode::FORBIDDEN,
        "forbidden",
    )
    .await;

    let family_member = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({ "email": ADMIN, "case_role": "family" }))
            .to_request(),
    )
    .await;
    assert_eq!(family_member.status(), StatusCode::CREATED);

    let unrelated_case_id = context.create_case().await;
    let commander_token = context.token(COMMANDER).await;
    let unrelated_case = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{unrelated_case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "email": VOLUNTEER, "case_role": "volunteer" }))
            .to_request(),
    )
    .await;
    assert_error(unrelated_case, StatusCode::NOT_FOUND, "not_found").await;
}

#[actix_web::test]
async fn commander_accepts_a_minimal_pending_case_without_global_case_visibility() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);

    let queue = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/cases/command-intake")
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(queue.status(), StatusCode::OK);
    let queue_body: Value = test::read_body_json(queue).await;
    let item = queue_body
        .as_array()
        .expect("queue should be an array")
        .iter()
        .find(|item| item["id"] == case_id)
        .expect("new family case should be pending");
    assert!(item.get("health_notes").is_none());
    assert!(item.get("display_name").is_none());

    let accepted = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/accept-command"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(accepted.status(), StatusCode::OK);
    let accepted_body: Value = test::read_body_json(accepted).await;
    assert_eq!(accepted_body["access_role"], "commander");

    let duplicate = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/accept-command"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_error(duplicate, StatusCode::CONFLICT, "conflict").await;
}

#[actix_web::test]
async fn post_case_members_allows_a_commander_to_assign_an_operational_account() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "email": VOLUNTEER, "case_role": "volunteer" }))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[actix_web::test]
async fn post_case_members_requires_matching_capabilities_and_never_auto_grants_access() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);

    let unauthorized_assignment = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "email": ADMIN, "case_role": "volunteer" }))
            .to_request(),
    )
    .await;
    assert_error(unauthorized_assignment, StatusCode::FORBIDDEN, "forbidden").await;

    let absent_admin_token = context.token(ADMIN).await;
    let absent_admin = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {absent_admin_token}"),
            ))
            .to_request(),
    )
    .await;
    assert_eq!(absent_admin.status(), StatusCode::NOT_FOUND);

    let controlled_assignment = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "email": ADMIN, "case_role": "family" }))
            .to_request(),
    )
    .await;
    assert_eq!(controlled_assignment.status(), StatusCode::CREATED);
    let assigned_body: Value = test::read_body_json(controlled_assignment).await;
    assert_eq!(assigned_body["account_type"], "member");
    assert_eq!(
        assigned_body["global_capabilities"],
        serde_json::json!(["admin"])
    );
    assert_eq!(assigned_body["case_role"], "family");

    let learner = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "email": LEARNER, "case_role": "family" }))
            .to_request(),
    )
    .await;
    assert_error(learner, StatusCode::FORBIDDEN, "forbidden").await;

    users::ActiveModel {
        id: Set("both-capable-user".to_owned()),
        email: Set("both-capable@demo.invalid".to_owned()),
        display_name: Set("Both capable member".to_owned()),
        account_type: Set("member".to_owned()),
        password_hash: Set("unused-in-this-test".to_owned()),
        status: Set("active".to_owned()),
        created_at: Set("2026-07-24T00:00:00Z".to_owned()),
        updated_at: Set("2026-07-24T00:00:00Z".to_owned()),
    }
    .insert(&context.database)
    .await
    .expect("both-capable fixture should insert");
    for capability in ["commander", "volunteer"] {
        user_global_capabilities::ActiveModel {
            user_id: Set("both-capable-user".to_owned()),
            capability: Set(capability.to_owned()),
            created_at: Set("2026-07-24T00:00:00Z".to_owned()),
        }
        .insert(&context.database)
        .await
        .expect("both-capable fixture capability should insert");
    }

    let commander_assignment = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "email": "both-capable@demo.invalid", "case_role": "commander" }))
            .to_request(),
    )
    .await;
    assert_eq!(commander_assignment.status(), StatusCode::CREATED);

    let second_case_id = context.create_case().await;
    context
        .add_member(&second_case_id, FAMILY, COMMANDER, "commander")
        .await;
    let volunteer_assignment = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{second_case_id}/members"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "email": "both-capable@demo.invalid", "case_role": "volunteer" }))
            .to_request(),
    )
    .await;
    assert_eq!(volunteer_assignment.status(), StatusCode::CREATED);
}
