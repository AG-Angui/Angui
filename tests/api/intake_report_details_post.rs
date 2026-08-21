use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::json;

use angui::{
    ai_gateway::AiGateway,
    entities::{case_attachments, intake_session_photos},
    models::{
        AcknowledgeIntakeAiInitialReviewRequest, ConfirmIntakeSessionRequest,
        ConfirmedIntakeProfile, StartIntakeAiInitialReviewRequest,
    },
    services::intake_session_service,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::support::{COMMANDER, FAMILY, TestContext, assert_error};

const PNG: [u8; 68] = [
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4, 0,
    0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5, 1, 1,
    39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[actix_web::test]
async fn intake_report_details_require_explicit_fields_and_keep_photos_owner_scoped() {
    let context = TestContext::new().await;
    context.enable_intake_report_details().await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/intake-sessions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({ "initial_answers": {
                "basic_information": "姓名：虚构老人；身高：168 厘米；特征描述：戴帽子",
                "last_seen": "虚构社区南门",
                "suspicious_motive": "外出后未按时回家",
                "police_report_status": "未报警",
                "family_phone": "+86 138-0000-0000"
            }}))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(body["question_set_version"], 3);
    assert_eq!(body["status"], "ready_for_confirmation");
    let session_id = body["id"].as_str().expect("session id");

    let boundary = "intake-photo-boundary";
    let mut upload = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"portrait.png\"\r\nContent-Type: image/png\r\n\r\n").into_bytes();
    upload.extend_from_slice(&PNG);
    upload.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let uploaded = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/intake-sessions/{session_id}/photos"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .insert_header((
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            ))
            .set_payload(upload.clone())
            .to_request(),
    )
    .await;
    assert_eq!(uploaded.status(), StatusCode::CREATED);
    let photo: serde_json::Value = test::read_body_json(uploaded).await;
    assert_eq!(photo["content_type"], "image/png");
    assert!(photo.get("storage_key").is_none());
    let photo_id = photo["id"].as_str().expect("photo id");
    let stored_photo = intake_session_photos::Entity::find_by_id(photo_id)
        .one(&context.database)
        .await
        .expect("intake photo query should succeed")
        .expect("uploaded photo should be persisted");
    let expected_photo_bytes = std::fs::read(
        context
            .app_state()
            .attachment_storage_directory
            .join(stored_photo.storage_key),
    )
    .expect("controlled photo should be written to storage");

    let downloaded = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/intake-sessions/{session_id}/photos/{photo_id}"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(downloaded.status(), StatusCode::OK);
    assert_eq!(
        downloaded
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("image/png")
    );
    assert_eq!(
        downloaded
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some("no-store, private")
    );
    assert_eq!(
        downloaded
            .headers()
            .get(header::X_CONTENT_TYPE_OPTIONS)
            .and_then(|value| value.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        test::read_body(downloaded).await.as_ref(),
        expected_photo_bytes.as_slice()
    );

    for (mime, filename) in [("image/x-png", "legacy.png"), ("", "blank-mime.png")] {
        let compatible_boundary = format!("intake-photo-{filename}");
        let mut compatible_upload = format!(
            "--{compatible_boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {mime}\r\n\r\n"
        )
        .into_bytes();
        compatible_upload.extend_from_slice(&PNG);
        compatible_upload
            .extend_from_slice(format!("\r\n--{compatible_boundary}--\r\n").as_bytes());
        let uploaded = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/intake-sessions/{session_id}/photos"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
                .insert_header((
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={compatible_boundary}"),
                ))
                .set_payload(compatible_upload)
                .to_request(),
        )
        .await;
        assert_eq!(uploaded.status(), StatusCode::CREATED);
    }

    for _ in 0..1 {
        let uploaded = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/intake-sessions/{session_id}/photos"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
                .insert_header((
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                ))
                .set_payload(upload.clone())
                .to_request(),
        )
        .await;
        assert_eq!(uploaded.status(), StatusCode::CREATED);
    }
    let rejected_upload = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/intake-sessions/{session_id}/photos"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .insert_header((
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            ))
            .set_payload(upload)
            .to_request(),
    )
    .await;
    assert_error(rejected_upload, StatusCode::BAD_REQUEST, "validation_error").await;

    let empty_photo_session = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/intake-sessions")
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({ "initial_answers": {
                "basic_information": "姓名：测试人员；身高：168 厘米；特征描述：戴帽子",
                "last_seen": "测试小区南门",
                "suspicious_motive": "外出后未按时回家",
                "police_report_status": "未报警",
                "family_phone": "+86 138-0000-0000"
            }}))
            .to_request(),
    )
    .await;
    assert_eq!(empty_photo_session.status(), StatusCode::CREATED);
    let empty_photo_session: serde_json::Value = test::read_body_json(empty_photo_session).await;
    let empty_photo_session_id = empty_photo_session["id"]
        .as_str()
        .expect("empty-photo session id");

    let wrong_session = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/intake-sessions/{empty_photo_session_id}/photos/{photo_id}"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    assert_error(wrong_session, StatusCode::NOT_FOUND, "not_found").await;

    let commander_token = context.token(COMMANDER).await;
    let hidden = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/intake-sessions/{session_id}/photos"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_error(hidden, StatusCode::NOT_FOUND, "not_found").await;

    let family = context.authenticated(FAMILY).await;
    let gateway = AiGateway::from_configurations(Vec::new()).expect("empty AI gateway");
    let empty_photo_profile = ConfirmedIntakeProfile {
        display_name: "测试人员".to_owned(),
        age: None,
        gender: None,
        physical_description: Some("戴帽子".to_owned()),
        clothing_description: None,
        health_notes: None,
        last_seen_at: None,
        last_seen_location: "测试小区南门".to_owned(),
        mobility_notes: None,
        transportation_ability: None,
        frequent_locations: None,
        behavior_habits: None,
        suspicious_motive: None,
    };
    let empty_photo_review = intake_session_service::start_ai_initial_review(
        &context.database,
        &family,
        empty_photo_session_id,
        StartIntakeAiInitialReviewRequest {
            profile: empty_photo_profile.clone(),
        },
        &gateway,
    )
    .await
    .expect("quality review should degrade safely for a session without photos");
    intake_session_service::acknowledge_ai_initial_review(
        &context.database,
        &family,
        empty_photo_session_id,
        AcknowledgeIntakeAiInitialReviewRequest {
            human_confirmed: true,
            confirmed_issue_ids: empty_photo_review
                .issues
                .iter()
                .map(|issue| issue.id.clone())
                .collect(),
            issue_responses: vec![],
        },
        &gateway,
    )
    .await
    .expect("family can acknowledge the review for a session without photos");
    let empty_photo_confirmation = intake_session_service::confirm_intake_session(
        &context.database,
        &family,
        empty_photo_session_id,
        ConfirmIntakeSessionRequest {
            human_confirmed: true,
            profile: empty_photo_profile,
        },
    )
    .await
    .expect_err("confirmation must require at least one controlled photo");
    assert!(matches!(
        empty_photo_confirmation,
        angui::error::ApiError::Conflict(message)
            if message == "upload at least one missing-person photo before confirming the case"
    ));

    let review = intake_session_service::start_ai_initial_review(
        &context.database,
        &family,
        session_id,
        StartIntakeAiInitialReviewRequest {
            profile: ConfirmedIntakeProfile {
                display_name: "虚构老人".to_owned(),
                age: None,
                gender: None,
                physical_description: Some("戴帽子".to_owned()),
                clothing_description: None,
                health_notes: None,
                last_seen_at: None,
                last_seen_location: "虚构社区南门".to_owned(),
                mobility_notes: None,
                transportation_ability: None,
                frequent_locations: None,
                behavior_habits: None,
                suspicious_motive: None,
            },
        },
        &gateway,
    )
    .await
    .expect("quality review should degrade safely");
    let acknowledged = intake_session_service::acknowledge_ai_initial_review(
        &context.database,
        &family,
        session_id,
        AcknowledgeIntakeAiInitialReviewRequest {
            human_confirmed: true,
            confirmed_issue_ids: review.issues.iter().map(|issue| issue.id.clone()).collect(),
            issue_responses: vec![],
        },
        &gateway,
    )
    .await
    .expect("family can acknowledge the corrected review prompts");
    assert!(acknowledged.ready_for_second_confirmation);

    let confirmed = intake_session_service::confirm_intake_session(
        &context.database,
        &family,
        session_id,
        ConfirmIntakeSessionRequest {
            human_confirmed: true,
            profile: ConfirmedIntakeProfile {
                display_name: "虚构老人".to_owned(),
                age: None,
                gender: None,
                physical_description: Some("戴帽子".to_owned()),
                clothing_description: None,
                health_notes: None,
                last_seen_at: None,
                last_seen_location: "虚构社区南门".to_owned(),
                mobility_notes: None,
                transportation_ability: None,
                frequent_locations: None,
                behavior_habits: None,
                suspicious_motive: None,
            },
        },
    )
    .await
    .expect("controlled photo should allow confirmed case creation");
    assert!(
        case_attachments::Entity::find()
            .filter(case_attachments::Column::CaseId.eq(&confirmed.case_id))
            .one(&context.database)
            .await
            .expect("case attachment query should succeed")
            .is_some()
    );
}
