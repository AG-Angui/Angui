use std::env;

use crate::ai_gateway::{ProviderConfig, validate_provider_configurations};

#[derive(Clone, Debug)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    pub frontend_origin: String,
    pub database_url: String,
    pub session_ttl_hours: i64,
    pub intake_answer_hard_max: usize,
    pub amap_webservice_key: Option<String>,
    pub amap_webservice_base_url: String,
    pub amap_timeout_ms: u64,
    pub ai_provider_configurations: Vec<ProviderConfig>,
}

impl Settings {
    pub fn from_env() -> Result<Self, String> {
        let host = env::var("ANGUI_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let port_value = env::var("ANGUI_PORT").unwrap_or_else(|_| "8080".to_owned());
        let port = port_value
            .parse::<u16>()
            .map_err(|_| format!("ANGUI_PORT must be a valid TCP port, got {port_value:?}"))?;
        let frontend_origin = env::var("ANGUI_FRONTEND_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:5173".to_owned());
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://data/angui.db?mode=rwc".to_owned());
        let session_ttl_value =
            env::var("ANGUI_SESSION_TTL_HOURS").unwrap_or_else(|_| "8".to_owned());
        let session_ttl_hours = session_ttl_value.parse::<i64>().map_err(|_| {
            format!("ANGUI_SESSION_TTL_HOURS must be a positive integer, got {session_ttl_value:?}")
        })?;
        if !(1..=168).contains(&session_ttl_hours) {
            return Err("ANGUI_SESSION_TTL_HOURS must be between 1 and 168".to_owned());
        }
        let intake_answer_hard_max_value =
            env::var("ANGUI_INTAKE_ANSWER_HARD_MAX").unwrap_or_else(|_| "2000".to_owned());
        let intake_answer_hard_max = intake_answer_hard_max_value.parse::<usize>().map_err(|_| {
            format!(
                "ANGUI_INTAKE_ANSWER_HARD_MAX must be a positive integer, got {intake_answer_hard_max_value:?}"
            )
        })?;
        if !(1..=10_000).contains(&intake_answer_hard_max) {
            return Err("ANGUI_INTAKE_ANSWER_HARD_MAX must be between 1 and 10000".to_owned());
        }
        let amap_webservice_key = env::var("AMAP_WEBSERVICE_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let amap_webservice_base_url = env::var("AMAP_WEBSERVICE_BASE_URL")
            .unwrap_or_else(|_| "https://restapi.amap.com".to_owned());
        if !amap_webservice_base_url.starts_with("https://") {
            return Err("AMAP_WEBSERVICE_BASE_URL must use https".to_owned());
        }
        let amap_timeout_value = env::var("AMAP_TIMEOUT_MS").unwrap_or_else(|_| "2500".to_owned());
        let amap_timeout_ms = amap_timeout_value.parse::<u64>().map_err(|_| {
            format!("AMAP_TIMEOUT_MS must be a positive integer, got {amap_timeout_value:?}")
        })?;
        if !(100..=10_000).contains(&amap_timeout_ms) {
            return Err("AMAP_TIMEOUT_MS must be between 100 and 10000".to_owned());
        }
        let provider_configurations_value =
            env::var("ANGUI_AI_PROVIDERS_JSON").unwrap_or_else(|_| "[]".to_owned());
        let ai_provider_configurations: Vec<ProviderConfig> = serde_json::from_str(
            &provider_configurations_value,
        )
        .map_err(|error| {
            format!(
                "ANGUI_AI_PROVIDERS_JSON must be a JSON array of provider configurations: {error}"
            )
        })?;
        validate_provider_configurations(&ai_provider_configurations)
            .map_err(|error| error.to_string())?;

        Ok(Self {
            host,
            port,
            frontend_origin,
            database_url,
            session_ttl_hours,
            intake_answer_hard_max,
            amap_webservice_key,
            amap_webservice_base_url,
            amap_timeout_ms,
            ai_provider_configurations,
        })
    }

    pub fn address(&self) -> (String, u16) {
        (self.host.clone(), self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;

    #[test]
    fn configured_address_is_returned() {
        let settings = Settings {
            host: "127.0.0.1".to_owned(),
            port: 8080,
            frontend_origin: "http://localhost:5173".to_owned(),
            database_url: "sqlite::memory:".to_owned(),
            session_ttl_hours: 8,
            intake_answer_hard_max: 2_000,
            amap_webservice_key: None,
            amap_webservice_base_url: "https://restapi.amap.com".to_owned(),
            amap_timeout_ms: 2_500,
            ai_provider_configurations: Vec::new(),
        };

        assert_eq!(settings.address(), ("127.0.0.1".to_owned(), 8080));
    }
}
