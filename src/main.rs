mod app_state;
mod config;
mod entities;
mod error;
mod models;
mod routes;
mod services;

use std::io;

use actix_cors::Cors;
use actix_web::{App, HttpServer, http, middleware::Logger, web};
use app_state::AppState;
use config::Settings;
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
    let state = web::Data::new(AppState { db: database });

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
