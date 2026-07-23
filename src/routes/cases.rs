use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AddCaseMemberRequest, AuthenticatedUser, CreateCaseRequest, CreateClueRequest,
        UpdateCaseStatusRequest,
    },
    services::case_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/cases")
            .route("", web::get().to(list_cases))
            .route("", web::post().to(create_case))
            .route("/{case_id}", web::get().to(get_case))
            .route("/{case_id}/status", web::patch().to(update_case_status))
            .route("/{case_id}/clues", web::post().to(create_clue))
            .route("/{case_id}/members", web::post().to(add_case_member)),
    );
}

async fn list_cases(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, ApiError> {
    let cases = case_service::list_cases(&state.db, &auth).await?;
    Ok(HttpResponse::Ok().json(cases))
}

async fn create_case(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    request: web::Json<CreateCaseRequest>,
) -> Result<HttpResponse, ApiError> {
    let case = case_service::create_case(&state.db, &auth, request.into_inner()).await?;
    Ok(HttpResponse::Created().json(case))
}

async fn get_case(
    state: web::Data<AppState>,
    auth: AuthenticatedUser,
    case_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let case = case_service::get_case(&state.db, &auth, &case_id).await?;
    Ok(HttpResponse::Ok().json(case))
}

async fn update_case_status(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<UpdateCaseStatusRequest>,
) -> Result<HttpResponse, ApiError> {
    let case =
        case_service::update_case_status(&state.db, &auth, &case_id, request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(case))
}

async fn create_clue(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<CreateClueRequest>,
) -> Result<HttpResponse, ApiError> {
    let clue = case_service::create_clue(&state.db, &auth, &case_id, request.into_inner()).await?;
    Ok(HttpResponse::Created().json(clue))
}

async fn add_case_member(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    case_id: web::Path<String>,
    request: web::Json<AddCaseMemberRequest>,
) -> Result<HttpResponse, ApiError> {
    let member =
        case_service::add_case_member(&state.db, &auth, &case_id, request.into_inner()).await?;
    Ok(HttpResponse::Created().json(member))
}
