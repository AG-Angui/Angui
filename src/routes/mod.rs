mod cases;
mod clues;
mod health;

use actix_web::web;

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/api")
            .service(health::get_health)
            .configure(cases::configure)
            .configure(clues::configure),
    );
}
