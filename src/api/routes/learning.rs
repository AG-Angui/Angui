use actix_web::{HttpResponse, http::header, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AuthenticatedUser, CreateLearningQuestionRequest, CreateLearningResourceRequest,
        KnowledgeAskRequest, LearningContentActionRequest, LearningQuestionQuery,
        LearningResourceQuery, SubmitLearningAnswerRequest,
    },
    services::learning_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(
            web::scope("/learning")
                .route(
                    "/public/prevention-card",
                    web::get().to(public_prevention_card),
                )
                .route("/resources", web::get().to(list_resources))
                .route("/questions", web::get().to(list_questions))
                .route(
                    "/questions/{question_id}/answers",
                    web::post().to(submit_answer),
                ),
        )
        .service(
            web::scope("/admin/learning")
                .route("/resources", web::get().to(list_managed_resources))
                .route("/resources", web::post().to(create_resource))
                .route(
                    "/resources/{resource_id}/deidentify",
                    web::post().to(deidentify_resource),
                )
                .route(
                    "/resources/{resource_id}/review",
                    web::post().to(review_resource),
                )
                .route(
                    "/resources/{resource_id}/publish",
                    web::post().to(publish_resource),
                )
                .route(
                    "/resources/{resource_id}/withdraw",
                    web::post().to(withdraw_resource),
                )
                .route(
                    "/resources/{resource_id}/export",
                    web::get().to(export_resource),
                )
                .route("/questions", web::get().to(list_managed_questions))
                .route("/questions", web::post().to(create_question))
                .route(
                    "/questions/{question_id}/deidentify",
                    web::post().to(deidentify_question),
                )
                .route(
                    "/questions/{question_id}/review",
                    web::post().to(review_question),
                )
                .route(
                    "/questions/{question_id}/publish",
                    web::post().to(publish_question),
                )
                .route(
                    "/questions/{question_id}/withdraw",
                    web::post().to(withdraw_question),
                )
                .route(
                    "/questions/{question_id}/export",
                    web::get().to(export_question),
                ),
        )
        .service(web::scope("/knowledge").route("/ask", web::post().to(ask_knowledge)));
}

async fn public_prevention_card(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(learning_service::public_prevention_card(&state.db).await?))
}

async fn list_resources(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    query: web::Query<LearningResourceQuery>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(learning_service::list_resources(&state.db, &auth, query.into_inner()).await?))
}

async fn list_questions(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    query: web::Query<LearningQuestionQuery>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(learning_service::list_questions(&state.db, &auth, query.into_inner()).await?))
}

async fn submit_answer(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    question_id: web::Path<String>,
    request: web::Json<SubmitLearningAnswerRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        learning_service::submit_answer(&state.db, &auth, &question_id, request.into_inner())
            .await?,
    ))
}

async fn ask_knowledge(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    request: web::Json<KnowledgeAskRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(learning_service::ask_knowledge(&state.db, &auth, request.into_inner()).await?))
}

async fn list_managed_resources(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(learning_service::list_managed_resources(&state.db, &auth).await?))
}

async fn create_resource(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    request: web::Json<CreateLearningResourceRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created()
        .json(learning_service::create_resource(&state.db, &auth, request.into_inner()).await?))
}

async fn deidentify_resource(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    resource_id: web::Path<String>,
    request: web::Json<LearningContentActionRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        learning_service::deidentify_resource(&state.db, &auth, &resource_id, request.into_inner())
            .await?,
    ))
}

async fn review_resource(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    resource_id: web::Path<String>,
    request: web::Json<LearningContentActionRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        learning_service::review_resource(&state.db, &auth, &resource_id, request.into_inner())
            .await?,
    ))
}

async fn publish_resource(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    resource_id: web::Path<String>,
    request: web::Json<LearningContentActionRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        learning_service::publish_resource(&state.db, &auth, &resource_id, request.into_inner())
            .await?,
    ))
}

async fn withdraw_resource(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    resource_id: web::Path<String>,
    request: web::Json<LearningContentActionRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        learning_service::withdraw_resource(&state.db, &auth, &resource_id, request.into_inner())
            .await?,
    ))
}

async fn export_resource(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    resource_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .insert_header((
            header::CONTENT_DISPOSITION,
            "attachment; filename=learning-resource.json",
        ))
        .json(learning_service::export_resource(&state.db, &auth, &resource_id).await?))
}

async fn list_managed_questions(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(learning_service::list_managed_questions(&state.db, &auth).await?))
}

async fn create_question(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    request: web::Json<CreateLearningQuestionRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created()
        .json(learning_service::create_question(&state.db, &auth, request.into_inner()).await?))
}

async fn deidentify_question(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    question_id: web::Path<String>,
    request: web::Json<LearningContentActionRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        learning_service::deidentify_question(&state.db, &auth, &question_id, request.into_inner())
            .await?,
    ))
}

async fn review_question(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    question_id: web::Path<String>,
    request: web::Json<LearningContentActionRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        learning_service::review_question(&state.db, &auth, &question_id, request.into_inner())
            .await?,
    ))
}

async fn publish_question(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    question_id: web::Path<String>,
    request: web::Json<LearningContentActionRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        learning_service::publish_question(&state.db, &auth, &question_id, request.into_inner())
            .await?,
    ))
}

async fn withdraw_question(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    question_id: web::Path<String>,
    request: web::Json<LearningContentActionRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        learning_service::withdraw_question(&state.db, &auth, &question_id, request.into_inner())
            .await?,
    ))
}

async fn export_question(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    question_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .insert_header((
            header::CONTENT_DISPOSITION,
            "attachment; filename=learning-question.json",
        ))
        .json(learning_service::export_question(&state.db, &auth, &question_id).await?))
}
