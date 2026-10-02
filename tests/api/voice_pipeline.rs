use actix_web::{
    http::{StatusCode, header},
    test,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;
use uuid::Uuid;

use crate::support::{COMMANDER, FAMILY, TestContext, VOLUNTEER};
use angui::{
    entities::{clue_drafts, space_events, voice_clue_candidates, voice_reports},
    models::{
        ClueDraftCandidate, CreateCollaborationSpaceRequest, CreateSpaceMessageRequest,
        JoinCollaborationSpaceRequest, ReviewClueDraftRequest,
    },
    services::{
        case_collaboration_service, collaboration_space_service, voice_review_service,
        voice_room_service,
    },
};

#[actix_web::test]
async fn voice_tickets_isolate_browser_sessions_and_old_floor_release() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let commander = context.authenticated(COMMANDER).await;
    let space = collaboration_space_service::create_space(
        &context.database,
        &commander,
        &case_id,
        CreateCollaborationSpaceRequest {
            name: "Voice sessions".to_owned(),
        },
    )
    .await
    .expect("commander can create a space");
    let mut state = context.app_state();
    state.livekit_url = Some("wss://livekit.example.invalid".to_owned());
    state.livekit_api_key = Some("test-key".to_owned());
    state.livekit_api_secret = Some("a-long-private-test-secret-0123456789".to_owned());

    let first = voice_room_service::room_ticket(&state, &commander, &space.id)
        .await
        .expect("first browser can request a ticket");
    let second = voice_room_service::room_ticket(&state, &commander, &space.id)
        .await
        .expect("second browser can request a ticket");
    assert_ne!(first.participant_identity, second.participant_identity);
    assert_ne!(first.token, second.token);
    assert_ne!(first.participant_identity, commander.id);

    state.voice_floors.lock().unwrap().insert(
        space.id.clone(),
        (
            second.participant_identity.clone(),
            std::time::Instant::now(),
        ),
    );
    voice_room_service::set_floor(
        &state,
        &commander,
        &space.id,
        &first.participant_identity,
        false,
    )
    .await
    .expect("old browser release is idempotent");
    assert_eq!(
        state.voice_floors.lock().unwrap().get(&space.id).unwrap().0,
        second.participant_identity,
    );

    let mut other_login = commander.clone();
    other_login.session_id = Uuid::new_v4().to_string();
    assert!(
        voice_room_service::set_floor(
            &state,
            &other_login,
            &space.id,
            &second.participant_identity,
            false,
        )
        .await
        .is_err(),
        "a different login cannot use this browser's identity"
    );
    voice_room_service::leave_room(&state, &commander, &space.id, &first.participant_identity)
        .await
        .expect("owner can close its browser session");
    assert!(
        !state
            .voice_sessions
            .lock()
            .unwrap()
            .contains_key(&first.participant_identity)
    );
    assert!(
        state
            .voice_sessions
            .lock()
            .unwrap()
            .contains_key(&second.participant_identity)
    );
}

