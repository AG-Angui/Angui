use actix_web::{HttpRequest, HttpResponse, web};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AuthenticatedUser, CreateTaskApplicationRequest, ReviewTaskApplicationRequest,
        SubmitTaskFeedbackRequest, SubmitTaskLocationReportRequest, UpdateTaskStatusRequest,
    },
    services::task_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/tasks")
            .route("/mine", web::get().to(list_my_tasks))
            .route(
                "/{task_id}/applications",
                web::post().to(create_task_application),
            )
            .route(
                "/{task_id}/applications",
                web::get().to(list_task_applications),
            )
            .route(
                "/{task_id}/applications/{application_id}",
                web::patch().to(review_task_application),
            )
            .route(
                "/{task_id}/collaboration-locations",
                web::get().to(list_collaboration_locations),
            )
            .route(
                "/{task_id}/location-reports",
                web::post().to(submit_location_report),
            )
            .route("/{task_id}/feedback", web::post().to(submit_task_feedback))
            .route(
                "/{task_id}/safety-briefing",
                web::get().to(get_safety_briefing),
            )
            .route("/{task_id}/navigation", web::get().to(get_navigation))
            .route("/{task_id}/status", web::patch().to(update_task_status)),
    );
}

async fn list_my_tasks(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(task_service::list_my_tasks(&state.db, &auth).await?))
}

async fn update_task_status(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    task_id: web::Path<String>,
    request: web::Json<UpdateTaskStatusRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        task_service::update_task_status(&state.db, &auth, &task_id, request.into_inner()).await?,
    ))
}

async fn submit_location_report(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    http_request: HttpRequest,
    task_id: web::Path<String>,
    request: web::Json<SubmitTaskLocationReportRequest>,
) -> Result<HttpResponse, ApiError> {
    let idempotency_key = required_idempotency_key(&http_request)?;
    let receipt = task_service::submit_location_report(
        &state.db,
        &auth,
        &task_id,
        request.into_inner(),
        &idempotency_key,
    )
    .await?;
    Ok(HttpResponse::Created().json(receipt))
}

async fn submit_task_feedback(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    http_request: HttpRequest,
    task_id: web::Path<String>,
    request: web::Json<SubmitTaskFeedbackRequest>,
) -> Result<HttpResponse, ApiError> {
    let idempotency_key = required_idempotency_key(&http_request)?;
    let receipt = task_service::submit_task_feedback(
        &state.db,
        &auth,
        &task_id,
        request.into_inner(),
        &idempotency_key,
    )
    .await?;
    Ok(HttpResponse::Created().json(receipt))
}

async fn create_task_application(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    task_id: web::Path<String>,
    request: web::Json<CreateTaskApplicationRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created().json(
        task_service::create_task_application(&state.db, &auth, &task_id, request.into_inner())
            .await?,
    ))
}

async fn list_task_applications(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    task_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(task_service::list_task_applications(&state.db, &auth, &task_id).await?))
}

async fn review_task_application(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    request: web::Json<ReviewTaskApplicationRequest>,
) -> Result<HttpResponse, ApiError> {
    let (task_id, application_id) = path.into_inner();
    Ok(HttpResponse::Ok().json(
        task_service::review_task_application(
            &state.db,
            &auth,
            &task_id,
            &application_id,
            request.into_inner(),
        )
        .await?,
    ))
}

async fn list_collaboration_locations(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    task_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(task_service::list_task_collaboration_locations(&state.db, &auth, &task_id).await?))
}

fn required_idempotency_key(request: &HttpRequest) -> Result<String, ApiError> {
    let value = request
        .headers()
        .get("Idempotency-Key")
        .ok_or_else(|| ApiError::Validation("Idempotency-Key header is required".to_owned()))?
        .to_str()
        .map_err(|_| ApiError::Validation("Idempotency-Key must be a UUID".to_owned()))?;
    Uuid::parse_str(value.trim())
        .map(|key| key.to_string())
        .map_err(|_| ApiError::Validation("Idempotency-Key must be a UUID".to_owned()))
}

async fn get_safety_briefing(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    task_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(task_service::get_task_safety_briefing(&state.db, &auth, &task_id).await?))
}

async fn get_navigation(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    task_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(task_service::get_task_navigation(&state.db, &auth, &task_id).await?))
}
