use crate::support::{COMMANDER, FAMILY, TestContext, VOLUNTEER, create_clue_request};
use angui::{
    entities::{clues, locations},
    services::{case_service, location_service, task_service},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, sea_query::Expr};
use serde_json::json;

#[actix_web::test]
async fn shared_location_is_canonical_for_clues_tasks_and_family_redaction() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    let commander = context.authenticated(COMMANDER).await;
    let family = context.authenticated(FAMILY).await;
    let mut request = create_clue_request();
    request.location_text = Some("Fictional park gate".to_owned());
    request.location_precision = Some("exact".to_owned());
    request.location_kind = Some("point".to_owned());
    request.longitude = Some(120.0);
    request.latitude = Some(30.0);
    request.visibility = Some("confirmed".to_owned());
    let clue = case_service::create_clue(&context.database, &commander, &case_id, request)
        .await
        .unwrap();
    let location_id = clue
        .location_id
        .clone()
        .expect("canonical location must be created");
    let mut reuse = create_clue_request();
    reuse.location_id = Some(location_id.clone());
    reuse.location_text = None;
    let second = case_service::create_clue(&context.database, &commander, &case_id, reuse)
        .await
        .unwrap();
    assert_eq!(second.location_id.as_deref(), Some(location_id.as_str()));
    assert_eq!(
        locations::Entity::find()
            .count(&context.database)
            .await
            .unwrap(),
        1
    );
    let confirmed = case_service::review_clue(
        &context.database,
        &commander,
        &clue.id,
        serde_json::from_value(json!({"status":"confirmed", "reason":"Fictional verified source"}))
            .unwrap(),
    )
    .await
    .unwrap();
    let task = task_service::create_task(&context.database, &commander, &case_id,
        serde_json::from_value(json!({
            "source_clue_id":confirmed.id,"location_id":location_id,"area_text":"",
            "title":"Fictional search", "objective":"Fictional objective",
            "latitude":null,"longitude":null,"due_at":"2099-07-27T12:00:00Z",
            "background":"Fictional background", "risk_level":"low", "risk_notes":"Fictional safety note",
            "safety_briefing":"Fictional briefing", "expected_feedback":"Fictional feedback"
        })).unwrap()).await.unwrap();
    assert_eq!(task.location_id.as_deref(), Some(location_id.as_str()));
    assert_eq!(
        task.location.unwrap().location_kind.as_deref(),
        Some("point")
    );
    // Deliberately make the compatibility columns stale. Reads must use the
    // canonical row, including map/task projections, rather than these copies.
    clues::Entity::update_many()
        .col_expr(
            clues::Column::LocationText,
            Expr::value("stale compatibility text"),
        )
        .filter(clues::Column::CaseId.eq(&case_id))
        .exec(&context.database)
        .await
        .unwrap();
    locations::Entity::update_many()
        .col_expr(
            locations::Column::LocationText,
            Expr::value("Fictional corrected gate"),
        )
        .col_expr(locations::Column::Longitude, Expr::value(121.0))
        .filter(locations::Column::Id.eq(&location_id))
        .exec(&context.database)
        .await
        .unwrap();
    let detail = case_service::get_case(&context.database, &commander, &case_id)
        .await
        .unwrap();
    assert!(
        detail
            .clues
            .iter()
            .all(|clue| clue.location_text.as_deref() == Some("Fictional corrected gate"))
    );
    assert!(
        detail
            .clues
            .iter()
            .all(|clue| clue.longitude == Some(121.0))
    );
    let tasks = task_service::list_all_visible_tasks(&context.database, &commander, &case_id)
        .await
        .unwrap();
    assert_eq!(tasks[0].longitude, Some(121.0));
    assert_eq!(tasks[0].area_text, "Fictional corrected gate");
    let detail = case_service::get_case(&context.database, &family, &case_id)
        .await
        .unwrap();
    for clue in detail.clues {
        assert!(clue.longitude.is_none());
        assert!(clue.latitude.is_none());
        let location = clue.location.unwrap();
        assert!(location.longitude.is_none());
        assert!(location.latitude.is_none());
        assert_eq!(location.location_precision.as_deref(), Some("unknown"));
    }
    let listed = location_service::list(&context.database, &family, &case_id)
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert!(listed[0].longitude.is_none());
    let mut family_reuse = create_clue_request();
    family_reuse.location_id = Some(location_id.clone());
    family_reuse.location_text = None;
    let response = case_service::create_clue(&context.database, &family, &case_id, family_reuse)
        .await
        .unwrap();
    assert!(response.longitude.is_none());
    assert!(response.location.unwrap().longitude.is_none());
}

#[actix_web::test]
async fn references_reject_cross_case_hidden_and_conflicting_positions_without_orphans() {
    let context = TestContext::new().await;
    let case_id = context.create_case().await;
    let other_case = context.create_case().await;
    context
        .add_member(&case_id, FAMILY, COMMANDER, "commander")
        .await;
    context
        .add_member(&case_id, COMMANDER, VOLUNTEER, "volunteer")
        .await;
    let commander = context.authenticated(COMMANDER).await;
    let family = context.authenticated(FAMILY).await;
    let volunteer = context.authenticated(VOLUNTEER).await;
    let clue = case_service::create_clue(
        &context.database,
        &commander,
        &case_id,
        create_clue_request(),
    )
    .await
    .unwrap();
    let location_id = clue.location_id.unwrap();
    for (auth, target, text) in [
        (&family, &other_case, None),
        (&volunteer, &case_id, None),
        (
            &commander,
            &case_id,
            Some("Conflicting fictional gate".to_owned()),
        ),
    ] {
        let mut request = create_clue_request();
        request.location_id = Some(location_id.clone());
        request.location_text = text;
        assert!(
            case_service::create_clue(&context.database, auth, target, request)
                .await
                .is_err()
        );
    }
    let mut failed = create_clue_request();
    failed
        .attachment_ids
        .push("nonexistent-fictional-attachment".to_owned());
    assert!(
        case_service::create_clue(&context.database, &commander, &case_id, failed)
            .await
            .is_err()
    );
    assert_eq!(
        locations::Entity::find()
            .count(&context.database)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        clues::Entity::find()
            .count(&context.database)
            .await
            .unwrap(),
        1
    );
    assert!(
        location_service::list(&context.database, &volunteer, &case_id)
            .await
            .unwrap()
            .is_empty()
    );
}
