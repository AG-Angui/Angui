use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AdminAuditEventQuery, AdminUserQuery, AuthenticatedUser, UpdateAdminUserStatusRequest,
    },
    services::admin_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/admin")
            .route("/audit-events", web::get().to(list_audit_events))
            .route("/users", web::get().to(list_users))
            .route(
                "/users/{user_id}/status",
                web::patch().to(update_user_status),
            ),
    );
}

async fn list_audit_events(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    query: web::Query<AdminAuditEventQuery>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(admin_service::list_audit_events(&state.db, &auth, query.into_inner()).await?))
}

async fn list_users(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    query: web::Query<AdminUserQuery>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok()
        .json(admin_service::list_users(&state.db, &auth, query.into_inner()).await?))
}

async fn update_user_status(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    user_id: web::Path<String>,
    request: web::Json<UpdateAdminUserStatusRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        admin_service::update_user_status(&state.db, &auth, &user_id, request.into_inner()).await?,
    ))
}
