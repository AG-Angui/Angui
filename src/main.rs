mod config;
mod routes;

use std::io;

use actix_cors::Cors;
use actix_web::{App, HttpServer, http, middleware::Logger};
use config::Settings;

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let settings = Settings::from_env()
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let address = settings.address();

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
            .wrap(Logger::default())
            .wrap(cors)
            .configure(routes::configure)
    })
    .bind(address)?
    .run()
    .await
}
