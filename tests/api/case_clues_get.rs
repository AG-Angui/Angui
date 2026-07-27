use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::Value;

use crate::support::{COMMANDER, FAMILY, LEARNER, TestContext, VOLUNTEER, assert_error};

#[actix_web::test]
async fn get_case_clues_applies_role_cuts_pagination_and_status_filters() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;

    let confirmed_clue = context.create_clue(&case_id, FAMILY).await;
    let commander = context.authenticated(COMMANDER).await;
    angui::services::case_service::review_clue(
        &context.database,
        &commander,
        &confirmed_clue,
        angui::models::ReviewClueRequest {
            status: "confirmed".to_owned(),
            reason: "fixture review".to_owned(),
            related_clue_id: None,
            relationship_type: None,
            next_action: None,
            linked_task_reference: None,
        },
    )
    .await
    .expect("fixture clue should be confirmed");
    context.create_clue(&case_id, COMMANDER).await;
    context.create_clue(&case_id, FAMILY).await;
    angui::services::case_service::create_clue(
        &context.database,
        &commander,
        &case_id,
        angui::models::CreateClueRequest {
            source: "field responder".to_owned(),
            source_type: Some("field_report".to_owned()),
            content: "A fictional field report for source filtering.".to_owned(),
            raw_record_reference: None,
            occurred_at: None,
            location_text: None,
            location_precision: None,
            next_action: None,
            linked_task_reference: None,
            attachment_ids: Vec::new(),
        },
    )
    .await
    .expect("fixture field report should be created");

    let family_token = context.token(FAMILY).await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let commander_token = context.token(COMMANDER).await;
    let learner_token = context.token(LEARNER).await;
    let app = crate::init_api_app!(&context);

    let family = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    let family_body: Value = test::read_body_json(family).await;
    assert_eq!(family_body["total"], 2);
    assert!(
        family_body["items"]
            .as_array()
            .expect("items should be an array")
            .iter()
            .all(|clue| clue["status"] == "confirmed" || clue["is_own_submission"] == true)
    );

    let volunteer = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    let volunteer_body: Value = test::read_body_json(volunteer).await;
    assert_eq!(volunteer_body["total"], 1);
    assert_eq!(volunteer_body["items"][0]["status"], "confirmed");
    assert_eq!(volunteer_body["items"][0]["review_reason"], Value::Null);
    assert_eq!(
        volunteer_body["items"][0]["attachment_ids"],
        serde_json::json!([])
    );

    let paged = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/cases/{case_id}/clues?page=1&page_size=2&sort=created_at&order=asc"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    let paged_body: Value = test::read_body_json(paged).await;
    assert_eq!(paged_body["total"], 4);
    assert_eq!(paged_body["items"].as_array().map(Vec::len), Some(2));
    assert_eq!(paged_body["page"], 1);
    assert_eq!(paged_body["page_size"], 2);

    let filtered = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/clues?status=confirmed"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    let filtered_body: Value = test::read_body_json(filtered).await;
    assert_eq!(filtered_body["total"], 1);
    assert_eq!(filtered_body["items"][0]["id"], confirmed_clue);

    let typed = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/cases/{case_id}/clues?source_type=manual_report"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    let typed_body: Value = test::read_body_json(typed).await;
    assert_eq!(typed_body["total"], 3);
    assert!(
        typed_body["items"]
            .as_array()
            .expect("items should be an array")
            .iter()
            .all(|clue| clue["source_type"] == "manual_report")
    );

    let field_reports = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/cases/{case_id}/clues?source_type=field_report"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    let field_reports_body: Value = test::read_body_json(field_reports).await;
    assert_eq!(field_reports_body["total"], 1);
    assert_eq!(
        field_reports_body["items"][0]["source_type"],
        "field_report"
    );

    let unavailable = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {learner_token}")))
            .to_request(),
    )
    .await;
    assert_error(unavailable, StatusCode::NOT_FOUND, "not_found").await;
}

#[actix_web::test]
async fn get_case_clues_rejects_invalid_whitelisted_query_values() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);

    for query in [
        "page=0",
        "page_size=101",
        "status=unreviewed",
        "source_type=untrusted_client_value",
        "sort=status",
        "order=sideways",
        "unexpected=value",
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/cases/{case_id}/clues?{query}"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .to_request(),
        )
        .await;
        assert_error(response, StatusCode::BAD_REQUEST, "validation_error").await;
    }
}

#[actix_web::test]
async fn get_case_clues_redacts_controlled_raw_references_for_non_submitters() {
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

    let created = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(serde_json::json!({
                "source": "family",
                "source_type": "manual_report",
                "content": "A fictional confirmed observation",
                "raw_record_reference": "controlled://source/record-16"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: Value = test::read_body_json(created).await;
    let clue_id = created["id"].as_str().expect("clue id");

    let reviewed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(serde_json::json!({ "status": "confirmed", "reason": "commander reviewed the controlled record" }))
            .to_request(),
    )
    .await;
    assert_eq!(reviewed.status(), StatusCode::OK);

    let commander = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    let commander: Value = test::read_body_json(commander).await;
    assert_eq!(
        commander["items"][0]["raw_record_reference"],
        "controlled://source/record-16"
    );

    let volunteer = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(volunteer.status(), StatusCode::OK);
    let volunteer: Value = test::read_body_json(volunteer).await;
    assert_eq!(volunteer["items"][0]["raw_record_reference"], Value::Null);
}
