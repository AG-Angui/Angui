use actix_web::{http::StatusCode, test};
use serde_json::json;

use crate::support::{COMMANDER, FAMILY, TestContext};

#[actix_web::test]
async fn update_elder_profile_supports_null_to_value_updates() {
    let context = TestContext::new().await;
    let family_token = context.token(FAMILY).await;
    let commander_token = context.token(COMMANDER).await;

    // Create a case with some null fields via API
    let app = crate::init_api_app!(&context);

    let create_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/cases")
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                format!("Bearer {family_token}"),
            ))
            .set_json(json!({
                "display_name": "测试老人",
                "age": 76,
                "gender": null, // null field
                "physical_description": "测试体貌",
                "clothing_description": null, // null field
                "health_notes": null, // null field
                "last_seen_at": "2026-07-13T09:00:00Z",
                "last_seen_location": "测试公园北门"
            }))
            .to_request(),
    )
    .await;

    assert_eq!(create_response.status(), StatusCode::CREATED);
    let create_body: serde_json::Value = test::read_body_json(create_response).await;
    let case_id = create_body["id"].as_str().expect("case id");

    // Add commander as member so they can access the case
    context
        .add_member(case_id, FAMILY, COMMANDER, "commander")
        .await;

    // Get initial case detail to verify null fields
    let get_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{}", case_id))
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                format!("Bearer {commander_token}"),
            ))
            .to_request(),
    )
    .await;

    assert_eq!(get_response.status(), StatusCode::OK);
    let get_body: serde_json::Value = test::read_body_json(get_response).await;
    assert!(get_body["elder_profile"]["gender"].is_null());
    assert!(get_body["elder_profile"]["clothing_description"].is_null());
    assert!(get_body["elder_profile"]["health_notes"].is_null());

    // Test: update null fields to values
    let update_response = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{}/elder-profile", case_id))
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                format!("Bearer {commander_token}"),
            ))
            .set_json(json!({
                "gender": "female",
                "clothing_description": "新增的衣着描述",
                "health_notes": "新增的健康备注"
            }))
            .to_request(),
    )
    .await;

    assert_eq!(
        update_response.status(),
        StatusCode::OK,
        "Should successfully update null fields to values"
    );

    // Verify the updates persisted
    let verify_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{}", case_id))
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                format!("Bearer {commander_token}"),
            ))
            .to_request(),
    )
    .await;

    assert_eq!(verify_response.status(), StatusCode::OK);
    let verify_body: serde_json::Value = test::read_body_json(verify_response).await;

    assert_eq!(verify_body["elder_profile"]["gender"], "female");
    assert_eq!(
        verify_body["elder_profile"]["clothing_description"],
        "新增的衣着描述"
    );
    assert_eq!(
        verify_body["elder_profile"]["health_notes"],
        "新增的健康备注"
    );
}

#[actix_web::test]
async fn update_elder_profile_supports_extended_field_null_updates() {
    let context = TestContext::new().await;
    let family_token = context.token(FAMILY).await;
    let commander_token = context.token(COMMANDER).await;

    let app = crate::init_api_app!(&context);

    // Create case with null extended fields
    let create_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/cases")
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                format!("Bearer {family_token}"),
            ))
            .set_json(json!({
                "display_name": "测试扩展字段",
                "age": 80,
                "gender": "male",
                "physical_description": "测试体貌",
                "clothing_description": "测试衣着",
                "health_notes": "测试健康",
                "last_seen_at": "2026-07-13T09:00:00Z",
                "last_seen_location": "测试地点",
                "mobility_notes": null, // null extended field
                "transportation_ability": null // null extended field
            }))
            .to_request(),
    )
    .await;

    assert_eq!(create_response.status(), StatusCode::CREATED);
    let create_body: serde_json::Value = test::read_body_json(create_response).await;
    let case_id = create_body["id"].as_str().expect("case id");

    // Add commander as member
    context
        .add_member(case_id, FAMILY, COMMANDER, "commander")
        .await;

    // Update null extended fields
    let update_response = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/cases/{}/elder-profile", case_id))
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                format!("Bearer {commander_token}"),
            ))
            .set_json(json!({
                "mobility_notes": json!({
                    "summary": "行动较慢，需要拐杖",
                    "source_fields": ["health_status"],
                    "confidence": "high"
                }).to_string(),
                "transportation_ability": json!({
                    "summary": "不会使用公共交通",
                    "source_fields": ["behavior_habits"],
                    "confidence": "medium"
                }).to_string()
            }))
            .to_request(),
    )
    .await;

    assert_eq!(
        update_response.status(),
        StatusCode::OK,
        "Should successfully update null extended fields"
    );

    // Verify extended fields were updated
    let verify_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{}", case_id))
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                format!("Bearer {commander_token}"),
            ))
            .to_request(),
    )
    .await;

    assert_eq!(verify_response.status(), StatusCode::OK);
    let verify_body: serde_json::Value = test::read_body_json(verify_response).await;

    let mobility = &verify_body["elder_profile"]["mobility_notes"];
    assert!(!mobility.is_null());
    assert_eq!(mobility["summary"], "行动较慢，需要拐杖");
    assert_eq!(mobility["confidence"], "high");

    let transportation = &verify_body["elder_profile"]["transportation_ability"];
    assert!(!transportation.is_null());
    assert_eq!(transportation["summary"], "不会使用公共交通");
    assert_eq!(transportation["confidence"], "medium");
}
