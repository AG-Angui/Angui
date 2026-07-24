use std::{env, io};

use angui::services::auth_service;
use sea_orm::Database;

fn demo_bootstrap_is_allowed(runtime_environment: &str, explicit_allow: &str) -> bool {
    matches!(
        runtime_environment.trim(),
        "development" | "preview" | "test"
    ) && explicit_allow.trim() == "1"
}

fn require_demo_bootstrap_permission() -> io::Result<()> {
    let runtime_environment = env::var("ANGUI_RUNTIME_ENV").unwrap_or_default();
    let explicit_allow = env::var("ANGUI_ALLOW_DEMO_BOOTSTRAP").unwrap_or_default();
    if demo_bootstrap_is_allowed(&runtime_environment, &explicit_allow) {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "bootstrap-demo is allowed only when ANGUI_RUNTIME_ENV is development, preview, or test and ANGUI_ALLOW_DEMO_BOOTSTRAP=1",
    ))
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let command = env::args().nth(1).unwrap_or_default();
    if command != "bootstrap-demo" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: cargo run --bin angui-admin -- bootstrap-demo",
        ));
    }
    require_demo_bootstrap_permission()?;

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

#[cfg(test)]
mod tests {
    use super::demo_bootstrap_is_allowed;

    #[test]
    fn demo_bootstrap_requires_an_explicit_non_production_environment_and_switch() {
        for environment in ["development", "preview", "test"] {
            assert!(demo_bootstrap_is_allowed(environment, "1"));
        }

        for environment in ["", "production", "staging", "Preview"] {
            assert!(!demo_bootstrap_is_allowed(environment, "1"));
        }
        assert!(!demo_bootstrap_is_allowed("preview", ""));
        assert!(!demo_bootstrap_is_allowed("preview", "true"));
    }
}
