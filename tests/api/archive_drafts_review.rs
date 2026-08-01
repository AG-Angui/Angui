use actix_web::{
    http::{StatusCode, header},
    test,
};
use angui::entities::{archive_drafts, audit_events, cases};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};

use crate::support::{ADMIN, COMMANDER, FAMILY, TestContext, assert_error};

macro_rules! create_finished_archive_draft {
    ($context:expr, $app:expr, $commander_token:expr) => {{
        let case_id = $context.create_case().await;
        $context
            .add_member(&case_id, FAMILY, COMMANDER, "commander")
            .await;
        $context.close_case(&case_id).await;
        let created = test::call_service(
            $app,
            test::TestRequest::post()
                .uri(&format!("/api/cases/{case_id}/archive-drafts"))
                .insert_header((
                    header::AUTHORIZATION,
                    format!("Bearer {}", $commander_token),
                ))
                .to_request(),
        )
        .await;
        assert_eq!(created.status(), StatusCode::CREATED);
        let created: Value = test::read_body_json(created).await;
        (
            case_id,
            created["id"]
                .as_str()
                .expect("archive draft id should be returned")
                .to_owned(),
        )
    }};
}

#[actix_web::test]
async fn archive_deidentification_and_review_require_admin_and_preserve_auditable_lifecycle() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let commander_token = context.token(COMMANDER).await;
    let admin_token = context.token(ADMIN).await;
    let (case_id, draft_id) = create_finished_archive_draft!(&context, &app, &commander_token);

    let family_deidentify = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/archive-drafts/{draft_id}/deidentify"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(FAMILY).await),
            ))
            .set_json(json!({ "outcome": "confirm", "reason": "fictional review" }))
            .to_request(),
    )
    .await;
    assert_error(family_deidentify, StatusCode::FORBIDDEN, "forbidden").await;

    let premature_publish = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/admin/archive-drafts/{draft_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({ "action": "publish", "reason": "fictional release" }))
            .to_request(),
    )
    .await;
    assert_error(premature_publish, StatusCode::CONFLICT, "conflict").await;

    let invalid_deidentify = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/archive-drafts/{draft_id}/deidentify"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({ "outcome": "automatic", "reason": "fictional review" }))
            .to_request(),
    )
    .await;
    assert_error(
        invalid_deidentify,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;

    let deidentified = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/archive-drafts/{draft_id}/deidentify"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({ "outcome": "confirm", "reason": "fictional manual de-identification confirmation", "deidentified_material": "At an unspecified time, a confirmed review item and a completed task were recorded. Exact identities, contacts, health details, locations, and routes were removed." }))
            .to_request(),
    )
    .await;
    assert_eq!(deidentified.status(), StatusCode::OK);
    let deidentified: Value = test::read_body_json(deidentified).await;
    assert_eq!(deidentified["status"], "pending_review");
    assert_eq!(deidentified["deidentification_status"], "deidentified");
    assert_eq!(deidentified["version"], 2);
    assert!(deidentified["deidentified_at"].is_string());
    assert!(
        !deidentified
            .to_string()
            .contains("fictional manual de-identification confirmation"),
        "the de-identification reason must not be returned in the archive response"
    );

    let published = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/admin/archive-drafts/{draft_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(
                json!({ "action": "publish", "reason": "fictional controlled learning approval" }),
            )
            .to_request(),
    )
    .await;
    assert_eq!(published.status(), StatusCode::OK);
    let published: Value = test::read_body_json(published).await;
    assert_eq!(published["status"], "published");
    assert_eq!(published["usage_scope"], "learning_resource");
    assert_eq!(published["retention_status"], "retained");
    assert_eq!(published["version"], 3);
    assert!(published["reviewed_at"].is_string());
    assert!(
        !published
            .to_string()
            .contains("fictional controlled learning approval"),
        "the review reason must not be returned in the archive response"
    );

    let withdrawn = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/admin/archive-drafts/{draft_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(
                json!({ "action": "withdraw", "reason": "fictional correction and withdrawal" }),
            )
            .to_request(),
    )
    .await;
    assert_eq!(withdrawn.status(), StatusCode::OK);
    let withdrawn: Value = test::read_body_json(withdrawn).await;
    assert_eq!(withdrawn["status"], "withdrawn");
    assert_eq!(withdrawn["usage_scope"], "internal_archive");
    assert_eq!(withdrawn["retention_status"], "withdrawn");
    assert_eq!(withdrawn["version"], 4);

    let stored = archive_drafts::Entity::find_by_id(&draft_id)
        .one(&context.database)
        .await
        .expect("archive draft query should succeed")
        .expect("archive draft should remain persisted for traceability");
    assert_eq!(stored.case_id, case_id);
    assert_eq!(stored.status, "withdrawn");
    assert_eq!(stored.version, 4);
    assert!(stored.content.contains("Timeline") || stored.content.contains("de-identified"));
    assert!(
        cases::Entity::find_by_id(&case_id)
            .one(&context.database)
            .await
            .expect("case query should succeed")
            .is_some(),
        "withdrawing an archive draft must not affect the source case"
    );
    let audits = audit_events::Entity::find()
        .filter(audit_events::Column::EntityId.eq(&draft_id))
        .all(&context.database)
        .await
        .expect("archive audits should be queryable");
    assert!(
        audits
            .iter()
            .any(|audit| audit.action == "archive_draft.deidentification_reviewed")
    );
    assert!(
        audits
            .iter()
            .any(|audit| audit.action == "archive_draft.reviewed")
    );
    assert!(audits.iter().all(|audit| {
        !audit
            .metadata_json
            .as_deref()
            .unwrap_or_default()
            .contains("fictional manual de-identification confirmation")
    }));
}

