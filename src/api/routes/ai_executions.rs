use actix_web::{HttpResponse, http::header, web};
use serde::Deserialize;

use crate::{
    app_state::AppState, error::ApiError, models::AuthenticatedUser, services::ai_execution_service,
};

#[derive(Debug, Deserialize)]
struct EventQuery {
    after: Option<i64>,
}

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/ai/executions")
            .route("/{execution_id}", web::get().to(get_execution))
            .route(
                "/{execution_id}/events",
                web::get().to(list_execution_events),
            ),
    );
}

async fn get_execution(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    execution_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(ai_execution_service::get_execution(&state.db, &auth, &execution_id).await?))
}

async fn list_execution_events(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    execution_id: web::Path<String>,
    query: web::Query<EventQuery>,
) -> Result<HttpResponse, ApiError> {
    let events = ai_execution_service::list_events(
        &state.db,
        &auth,
        &execution_id,
        query.after.unwrap_or(0),
    )
    .await?;
    let body = events
        .into_iter()
        .map(|event| {
            let payload = serde_json::to_string(&event)
                .unwrap_or_else(|_| "{\"message\":\"internal service error\"}".to_owned());
            format!(
                "id: {}\nevent: {}\ndata: {}\n\n",
                event.event_id, event.event_type, payload
            )
        })
        .collect::<String>();
    Ok(HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/event-stream"))
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .body(body))
}
