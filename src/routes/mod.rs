mod auth;
mod cases;
mod clues;
mod health;
mod intake_sessions;

use actix_web::web;

use crate::auth::ApiSessionAuthentication;

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/api")
            .wrap(ApiSessionAuthentication)
            .service(health::get_health)
            .configure(auth::configure)
            .configure(cases::configure)
            .configure(intake_sessions::configure)
            .configure(clues::configure),
    );
}