#[actix_web::test]
async fn voice_room_and_socket_tickets_require_membership_and_hide_session_tokens() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let commander = context.authenticated(COMMANDER).await;
    let space = collaboration_space_service::create_space(
        &context.database,
        &commander,
        &case_id,
        CreateCollaborationSpaceRequest {
            name: "Voice test".to_owned(),
        },
    )
    .await
    .expect("commander can create a space");
    let app = crate::init_api_app!(&context);
    let commander_token = context.token(COMMANDER).await;
    let volunteer_token = context.token(VOLUNTEER).await;
    let family_token = context.token(FAMILY).await;

    let room_uri = format!("/api/collaboration-spaces/{}/voice-room/ticket", space.id);
    let commander_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&room_uri)
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(
        commander_response.status(),
        StatusCode::CONFLICT,
        "unconfigured LiveKit is explicit"
    );
    let volunteer_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&room_uri)
            .insert_header((header::AUTHORIZATION, format!("Bearer {volunteer_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(
        volunteer_response.status(),
        StatusCode::NOT_FOUND,
        "case membership alone cannot enter a room"
    );
    let family_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&room_uri)
            .insert_header((header::AUTHORIZATION, format!("Bearer {family_token}")))
            .to_request(),
    )
    .await;
    assert!(matches!(
        family_response.status(),
        StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
    ));

    let ticket_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!(
                "/api/collaboration-spaces/{}/events/ticket",
                space.id
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {commander_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(ticket_response.status(), StatusCode::OK);
    let ticket: Value = test::read_body_json(ticket_response).await;
    assert_eq!(ticket["expires_in_seconds"], 30);
    assert!(
        ticket["ticket"]
            .as_str()
            .is_some_and(|value| value != commander_token)
    );

    let old_query_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/collaboration-spaces/{}/events/ws?access_token={commander_token}",
                space.id
            ))
            .to_request(),
    )
    .await;
    assert_eq!(old_query_response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn returned_voice_draft_requires_submitter_revision_before_approval() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let commander = context.authenticated(COMMANDER).await;
    let volunteer = context.authenticated(VOLUNTEER).await;
    let family = context.authenticated(FAMILY).await;
    let space = collaboration_space_service::create_space(
        &context.database,
        &commander,
        &case_id,
        CreateCollaborationSpaceRequest {
            name: "Review test".to_owned(),
        },
    )
    .await
    .expect("create space");
    collaboration_space_service::join_space(
        &context.database,
        &volunteer,
        &space.id,
        JoinCollaborationSpaceRequest {
            location_consent: true,
            consent_version: Some("v1".to_owned()),
        },
    )
    .await
    .expect("join space");
    let timestamp = "2026-09-29T00:00:00.000Z".to_owned();
    let report_id = Uuid::new_v4().to_string();
    let draft_id = Uuid::new_v4().to_string();
    let candidate_id = Uuid::new_v4().to_string();
    let original = ClueDraftCandidate {
        content_summary: Some("Original".to_owned()),
        ..Default::default()
    };
    let original_json = serde_json::to_string(&original).expect("candidate JSON");
    voice_reports::ActiveModel {
        id: Set(report_id.clone()),
        space_id: Set(space.id.clone()),
        case_id: Set(case_id.clone()),
        reporter_id: Set(volunteer.id.clone()),
        object_key: Set("voice/test.wav".to_owned()),
        content_type: Set("audio/wav".to_owned()),
        byte_size: Set(44),
        status: Set("draft_ready".to_owned()),
        created_at: Set(timestamp.clone()),
        failed_reason: Set(None),
        audio_deleted_at: Set(None),
    }
    .insert(&context.database)
    .await
    .expect("insert report");
    clue_drafts::ActiveModel {
        id: Set(draft_id.clone()),
        case_id: Set(case_id.clone()),
        status: Set("draft".to_owned()),
        content: Set("Original transcript".to_owned()),
        source_type: Set("field_report".to_owned()),
        raw_record_reference: Set(Some(format!("voice_candidate:{candidate_id}"))),
        source_record_id: Set(None),
        uncertainty_notice: Set("Review required".to_owned()),
        template_version: Set("voice-clue-v1".to_owned()),
        provider_model: Set(Some("fixture".to_owned())),
        degradation_status: Set("manual_review_required".to_owned()),
        candidate_json: Set(original_json.clone()),
        review_status: Set("pending_review".to_owned()),
        reviewed_by_user_id: Set(None),
        reviewed_at: Set(None),
        review_reason: Set(None),
        version: Set(1),
        promoted_clue_id: Set(None),
        created_by_user_id: Set(volunteer.id.clone()),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp.clone()),
    }
    .insert(&context.database)
    .await
    .expect("insert draft");
    voice_clue_candidates::ActiveModel {
        id: Set(candidate_id.clone()),
        case_id: Set(case_id.clone()),
        voice_report_id: Set(Some(report_id)),
        intercom_recording_id: Set(None),
        submitted_by_user_id: Set(volunteer.id.clone()),
        discoverer_user_id: Set(Some(volunteer.id.clone())),
        source_type: Set("voice_report".to_owned()),
        ai_generated: Set(true),
        transcript_text: Set(Some("Original transcript".to_owned())),
        candidate_json: Set(original_json),
        asr_version: Set(Some("fixture".to_owned())),
        model_version: Set(Some("fixture".to_owned())),
        status: Set("pending_review".to_owned()),
        retry_count: Set(0),
        failure_reason: Set(None),
        promoted_clue_id: Set(None),
        clue_draft_id: Set(Some(draft_id.clone())),
        returned_for_revision: Set(false),
        created_at: Set(timestamp.clone()),
        updated_at: Set(timestamp),
    }
    .insert(&context.database)
    .await
    .expect("insert voice candidate");

    voice_review_service::return_candidate(
        &context.database,
        &commander,
        &space.id,
        &candidate_id,
        "Clarify the place",
    )
    .await
    .expect("commander returns draft");
    let candidates = collaboration_space_service::list_voice_candidates(
        &context.database,
        &volunteer,
        &space.id,
    )
    .await
    .expect("submitter sees candidate");
    assert_eq!(candidates[0]["returned_for_revision"], true);
    assert_eq!(candidates[0]["review_note"], "Clarify the place");
    assert_eq!(
        candidates[0]["editable_candidate"]["content_summary"],
        "Original"
    );
    assert!(
        collaboration_space_service::list_voice_candidates(&context.database, &family, &space.id)
            .await
            .is_err()
    );
    let review = ReviewClueDraftRequest {
        action: "accept".to_owned(),
        reason: "Reviewed".to_owned(),
        candidate: original.clone(),
        field_decisions: Default::default(),
    };
    assert!(
        case_collaboration_service::review_clue_draft(
            &context.database,
            &commander,
            &case_id,
            &draft_id,
            review
        )
        .await
        .is_err()
    );
    assert!(
        voice_review_service::resubmit_candidate(
            &context.database,
            &commander,
            &space.id,
            &candidate_id,
            original.clone()
        )
        .await
        .is_err()
    );
    let revised = ClueDraftCandidate {
        content_summary: Some("Revised".to_owned()),
        ..original
    };
    voice_review_service::resubmit_candidate(
        &context.database,
        &volunteer,
        &space.id,
        &candidate_id,
        revised,
    )
    .await
    .expect("submitter resubmits");
    let candidates = collaboration_space_service::list_voice_candidates(
        &context.database,
        &volunteer,
        &space.id,
    )
    .await
    .expect("reload candidate");
    assert_eq!(candidates[0]["returned_for_revision"], false);
    assert!(candidates[0]["editable_candidate"].is_null());
}

