use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::json;

use crate::support::{COMMANDER, FAMILY, TestContext, VOLUNTEER, assert_error};

#[actix_web::test]
async fn resource_configuration_is_available_only_to_case_members() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/resource-configuration"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(body["attachment_max_image_bytes"], 5 * 1024 * 1024);
    assert_eq!(body["attachment_max_per_case"], 12);
    assert_eq!(body["case_place_types"][0], "frequent");

    let volunteer_token = context.token(VOLUNTEER).await;
    let hidden = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/resource-configuration"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    assert_error(hidden, StatusCode::NOT_FOUND, "not_found").await;
}

#[actix_web::test]
async fn post_case_places_requires_family_or_commander_and_returns_pending_review() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(&app, test::TestRequest::post()
        .uri(&format!("/api/cases/{case_id}/places"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
        .set_json(json!({
            "name": "Fictional park", "place_type": "frequent", "address": "Fictional park north gate",
            "longitude": 117.2272, "latitude": 31.8206, "visibility": "confirmed"
        })).to_request()).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(body["review_status"], "pending_review");
    assert_eq!(body["is_own_submission"], true);
    assert_eq!(body["source"], "family");

    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let denied = test::call_service(&app, test::TestRequest::post()
        .uri(&format!("/api/cases/{case_id}/places"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
        .set_json(json!({ "name": "Home", "place_type": "other", "address": "Private", "visibility": "internal" })).to_request()).await;
    assert_error(denied, StatusCode::FORBIDDEN, "forbidden").await;
}

#[actix_web::test]
async fn post_case_places_uses_the_configured_place_type_allowlist() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family_token = context.token(FAMILY).await;
    let mut state = context.app_state();
    state.case_place_types = vec!["station".to_owned()];
    let app = test::init_service(
        actix_web::App::new()
            .app_data(actix_web::web::Data::new(state))
            .configure(angui::routes::configure),
    )
    .await;
    let station = |place_type: &str| {
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/places"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({
                "name": "Fictional station", "place_type": place_type, "address": "Fictional station",
                "visibility": "confirmed"
            }))
            .to_request()
    };

    let disallowed = test::call_service(&app, station("frequent")).await;
    assert_error(disallowed, StatusCode::BAD_REQUEST, "validation_error").await;
    let allowed = test::call_service(&app, station("station")).await;
    assert_eq!(allowed.status(), StatusCode::CREATED);
}

#[actix_web::test]
async fn post_case_attachments_normalizes_images_and_protects_downloads() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let boundary = "angui-test-boundary";
    let png: [u8; 68] = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5,
        1, 1, 39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"photo.png\"\r\nContent-Type: image/png; charset=binary\r\n\r\n").into_bytes();
    body.extend_from_slice(&png);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/attachments"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .insert_header((
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            ))
            .set_payload(body)
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let attachment: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(attachment["content_type"], "image/png");
    assert_eq!(attachment["review_status"], "pending_review");
    let attachment_id = attachment["id"].as_str().expect("attachment id");

    let downloaded = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/attachments/{attachment_id}"))
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
            .get(header::X_CONTENT_TYPE_OPTIONS)
            .and_then(|value| value.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        downloaded
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some("no-store, private")
    );
}

#[actix_web::test]
async fn post_case_attachments_rejects_mismatched_or_non_image_content() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let family_token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let png: [u8; 68] = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5,
        1, 1, 39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    for (index, (content_type, content)) in [
        ("image/jpeg", png.as_slice()),
        ("image/png", b"not an image".as_slice()),
    ]
    .into_iter()
    .enumerate()
    {
        let boundary = format!("angui-test-boundary-{index}");
        let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"photo.png\"\r\nContent-Type: {content_type}\r\n\r\n").into_bytes();
        body.extend_from_slice(content);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/cases/{case_id}/attachments"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
                .insert_header((
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                ))
                .set_payload(body)
                .to_request(),
        )
        .await;
        assert_error(response, StatusCode::BAD_REQUEST, "validation_error").await;
    }
}
