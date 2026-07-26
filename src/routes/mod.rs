mod auth;
mod cases;
mod clues;
mod health;
mod intake_sessions;
mod tasks;

use actix_web::{HttpResponse, web};

use serde_json::json;

use crate::auth::ApiSessionAuthentication;

pub fn configure(config: &mut web::ServiceConfig) {
    let json_config = web::JsonConfig::default().error_handler(|error, _request| {
        actix_web::error::InternalError::from_response(
            error,
            HttpResponse::BadRequest().json(json!({
                "error": {
                    "code": "validation_error",
                    "message": "request body is invalid"
                }
            })),
        )
        .into()
    });
    let query_config = web::QueryConfig::default().error_handler(|error, _request| {
        actix_web::error::InternalError::from_response(
            error,
            HttpResponse::BadRequest().json(json!({
                "error": {
                    "code": "validation_error",
                    "message": "query parameters are invalid"
                }
            })),
        )
        .into()
    });
    config.service(
        web::scope("/api")
            .app_data(json_config)
            .app_data(query_config)
            .wrap(ApiSessionAuthentication)
            .service(health::get_health)
            .configure(auth::configure)
            .configure(cases::configure)
            .configure(intake_sessions::configure)
            .configure(clues::configure)
            .configure(tasks::configure),
    );
}
