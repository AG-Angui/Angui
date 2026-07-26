use std::io;

use actix_cors::Cors;
use actix_web::{App, HttpServer, http, middleware::Logger, web};
use angui::{
    ai_gateway::AiGateway, amap_service::AmapService, app_state::AppState, config::Settings,
    rate_limit::LoginRateLimiter, routes, services::task_service,
};
use sea_orm::Database;

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let settings = Settings::from_env()
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let address = settings.address();
    let database = Database::connect(&settings.database_url)
        .await
        .map_err(|error| io::Error::other(format!("database connection failed: {error}")))?;
    let ai_gateway = AiGateway::from_configurations(settings.ai_provider_configurations.clone())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
    let state = web::Data::new(AppState {
        db: database,
        session_ttl_hours: settings.session_ttl_hours,
        intake_answer_hard_max: settings.intake_answer_hard_max,
        attachment_storage_directory: settings.attachment_storage_directory.clone(),
        attachment_max_image_bytes: settings.attachment_max_image_bytes,
        attachment_max_per_case: settings.attachment_max_per_case,
        case_place_types: settings.case_place_types.clone(),
        amap_service: AmapService::new(
            settings.amap_webservice_key,
            settings.amap_webservice_base_url,
            settings.amap_timeout_ms,
        )
        .map_err(|error| io::Error::other(format!("AMap client initialization failed: {error}")))?,
        ai_gateway,
        login_limiter: LoginRateLimiter::default(),
    });
    task_service::start_location_report_retention_purger(state.db.clone());

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&settings.frontend_origin)
            .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::CONTENT_TYPE,
            ])
            .max_age(3600);

        App::new()
            .app_data(state.clone())
            .wrap(Logger::default())
            .wrap(cors)
            .configure(routes::configure)
    })
    .bind(address)?
    .run()
    .await
}
