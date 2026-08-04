use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AuthenticatedUser, KnowledgeAskRequest, LearningQuestionQuery, LearningResourceQuery,
        SubmitLearningAnswerRequest,
    },
    services::learning_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(
            web::scope("/learning")
                .route("/resources", web::get().to(list_resources))
                .route("/questions", web::get().to(list_questions))
                .route(
                    "/questions/{question_id}/answers",
                    web::post().to(submit_answer),
                ),
        )
        .service(web::scope("/knowledge").route("/ask", web::post().to(ask_knowledge)));
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
