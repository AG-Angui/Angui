use actix_web::{
    http::{StatusCode, header},
    test,
};
use angui::entities::{audit_events, clue_attachment_links, clue_attributions, clues};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set};
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
    assert!(body.get("case_place_types").is_none());

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
async fn retired_place_submission_uses_pending_location_clues() {
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
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "legacy place write route is retired"
    );

    let created = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({
                "source": "family", "content": "Fictional park: north gate",
                "location_text": "Fictional park north gate", "location_kind": "point",
                "longitude": 117.2272, "latitude": 31.8206,
                "location_precision": "exact", "visibility": "confirmed",
                "confidence": "unverified"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(created).await;
    assert_eq!(body["status"], "pending_review");
    assert_eq!(body["location_kind"], "point");

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
    assert_eq!(denied.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn location_clue_review_requires_commander_and_records_audited_transition() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let family_token = context.token(FAMILY).await;
    let commander_token = context.token(COMMANDER).await;
    let app = crate::init_api_app!(&context);

    let created = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({
                "source": "family", "content": "Fictional clinic: entrance",
                "location_text": "Fictional clinic entrance",
                "location_kind": "point", "location_precision": "approximate",
                "visibility": "confirmed", "confidence": "unverified"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: serde_json::Value = test::read_body_json(created).await;
    let clue_id = created["id"].as_str().expect("clue id");
    let review_uri = format!("/api/clues/{clue_id}/review");

    let family_denied = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&review_uri)
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .set_json(json!({
                "status": "confirmed",
                "reason": "family cannot review its own submission"
            }))
            .to_request(),
    )
    .await;
    assert_error(family_denied, StatusCode::FORBIDDEN, "forbidden").await;

    let invalid = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&review_uri)
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({ "status": "pending_review", "reason": "invalid transition" }))
            .to_request(),
    )
    .await;
    assert_error(invalid, StatusCode::BAD_REQUEST, "validation_error").await;

    let confirmed = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&review_uri)
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({
                "status": "confirmed",
                "reason": "verified against the fictional family report"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(confirmed.status(), StatusCode::OK);
    let confirmed: serde_json::Value = test::read_body_json(confirmed).await;
    assert_eq!(confirmed["status"], "confirmed");

    let audit = audit_events::Entity::find()
        .filter(audit_events::Column::CaseId.eq(&case_id))
        .filter(audit_events::Column::Action.eq("clue.reviewed"))
        .filter(audit_events::Column::EntityId.eq(clue_id))
        .one(&context.database)
        .await
        .expect("audit query should succeed")
        .expect("clue review audit should exist");
    let metadata: serde_json::Value = serde_json::from_str(
        audit
            .metadata_json
            .as_deref()
            .expect("clue review audit should have metadata"),
    )
    .expect("clue review metadata should be JSON");
    assert_eq!(metadata["from"], "pending_review");
    assert_eq!(metadata["to"], "confirmed");
    assert_eq!(
        metadata["reason"],
        "verified against the fictional family report"
    );

    let repeated = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&review_uri)
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .set_json(json!({
                "status": "rejected",
                "reason": "a completed review cannot be overwritten"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(repeated.status(), StatusCode::OK);
}

#[actix_web::test]
async fn location_clues_follow_role_visibility_and_do_not_expose_legacy_places() {
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
    let app = crate::init_api_app!(&context);
    let mut ids = Vec::new();
    for (content, visibility, token, confirmed) in [
        ("Family private draft", "internal", &family_token, false),
        ("Public meeting point", "public", &commander_token, true),
        ("Confirmed meeting point", "confirmed", &family_token, true),
        (
            "Unreviewed public report",
            "public",
            &commander_token,
            false,
        ),
        (
            "Internal search direction",
            "internal",
            &commander_token,
            true,
        ),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/cases/{case_id}/clues"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .set_json(
                    json!({"source": "manual", "content": content, "location_text": content,
                "location_kind": "point", "longitude": 117.2272, "latitude": 31.8206,
                "visibility": visibility}),
                )
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let body: serde_json::Value = test::read_body_json(response).await;
        let id = body["id"].as_str().expect("clue id").to_owned();
        if content == "Family private draft" {
            clue_attributions::Entity::delete_by_id(&id)
                .exec(&context.database)
                .await
                .expect("remove attribution to simulate migrated record");
        }
        if confirmed {
            let clue = clues::Entity::find_by_id(&id)
                .one(&context.database)
                .await
                .expect("clue query")
                .expect("clue");
            let mut clue = clue.into_active_model();
            clue.status = Set("confirmed".to_owned());
            clue.update(&context.database).await.expect("clue update");
        }
        ids.push(id);
    }
    for (token, expected, has_initial_profile_clue, is_family) in [
        (family_token, vec![0, 1, 2], true, true),
        (context.token(VOLUNTEER).await, vec![1, 2], false, false),
        (commander_token, vec![0, 1, 2, 3, 4], true, false),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/cases/{case_id}"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(response).await;
        assert!(body.get("places").is_none());
        let clues = body["clues"].as_array().expect("visible clues");
        let initial_profile_clues = usize::from(has_initial_profile_clue);
        assert_eq!(clues.len(), expected.len() + initial_profile_clues);
        for index in expected {
            assert!(clues.iter().any(|clue| clue["id"] == ids[index]));
        }
        if is_family {
            assert!(
                clues
                    .iter()
                    .any(|clue| { clue["id"] == ids[0] && clue["is_own_submission"] == true })
            );
        }
    }
    let hidden = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .to_request(),
    )
    .await;
    assert_error(hidden, StatusCode::NOT_FOUND, "not_found").await;
}

#[actix_web::test]
async fn volunteer_can_search_from_a_confirmed_location_clue_with_coordinates() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let app = crate::init_api_app!(&context);
    let created = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/cases/{case_id}/clues"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(FAMILY).await),
            ))
            .set_json(
                json!({"source": "family", "content": "Confirmed search center",
            "location_text": "Fictional confirmed square", "location_kind": "point",
            "longitude": 117.2272, "latitude": 31.8206, "visibility": "confirmed"}),
            )
            .to_request(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(created).await;
    let clue = clues::Entity::find_by_id(body["id"].as_str().expect("clue id"))
        .one(&context.database)
        .await
        .expect("clue query")
        .expect("clue");
    let mut clue = clue.into_active_model();
    clue.status = Set("confirmed".to_owned());
    clue.update(&context.database).await.expect("clue update");
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/cases/{case_id}/pois?category=hospital"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(VOLUNTEER).await),
            ))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(body["center_source"], "authorized_case_location");
    assert_eq!(body["degradation_status"], "degraded");
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

