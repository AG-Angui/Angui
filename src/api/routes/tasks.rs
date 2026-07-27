use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AuthenticatedUser, SubmitTaskFeedbackRequest, SubmitTaskLocationReportRequest,
        UpdateTaskStatusRequest,
    },
    services::task_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/tasks")
            .route("/mine", web::get().to(list_my_tasks))
            .route(
                "/{task_id}/location-reports",
                web::post().to(submit_location_report),
            )
            .route("/{task_id}/feedback", web::post().to(submit_task_feedback))
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
    task_id: web::Path<String>,
    request: web::Json<SubmitTaskLocationReportRequest>,
) -> Result<HttpResponse, ApiError> {
    let receipt =
        task_service::submit_location_report(&state.db, &auth, &task_id, request.into_inner())
            .await?;
    Ok(HttpResponse::Created().json(receipt))
}

async fn submit_task_feedback(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    task_id: web::Path<String>,
    request: web::Json<SubmitTaskFeedbackRequest>,
) -> Result<HttpResponse, ApiError> {
    let receipt =
        task_service::submit_task_feedback(&state.db, &auth, &task_id, request.into_inner())
            .await?;
    Ok(HttpResponse::Created().json(receipt))
}
