use actix_multipart::Multipart;
use actix_web::{HttpResponse, web};
use futures_util::StreamExt;

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AuthenticatedUser, CreateKnowledgeBaseRequest, CreateKnowledgeItemRequest,
        KnowledgeChatRequest, KnowledgeSearchRequest, UpdateKnowledgeBaseRequest,
        UpdateKnowledgeItemRequest,
    },
    services::knowledge_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(
            web::scope("/admin/knowledge-bases")
                .route("", web::get().to(list_bases))
                .route("", web::post().to(create_base))
                .route("/{id}", web::get().to(get_base))
                .route("/{id}", web::patch().to(update_base))
                .route("/{id}/enable", web::post().to(enable_base))
                .route("/{id}/disable", web::post().to(disable_base))
                .route("/{id}/items", web::get().to(list_items))
                .route("/{id}/items", web::post().to(create_item)),
        )
        .service(
            web::scope("/admin/knowledge-items")
                .route("/{id}", web::get().to(get_item))
                .route("/{id}", web::patch().to(update_item))
                .route("/{id}/review", web::post().to(review_item))
                .route("/{id}/publish", web::post().to(publish_item))
                .route("/{id}/withdraw", web::post().to(withdraw_item)),
        )
        .service(
            web::scope("/knowledge-bases")
                .route("/{id}/search", web::post().to(search))
                .route("/{id}/chat", web::post().to(chat)),
        )
        .service(
            web::scope("/admin/knowledge-imports")
                .route("/{id}", web::get().to(get_import))
                .route("/{id}/confirm", web::post().to(confirm_import))
                .route("/{id}/cancel", web::post().to(cancel_import)),
        )
        .service(
            web::scope("/admin/knowledge-bases/{id}/imports")
                .route("/preview", web::post().to(preview_import)),
        );
}

async fn list_bases(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(knowledge_service::list_bases(&state.db, &auth).await?))
}
async fn create_base(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    request: web::Json<CreateKnowledgeBaseRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created()
        .json(knowledge_service::create_base(&state.db, &auth, request.into_inner()).await?))
}
async fn get_base(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(knowledge_service::get_base(&state.db, &auth, &id).await?))
}
async fn update_base(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
    request: web::Json<UpdateKnowledgeBaseRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(knowledge_service::update_base(&state.db, &auth, &id, request.into_inner()).await?))
}
async fn enable_base(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(knowledge_service::set_base_status(&state.db, &auth, &id, "enabled").await?))
}
async fn disable_base(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(knowledge_service::set_base_status(&state.db, &auth, &id, "disabled").await?))
}
async fn list_items(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(knowledge_service::list_items(&state.db, &auth, &id).await?))
}
async fn create_item(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
    request: web::Json<CreateKnowledgeItemRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Created()
        .json(knowledge_service::create_item(&state.db, &auth, &id, request.into_inner()).await?))
}
async fn get_item(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(knowledge_service::get_item(&state.db, &auth, &id).await?))
}
async fn update_item(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
    request: web::Json<UpdateKnowledgeItemRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(knowledge_service::update_item(&state.db, &auth, &id, request.into_inner()).await?))
}
async fn review_item(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(knowledge_service::transition_item(&state.db, &auth, &id, "review").await?))
}
async fn publish_item(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(knowledge_service::transition_item(&state.db, &auth, &id, "publish").await?))
}
async fn withdraw_item(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(knowledge_service::transition_item(&state.db, &auth, &id, "withdraw").await?))
}
async fn search(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
    request: web::Json<KnowledgeSearchRequest>,
) -> Result<HttpResponse, ApiError> {
    let request = request.into_inner();
    Ok(HttpResponse::Ok().json(
        knowledge_service::search(&state.db, &auth, &id, &request.query, request.limit).await?,
    ))
}
async fn chat(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
    request: web::Json<KnowledgeChatRequest>,
) -> Result<HttpResponse, ApiError> {
    let request = request.into_inner();
    Ok(HttpResponse::Ok().json(
        knowledge_service::chat_with_gateway(
            &state.db,
            &auth,
            &id,
            &request.query,
            request.limit,
            &state.ai_gateway,
        )
        .await?,
    ))
}

async fn preview_import(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
    mut payload: Multipart,
) -> Result<HttpResponse, ApiError> {
    let mut bytes = Vec::new();
    let mut file_name = "import.csv".to_owned();
    while let Some(field) = payload.next().await {
        let mut field =
            field.map_err(|_| ApiError::Validation("invalid multipart upload".to_owned()))?;
        if let Some(disposition) = field.content_disposition()
            && let Some(name) = disposition.get_filename()
        {
            file_name = name.to_owned();
        }
        while let Some(chunk) = field.next().await {
            let chunk =
                chunk.map_err(|_| ApiError::Validation("invalid multipart upload".to_owned()))?;
            bytes.extend_from_slice(&chunk);
            if bytes.len() > 5 * 1024 * 1024 {
                return Err(ApiError::Validation(
                    "CSV file must not exceed 5 MB".to_owned(),
                ));
            }
        }
    }
    Ok(HttpResponse::Created()
        .json(knowledge_service::preview_csv(&state.db, &auth, &id, file_name, bytes).await?))
}
async fn get_import(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(knowledge_service::get_import(&state.db, &auth, &id).await?))
}
async fn confirm_import(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(knowledge_service::confirm_import(&state.db, &auth, &id).await?))
}
async fn cancel_import(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(knowledge_service::cancel_import(&state.db, &auth, &id).await?))
}
