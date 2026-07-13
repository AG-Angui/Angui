use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState, error::ApiError, models::ReviewClueRequest, services::case_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(web::scope("/clues").route("/{clue_id}/review", web::patch().to(review_clue)));
}

async fn review_clue(
    state: web::Data<AppState>,
    clue_id: web::Path<String>,
    request: web::Json<ReviewClueRequest>,
) -> Result<HttpResponse, ApiError> {
    let clue = case_service::review_clue(&state.db, &clue_id, request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(clue))
}