#[actix_web::test]
async fn post_clue_attachments_links_evidence_atomically_and_enforces_submitter_scope() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let family_clue_id = context.create_clue(&case_id, FAMILY).await;
    let volunteer_clue_id = context.create_clue(&case_id, VOLUNTEER).await;
    let app = crate::init_api_app!(&context);
    let png: [u8; 68] = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5,
        1, 1, 39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let upload = |clue_id: &str, token: &str, boundary: &str| {
        let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"fictional-evidence.png\"\r\nContent-Type: image/png\r\n\r\n").into_bytes();
        body.extend_from_slice(&png);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        test::TestRequest::post()
            .uri(&format!("/api/clues/{clue_id}/attachments"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .insert_header((
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            ))
            .set_payload(body)
            .to_request()
    };

    let family_token = context.token(FAMILY).await;
    let uploaded = test::call_service(
        &app,
        upload(&family_clue_id, &family_token, "clue-attachment-family"),
    )
    .await;
    assert_eq!(uploaded.status(), StatusCode::CREATED);
    let uploaded: serde_json::Value = test::read_body_json(uploaded).await;
    let attachment_id = uploaded["id"].as_str().expect("attachment id");
    assert_eq!(uploaded["review_status"], "pending_review");
    assert!(
        clue_attachment_links::Entity::find()
            .filter(clue_attachment_links::Column::ClueId.eq(&family_clue_id))
            .filter(clue_attachment_links::Column::AttachmentId.eq(attachment_id))
            .one(&context.database)
            .await
            .expect("attachment link query should succeed")
            .is_some()
    );
    assert!(
        audit_events::Entity::find()
            .filter(audit_events::Column::EntityId.eq(attachment_id))
            .filter(audit_events::Column::Action.eq("clue.attachment_submitted"))
            .one(&context.database)
            .await
            .expect("attachment audit query should succeed")
            .is_some()
    );

    let volunteer_token = context.token(VOLUNTEER).await;
    let hidden = test::call_service(
        &app,
        upload(&family_clue_id, &volunteer_token, "clue-attachment-hidden"),
    )
    .await;
    assert_error(hidden, StatusCode::NOT_FOUND, "not_found").await;

    let commander_token = context.token(COMMANDER).await;
    let commander_upload = test::call_service(
        &app,
        upload(
            &volunteer_clue_id,
            &commander_token,
            "clue-attachment-commander",
        ),
    )
    .await;
    assert_eq!(commander_upload.status(), StatusCode::CREATED);
    let commander_attachment: serde_json::Value = test::read_body_json(commander_upload).await;
    let commander_attachment_id = commander_attachment["id"]
        .as_str()
        .expect("commander attachment id");
    let family_cannot_download_internal_evidence = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/cases/{case_id}/attachments/{commander_attachment_id}"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    assert_error(
        family_cannot_download_internal_evidence,
        StatusCode::FORBIDDEN,
        "forbidden",
    )
    .await;

    context.close_case(&case_id).await;
    let closed = test::call_service(
        &app,
        upload(&family_clue_id, &family_token, "clue-attachment-closed"),
    )
    .await;
    assert_error(closed, StatusCode::CONFLICT, "conflict").await;
}