#[actix_web::test]
async fn repeated_message_delivery_receipts_are_idempotent() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let commander = context.authenticated(COMMANDER).await;
    let space = collaboration_space_service::create_space(
        &context.database,
        &commander,
        &case_id,
        CreateCollaborationSpaceRequest {
            name: "Receipt test".to_owned(),
        },
    )
    .await
    .expect("create space");
    let message = collaboration_space_service::create_message(
        &context.database,
        &commander,
        &space.id,
        CreateSpaceMessageRequest {
            content: "Check in".to_owned(),
            message_type: None,
        },
    )
    .await
    .expect("create message");

    collaboration_space_service::acknowledge_message(
        &context.database,
        &commander,
        &space.id,
        &message.id,
        "delivered",
    )
    .await
    .expect("first delivery receipt");
    collaboration_space_service::acknowledge_message(
        &context.database,
        &commander,
        &space.id,
        &message.id,
        "delivered",
    )
    .await
    .expect("duplicate delivery receipt");
    let events = space_events::Entity::find()
        .filter(space_events::Column::SpaceId.eq(&space.id))
        .filter(space_events::Column::EventType.eq("message.acknowledged"))
        .all(&context.database)
        .await
        .expect("list receipts");
    assert_eq!(events.len(), 1);

    collaboration_space_service::acknowledge_message(
        &context.database,
        &commander,
        &space.id,
        &message.id,
        "acknowledged",
    )
    .await
    .expect("upgrade receipt");
    let events = space_events::Entity::find()
        .filter(space_events::Column::SpaceId.eq(&space.id))
        .filter(space_events::Column::EventType.eq("message.acknowledged"))
        .all(&context.database)
        .await
        .expect("list upgraded receipts");
    assert_eq!(events.len(), 2);
}
