use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::json;

use angui::{
    ai_gateway::AiGateway,
    entities::case_attachments,
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
            .set_payload(upload)
            .to_request(),
    )
    .await;
    assert_eq!(uploaded.status(), StatusCode::CREATED);
    let photo: serde_json::Value = test::read_body_json(uploaded).await;
    assert_eq!(photo["content_type"], "image/png");
    assert!(photo.get("storage_key").is_none());

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
        },
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
