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
            .set_json(json!({ "outcome": "confirm", "reason": "fictional manual de-identification confirmation" }))
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
    assert!(stored.content.contains("Source scope is limited"));
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
            .set_json(json!({ "outcome": "confirm", "reason": "fictional manual confirmation before review rejection" }))
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