#[actix_web::test]
async fn archive_deidentification_rejection_and_missing_drafts_do_not_publish_or_mutate_cases() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let commander_token = context.token(COMMANDER).await;
    let admin_token = context.token(ADMIN).await;
    let (case_id, draft_id) = create_finished_archive_draft!(&context, &app, &commander_token);

    let rejected = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/archive-drafts/{draft_id}/deidentify"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({ "outcome": "reject", "reason": "fictional unsafe de-identification result" }))
            .to_request(),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::OK);
    let rejected: Value = test::read_body_json(rejected).await;
    assert_eq!(rejected["status"], "rejected");
    assert_eq!(rejected["deidentification_status"], "rejected");
    assert_eq!(rejected["usage_scope"], "internal_archive");
    assert_eq!(rejected["version"], 2);

    let rejected_publish = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/admin/archive-drafts/{draft_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({ "action": "publish", "reason": "fictional improper release" }))
            .to_request(),
    )
    .await;
    assert_error(rejected_publish, StatusCode::CONFLICT, "conflict").await;

    let review_rejection = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/archive-drafts"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(review_rejection.status(), StatusCode::CREATED);
    let review_rejection: Value = test::read_body_json(review_rejection).await;
    let review_rejection_id = review_rejection["id"]
        .as_str()
        .expect("second archive draft id should be returned");
    let deidentified_for_rejection = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!(
                "/api/admin/archive-drafts/{review_rejection_id}/deidentify"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({ "outcome": "confirm", "reason": "fictional manual confirmation before review rejection", "deidentified_material": "De-identified test material for review rejection." }))
            .to_request(),
    )
    .await;
    assert_eq!(deidentified_for_rejection.status(), StatusCode::OK);
    let review_rejected = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!(
                "/api/admin/archive-drafts/{review_rejection_id}/review"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(
                json!({ "action": "reject", "reason": "fictional archive quality rejection" }),
            )
            .to_request(),
    )
    .await;
    assert_eq!(review_rejected.status(), StatusCode::OK);
    let review_rejected: Value = test::read_body_json(review_rejected).await;
    assert_eq!(review_rejected["status"], "rejected");
    assert_eq!(review_rejected["usage_scope"], "internal_archive");
    assert_eq!(review_rejected["retention_status"], "retained");
    assert_eq!(review_rejected["version"], 3);
    assert!(
        !review_rejected
            .to_string()
            .contains("fictional archive quality rejection")
    );
    assert!(
        cases::Entity::find_by_id(&case_id)
            .one(&context.database)
            .await
            .expect("case query should succeed")
            .is_some()
    );

    let missing = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/archive-drafts/missing-draft/deidentify")
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({ "outcome": "confirm", "reason": "fictional missing resource" }))
            .to_request(),
    )
    .await;
    assert_error(missing, StatusCode::NOT_FOUND, "not_found").await;
}

