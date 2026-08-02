use actix_web::{HttpResponse, web};

use crate::{
    app_state::AppState,
    error::ApiError,
    models::{
        AdminAuditEventQuery, AdminUserQuery, AuthenticatedUser, DeidentifyArchiveDraftRequest,
        ReviewArchiveDraftRequest, UpdateAdminUserStatusRequest,
    },
    services::admin_service,
};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/admin")
            .route("/audit-events", web::get().to(list_audit_events))
            .route("/users", web::get().to(list_users))
            .route("/archive-drafts", web::get().to(list_archive_drafts))
            .route(
                "/users/{user_id}/status",
                web::patch().to(update_user_status),
            )
            .route(
                "/archive-drafts/{draft_id}/deidentify",
                web::post().to(deidentify_archive_draft),
            )
            .route(
                "/archive-drafts/{draft_id}/review",
                web::patch().to(review_archive_draft),
            )
            .route(
                "/archive-drafts/{draft_id}/review-materials",
                web::get().to(list_archive_review_materials),
            )
            .route(
                "/archive-drafts/{draft_id}/review-materials/diff/{from_version}/{to_version}",
                web::get().to(diff_archive_review_materials),
            )
            .route(
                "/archive-drafts/{draft_id}/review-materials/{version}/restore",
                web::post().to(restore_archive_review_material),
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

async fn list_archive_drafts(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        crate::services::case_collaboration_service::list_archive_drafts_for_admin(
            &state.db, &auth,
        )
        .await?,
    ))
}

async fn deidentify_archive_draft(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    draft_id: web::Path<String>,
    request: web::Json<DeidentifyArchiveDraftRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        crate::services::case_collaboration_service::deidentify_archive_draft(
            &state.db,
            &auth,
            &draft_id,
            request.into_inner(),
            &state.ai_gateway,
        )
        .await?,
    ))
}

async fn review_archive_draft(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    draft_id: web::Path<String>,
    request: web::Json<ReviewArchiveDraftRequest>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        crate::services::case_collaboration_service::review_archive_draft(
            &state.db,
            &auth,
            &draft_id,
            request.into_inner(),
        )
        .await?,
    ))
}

async fn list_archive_review_materials(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    draft_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(
        crate::services::case_collaboration_service::list_archive_review_materials(
            &state.db, &auth, &draft_id,
        )
        .await?,
    ))
}

async fn diff_archive_review_materials(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, i32, i32)>,
) -> Result<HttpResponse, ApiError> {
    let (draft_id, from_version, to_version) = path.into_inner();
    Ok(HttpResponse::Ok().json(
        crate::services::case_collaboration_service::diff_archive_review_materials(
            &state.db,
            &auth,
            &draft_id,
            from_version,
            to_version,
        )
        .await?,
    ))
}

async fn restore_archive_review_material(
    auth: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(String, i32)>,
    request: web::Json<crate::models::RestoreArchiveReviewMaterialRequest>,
) -> Result<HttpResponse, ApiError> {
    let (draft_id, version) = path.into_inner();
    Ok(HttpResponse::Ok().json(
        crate::services::case_collaboration_service::restore_archive_review_material(
            &state.db,
            &auth,
            &draft_id,
            version,
            request.into_inner(),
            &state.ai_gateway,
        )
        .await?,
    ))
}
