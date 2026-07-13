use std::{env, io};

use angui::services::auth_service;
use sea_orm::Database;

#[tokio::main]
async fn main() -> io::Result<()> {
    let command = env::args().nth(1).unwrap_or_default();
    if command != "bootstrap-demo" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: cargo run --bin angui-admin -- bootstrap-demo",
        ));
    }

    let database_url = env::var("DATABASE_URL")
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "DATABASE_URL is required"))?;
    let password = env::var("ANGUI_DEMO_PASSWORD").map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ANGUI_DEMO_PASSWORD is required and must contain 12-256 characters",
        )
    })?;
    let database = Database::connect(database_url)
        .await
        .map_err(|error| io::Error::other(format!("database connection failed: {error}")))?;
    let users = auth_service::bootstrap_demo_users(&database, &password)
        .await
        .map_err(|error| io::Error::other(error.to_string()))?;

    for user in users {
        println!("{}\t{}\t{}", user.role, user.email, user.display_name);
    }
    Ok(())
}
