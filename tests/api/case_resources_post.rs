use actix_web::{
    http::{StatusCode, header},
    test,
};
use angui::{
    entities::case_places,
    models::{CreateCasePlaceRequest, PlaceVisibility},
    services::case_resource_service,
};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set};
use serde_json::json;

use crate::support::{COMMANDER, FAMILY, LEARNER, TestContext, VOLUNTEER, assert_error};

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
async fn get_case_places_applies_role_visibility_and_hides_non_members() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let place_types = context.app_state().case_place_types;
    let family = context.authenticated(FAMILY).await;
    let commander = context.authenticated(COMMANDER).await;
    let own_draft = case_resource_service::create_place(
        &context.database,
        &family,
        &case_id,
        CreateCasePlaceRequest {
            name: "Family private draft".to_owned(),
            place_type: "frequent".to_owned(),
            address: "Fictional family address".to_owned(),
            longitude: None,
            latitude: None,
            visibility: PlaceVisibility::Internal,
        },
        &place_types,
    )
    .await
    .expect("fixture place should be created");
    let public_confirmed = case_resource_service::create_place(
        &context.database,
        &commander,
        &case_id,
        CreateCasePlaceRequest {
            name: "Confirmed public meeting point".to_owned(),
            place_type: "key_location".to_owned(),
            address: "Fictional public square".to_owned(),
            longitude: Some(117.2272),
            latitude: Some(31.8206),
            visibility: PlaceVisibility::Public,
        },
        &place_types,
    )
    .await
    .expect("fixture place should be created");
    let confirmed_visible = case_resource_service::create_place(
        &context.database,
        &commander,
        &case_id,
        CreateCasePlaceRequest {
            name: "Confirmed non-public meeting point".to_owned(),
            place_type: "key_location".to_owned(),
            address: "Fictional confirmed square".to_owned(),
            longitude: None,
            latitude: None,
            visibility: PlaceVisibility::Confirmed,
        },
        &place_types,
    )
    .await
    .expect("fixture place should be created");
    let unreviewed_public = case_resource_service::create_place(
        &context.database,
        &commander,
        &case_id,
        CreateCasePlaceRequest {
            name: "Unreviewed public report".to_owned(),
            place_type: "other".to_owned(),
            address: "Fictional unreviewed address".to_owned(),
            longitude: None,
            latitude: None,
            visibility: PlaceVisibility::Public,
        },
        &place_types,
    )
    .await
    .expect("fixture place should be created");
    let internal_confirmed = case_resource_service::create_place(
        &context.database,
        &commander,
        &case_id,
        CreateCasePlaceRequest {
            name: "Internal search direction".to_owned(),
            place_type: "other".to_owned(),
            address: "Fictional internal address".to_owned(),
            longitude: None,
            latitude: None,
            visibility: PlaceVisibility::Internal,
        },
        &place_types,
    )
    .await
    .expect("fixture place should be created");
    mark_place_confirmed(&context, &public_confirmed.id).await;
    mark_place_confirmed(&context, &confirmed_visible.id).await;
    mark_place_confirmed(&context, &internal_confirmed.id).await;

    let app = crate::init_api_app!(&context);
    let request_for = |token: String| {
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/places"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request()
    };

    let family_places: Vec<serde_json::Value> = test::read_body_json(
        test::call_service(&app, request_for(context.token(FAMILY).await)).await,
    )
    .await;
    assert!(
        family_places
            .iter()
            .any(|place| place["id"] == own_draft.id)
    );
    assert!(
        family_places
            .iter()
            .any(|place| place["id"] == public_confirmed.id)
    );
    assert!(
        family_places
            .iter()
            .any(|place| place["id"] == confirmed_visible.id)
    );
    assert!(
        !family_places
            .iter()
            .any(|place| place["id"] == unreviewed_public.id)
    );
    assert!(
        !family_places
            .iter()
            .any(|place| place["id"] == internal_confirmed.id)
    );

    let volunteer_places: Vec<serde_json::Value> = test::read_body_json(
        test::call_service(&app, request_for(context.token(VOLUNTEER).await)).await,
    )
    .await;
    assert_eq!(volunteer_places.len(), 1);
    assert_eq!(volunteer_places[0]["id"], public_confirmed.id);

    let commander_places: Vec<serde_json::Value> = test::read_body_json(
        test::call_service(&app, request_for(context.token(COMMANDER).await)).await,
    )
    .await;
    assert_eq!(commander_places.len(), 5);

    let hidden = test::call_service(&app, request_for(context.token(LEARNER).await)).await;
    assert_error(hidden, StatusCode::NOT_FOUND, "not_found").await;
}

async fn mark_place_confirmed(context: &TestContext, place_id: &str) {
    let place = case_places::Entity::find_by_id(place_id)
        .one(&context.database)
        .await
        .expect("fixture place should load")
        .expect("fixture place should exist");
    let mut place = place.into_active_model();
    place.review_status = Set("confirmed".to_owned());
    place
        .update(&context.database)
        .await
        .expect("fixture place should update");
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

    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let attachment_id = attachment_id.to_owned();
    let linked_clue = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({
                "source": "family",
                "content": "Fictional photo submitted for review",
                "attachment_ids": [attachment_id.clone()]
            }))
            .to_request(),
    )
    .await;
    assert_eq!(linked_clue.status(), StatusCode::CREATED);
    let linked_clue: serde_json::Value = test::read_body_json(linked_clue).await;
    let clue_id = linked_clue["id"].as_str().expect("clue id");

    let commander_token = context.token(COMMANDER).await;
    let confirmed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/clues/{clue_id}/review"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(
                json!({ "status": "confirmed", "reason": "image matched the submitted report" }),
            )
            .to_request(),
    )
    .await;
    assert_eq!(confirmed.status(), StatusCode::OK);

    let commander_timeline = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    let commander_timeline: serde_json::Value = test::read_body_json(commander_timeline).await;
    assert_eq!(
        commander_timeline["items"][0]["attachment_ids"],
        json!([attachment_id.clone()])
    );

    let volunteer_token = context.token(VOLUNTEER).await;
    let volunteer_timeline = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    let volunteer_timeline: serde_json::Value = test::read_body_json(volunteer_timeline).await;
    assert_eq!(volunteer_timeline["items"][0]["attachment_ids"], json!([]));

    let volunteer_download = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/attachments/{attachment_id}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    assert_error(volunteer_download, StatusCode::FORBIDDEN, "forbidden").await;
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