#[actix_web::test]
async fn archive_review_material_versions_support_admin_list_diff_restore_and_rbac() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let commander_token = context.token(COMMANDER).await;
    let admin_token = context.token(ADMIN).await;
    let (case_id, draft_id) = create_finished_archive_draft!(&context, &app, &commander_token);

    let family_list = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/admin/archive-drafts/{draft_id}/review-materials"
            ))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(FAMILY).await),
            ))
            .to_request(),
    )
    .await;
    assert_error(family_list, StatusCode::FORBIDDEN, "forbidden").await;

    let initial = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/admin/archive-drafts/{draft_id}/review-materials"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(initial.status(), StatusCode::OK);
    let initial: Vec<Value> = test::read_body_json(initial).await;
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0]["version"], 1);
    assert_eq!(initial[0]["status"], "draft");
    assert_eq!(initial[0]["selected_for_ai"], true);
    assert_eq!(
        initial[0]["source_scope"][0],
        "confirmed_clue_review_material"
    );

    let deidentified = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/archive-drafts/{draft_id}/deidentify"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({
                "outcome": "confirm",
                "reason": "version test",
                "deidentified_material": "line one\nline two"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(deidentified.status(), StatusCode::OK);

    let versions = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/admin/archive-drafts/{draft_id}/review-materials"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    let versions: Vec<Value> = test::read_body_json(versions).await;
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0]["version"], 2);
    assert_eq!(versions[0]["status"], "deidentified");
    assert_eq!(versions[0]["selected_for_ai"], true);
    assert_eq!(versions[1]["selected_for_ai"], false);

    let diff = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/admin/archive-drafts/{draft_id}/review-materials/diff/1/2"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(diff.status(), StatusCode::OK);
    let diff: Value = test::read_body_json(diff).await;
    assert_eq!(diff["from_version"], 1);
    assert_eq!(diff["to_version"], 2);
    assert!(
        diff["added"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let restored = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!(
                "/api/admin/archive-drafts/{draft_id}/review-materials/2/restore"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({"reason": "restore approved version for correction"}))
            .to_request(),
    )
    .await;
    assert_eq!(restored.status(), StatusCode::OK);
    let restored: Value = test::read_body_json(restored).await;
    assert_eq!(restored["status"], "pending_review");
    assert_eq!(restored["version"], 3);
    let selected_id = restored["review_material_id"]
        .as_str()
        .expect("restored draft should select a material");

    let versions_after_restore = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/admin/archive-drafts/{draft_id}/review-materials"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    let versions_after_restore: Vec<Value> = test::read_body_json(versions_after_restore).await;
    assert_eq!(versions_after_restore.len(), 3);
    assert_eq!(versions_after_restore[0]["version"], 3);
    assert_eq!(versions_after_restore[0]["status"], "deidentified");
    assert_eq!(versions_after_restore[0]["selected_for_ai"], true);
    assert_eq!(
        versions_after_restore[0]["parent_material_id"],
        versions_after_restore[1]["id"]
    );
    assert_eq!(versions_after_restore[0]["id"], selected_id);
    assert_eq!(versions_after_restore[1]["content"], "line one\nline two");

    let audit = audit_events::Entity::find()
        .filter(audit_events::Column::EntityType.eq("archive_review_material"))
        .filter(audit_events::Column::EntityId.eq(selected_id))
        .one(&context.database)
        .await
        .expect("material audit query should succeed")
        .expect("restore should be audited");
    assert_eq!(audit.action, "archive_review_material.restored");
    assert!(!audit.metadata_json.unwrap_or_default().contains("line one"));
    assert_eq!(case_id, restored["case_id"]);
}
