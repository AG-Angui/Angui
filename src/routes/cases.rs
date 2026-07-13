use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{CreateCaseRequest, CreateClueRequest, UpdateCaseStatusRequest},
    services::case_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/cases")
            .route("", web::get().to(list_cases))
            .route("", web::post().to(create_case))
            .route("/{case_id}", web::get().to(get_case))
            .route("/{case_id}/status", web::patch().to(update_case_status))
            .route("/{case_id}/clues", web::post().to(create_clue)),
    );
}

async fn list_cases(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let cases = case_service::list_cases(&state.db).await?;
    Ok(HttpResponse::Ok().json(cases))
}

async fn create_case(
    state: web::Data<AppState>,
    request: web::Json<CreateCaseRequest>,
) -> Result<HttpResponse, ApiError> {
    let case = case_service::create_case(&state.db, request.into_inner()).await?;
    Ok(HttpResponse::Created().json(case))
}

async fn get_case(
    state: web::Data<AppState>,
    case_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let case = case_service::get_case(&state.db, &case_id).await?;
    Ok(HttpResponse::Ok().json(case))
}

async fn update_case_status(
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<UpdateCaseStatusRequest>,
) -> Result<HttpResponse, ApiError> {
    let case = case_service::update_case_status(&state.db, &case_id, request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(case))
}

async fn create_clue(
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<CreateClueRequest>,
) -> Result<HttpResponse, ApiError> {
    let clue = case_service::create_clue(&state.db, &case_id, request.into_inner()).await?;
    Ok(HttpResponse::Created().json(clue))
}
