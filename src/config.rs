use std::env;
use std::{
    collections::HashSet,
    path::{Component, PathBuf},
};
use uuid::Uuid;

use crate::integrations::ai_gateway::{
    AiGateway, ProviderConfig, validate_provider_configurations,
};

/// Load a local development `.env` file without overriding process-level
/// configuration. A missing file is normal in production and CI.
pub fn load_local_env_file() -> Result<(), String> {
    match dotenvy::dotenv() {
        Ok(_) => Ok(()),
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not load local .env configuration: {error}")),
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    pub frontend_origin: String,
    pub database_url: String,
    pub session_ttl_hours: i64,
    pub intake_answer_hard_max: usize,
    pub attachment_storage_directory: PathBuf,
    pub attachment_max_image_bytes: usize,
    pub attachment_max_per_case: u64,
    pub case_place_types: Vec<String>,
    pub poi_selection_token_secret: String,
    pub amap_webservice_key: Option<String>,
    pub amap_webservice_base_url: String,
    pub amap_timeout_ms: u64,
    pub ai_provider_configurations: Vec<ProviderConfig>,
}

impl Settings {
    pub fn from_env() -> Result<Self, String> {
        load_local_env_file()?;
        Self::from_values(environment_value)
    }

    /// Parses settings from a supplied lookup so validation can be tested without
    /// mutating process-global environment variables.
    fn from_values<F>(mut value: F) -> Result<Self, String>
    where
        F: FnMut(&str) -> Result<Option<String>, String>,
    {
        let host = value("ANGUI_HOST")?.unwrap_or_else(|| "127.0.0.1".to_owned());
        let port_value = value("ANGUI_PORT")?.unwrap_or_else(|| "8080".to_owned());
        let port = port_value
            .parse::<u16>()
            .map_err(|_| format!("ANGUI_PORT must be a valid TCP port, got {port_value:?}"))?;
        let frontend_origin =
            value("ANGUI_FRONTEND_ORIGIN")?.unwrap_or_else(|| "http://localhost:5173".to_owned());
        let database_url =
            value("DATABASE_URL")?.unwrap_or_else(|| "sqlite://data/angui.db?mode=rwc".to_owned());
        let session_ttl_value = value("ANGUI_SESSION_TTL_HOURS")?.unwrap_or_else(|| "8".to_owned());
        let session_ttl_hours = session_ttl_value.parse::<i64>().map_err(|_| {
            format!("ANGUI_SESSION_TTL_HOURS must be a positive integer, got {session_ttl_value:?}")
        })?;
        if !(1..=168).contains(&session_ttl_hours) {
            return Err("ANGUI_SESSION_TTL_HOURS must be between 1 and 168".to_owned());
        }
        let intake_answer_hard_max_value =
            value("ANGUI_INTAKE_ANSWER_HARD_MAX")?.unwrap_or_else(|| "2000".to_owned());
        let intake_answer_hard_max = intake_answer_hard_max_value.parse::<usize>().map_err(|_| {
            format!(
                "ANGUI_INTAKE_ANSWER_HARD_MAX must be a positive integer, got {intake_answer_hard_max_value:?}"
            )
        })?;
        if !(1..=10_000).contains(&intake_answer_hard_max) {
            return Err("ANGUI_INTAKE_ANSWER_HARD_MAX must be between 1 and 10000".to_owned());
        }
        let attachment_storage_directory = PathBuf::from(
            value("ANGUI_ATTACHMENT_STORAGE_DIRECTORY")?
                .unwrap_or_else(|| "data/attachments".to_owned()),
        );
        if attachment_storage_directory.as_os_str().is_empty() {
            return Err("ANGUI_ATTACHMENT_STORAGE_DIRECTORY must not be empty".to_owned());
        }
        if attachment_storage_directory
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(
                "ANGUI_ATTACHMENT_STORAGE_DIRECTORY must not contain '..' path components"
                    .to_owned(),
            );
        }
        let attachment_max_image_bytes = parse_bounded_usize(
            value("ANGUI_ATTACHMENT_MAX_IMAGE_BYTES")?
                .unwrap_or_else(|| (5 * 1024 * 1024).to_string()),
            "ANGUI_ATTACHMENT_MAX_IMAGE_BYTES",
            1024,
            20 * 1024 * 1024,
        )?;
        let attachment_max_per_case = parse_bounded_u64(
            value("ANGUI_ATTACHMENT_MAX_PER_CASE")?.unwrap_or_else(|| "12".to_owned()),
            "ANGUI_ATTACHMENT_MAX_PER_CASE",
            1,
            100,
        )?;
        let case_place_types =
            parse_case_place_types(&value("ANGUI_CASE_PLACE_TYPES")?.unwrap_or_else(|| {
                "frequent,key_location,last_seen_context,medical,shelter,other".to_owned()
            }))?;
        let poi_selection_token_secret = value("ANGUI_POI_SELECTION_TOKEN_SECRET")?
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()));
        if poi_selection_token_secret.len() < 32 {
            return Err(
                "ANGUI_POI_SELECTION_TOKEN_SECRET must be at least 32 characters".to_owned(),
            );
        }
        let amap_webservice_key =
            value("AMAP_WEBSERVICE_KEY")?.filter(|value| !value.trim().is_empty());
        let amap_webservice_base_url = value("AMAP_WEBSERVICE_BASE_URL")?
            .unwrap_or_else(|| "https://restapi.amap.com".to_owned());
        if !amap_webservice_base_url.starts_with("https://") {
            return Err("AMAP_WEBSERVICE_BASE_URL must use https".to_owned());
        }
        let amap_timeout_value = value("AMAP_TIMEOUT_MS")?.unwrap_or_else(|| "2500".to_owned());
        let amap_timeout_ms = amap_timeout_value.parse::<u64>().map_err(|_| {
            format!("AMAP_TIMEOUT_MS must be a positive integer, got {amap_timeout_value:?}")
        })?;
        if !(100..=10_000).contains(&amap_timeout_ms) {
            return Err("AMAP_TIMEOUT_MS must be between 100 and 10000".to_owned());
        }
        let ai_provider_configurations = parse_ai_provider_configurations(
            value("ANGUI_AI_PROVIDERS_JSON")?.unwrap_or_else(|| "[]".to_owned()),
        )?;

        Ok(Self {
            host,
            port,
            frontend_origin,
            database_url,
            session_ttl_hours,
            intake_answer_hard_max,
            attachment_storage_directory,
            attachment_max_image_bytes,
            attachment_max_per_case,
            case_place_types,
            poi_selection_token_secret,
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

/// Validate the AI provider policy without initializing any other runtime
/// components. Preview deployment uses this before replacing an existing
/// environment so malformed policy cannot take the old preview down first.
pub fn validate_ai_provider_configurations_from_env() -> Result<(), String> {
    load_local_env_file()?;
    let value = environment_value("ANGUI_AI_PROVIDERS_JSON")?.unwrap_or_else(|| "[]".to_owned());
    let configurations = parse_ai_provider_configurations(value)?;
    let gateway =
        AiGateway::from_configurations(configurations).map_err(|error| error.to_string())?;
    gateway
        .validate_runtime_configuration()
        .map_err(|error| error.to_string())
}

fn environment_value(name: &str) -> Result<Option<String>, String> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(format!("{name} must contain valid UTF-8 text")),
    }
}

fn parse_ai_provider_configurations(value: String) -> Result<Vec<ProviderConfig>, String> {
    let configurations: Vec<ProviderConfig> = serde_json::from_str(&value).map_err(|error| {
        format!("ANGUI_AI_PROVIDERS_JSON must be a JSON array of provider configurations: {error}")
    })?;
    validate_provider_configurations(&configurations).map_err(|error| error.to_string())?;
    Ok(configurations)
}

fn parse_bounded_usize(
    value: String,
    name: &str,
    minimum: usize,
    maximum: usize,
) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive integer, got {value:?}"))?;
    if !(minimum..=maximum).contains(&parsed) {
        return Err(format!("{name} must be between {minimum} and {maximum}"));
    }
    Ok(parsed)
}

fn parse_bounded_u64(value: String, name: &str, minimum: u64, maximum: u64) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a positive integer, got {value:?}"))?;
    if !(minimum..=maximum).contains(&parsed) {
        return Err(format!("{name} must be between {minimum} and {maximum}"));
    }
    Ok(parsed)
}

fn parse_case_place_types(value: &str) -> Result<Vec<String>, String> {
    let place_types: Vec<_> = value
        .split(',')
        .map(str::trim)
        .filter(|place_type| !place_type.is_empty())
        .map(str::to_lowercase)
        .collect();
    if place_types.is_empty() || place_types.len() > 16 {
        return Err("ANGUI_CASE_PLACE_TYPES must contain between 1 and 16 values".to_owned());
    }
    if place_types.iter().any(|place_type| {
        place_type.len() > 64
            || !place_type.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
            })
    }) {
        return Err("ANGUI_CASE_PLACE_TYPES values must use lowercase letters, digits, or underscores and be at most 64 characters".to_owned());
    }
    let unique: HashSet<_> = place_types.iter().collect();
    if unique.len() != place_types.len() {
        return Err("ANGUI_CASE_PLACE_TYPES must not contain duplicates".to_owned());
    }
    Ok(place_types)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use super::Settings;

    fn settings_with(values: &[(&str, &str)]) -> Result<Settings, String> {
        let values: HashMap<_, _> = values
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        Settings::from_values(|name| Ok(values.get(name).cloned()))
    }

    #[test]
    fn defaults_are_safe_for_local_development() {
        let settings = settings_with(&[]).expect("default settings should be valid");

        assert_eq!(settings.host, "127.0.0.1");
        assert_eq!(settings.port, 8080);
        assert_eq!(settings.frontend_origin, "http://localhost:5173");
        assert_eq!(settings.database_url, "sqlite://data/angui.db?mode=rwc");
        assert_eq!(settings.session_ttl_hours, 8);
        assert_eq!(settings.intake_answer_hard_max, 2_000);
        assert_eq!(
            settings.attachment_storage_directory,
            PathBuf::from("data/attachments")
        );
        assert_eq!(settings.attachment_max_image_bytes, 5 * 1024 * 1024);
        assert_eq!(settings.attachment_max_per_case, 12);
        assert_eq!(
            settings.case_place_types,
            vec![
                "frequent",
                "key_location",
                "last_seen_context",
                "medical",
                "shelter",
                "other"
            ]
        );
        assert_eq!(settings.amap_webservice_key, None);
        assert!(settings.poi_selection_token_secret.len() >= 32);
        assert_eq!(
            settings.amap_webservice_base_url,
            "https://restapi.amap.com"
        );
        assert_eq!(settings.amap_timeout_ms, 2_500);
        assert!(settings.ai_provider_configurations.is_empty());
    }

    #[test]
    fn valid_overrides_are_loaded_without_process_environment_mutation() {
        let settings = settings_with(&[
            ("ANGUI_HOST", "0.0.0.0"),
            ("ANGUI_PORT", "9090"),
            ("ANGUI_FRONTEND_ORIGIN", "https://family.example.invalid"),
            ("DATABASE_URL", "sqlite::memory:"),
            ("ANGUI_SESSION_TTL_HOURS", "168"),
            ("ANGUI_INTAKE_ANSWER_HARD_MAX", "10000"),
            ("ANGUI_ATTACHMENT_STORAGE_DIRECTORY", "private/case-media"),
            ("ANGUI_ATTACHMENT_MAX_IMAGE_BYTES", "10485760"),
            ("ANGUI_ATTACHMENT_MAX_PER_CASE", "24"),
            ("ANGUI_CASE_PLACE_TYPES", "frequent,station,clinic"),
            (
                "ANGUI_POI_SELECTION_TOKEN_SECRET",
                "0123456789abcdef0123456789abcdef",
            ),
            ("AMAP_WEBSERVICE_KEY", "test-key"),
            ("AMAP_WEBSERVICE_BASE_URL", "https://maps.example.invalid"),
            ("AMAP_TIMEOUT_MS", "10000"),
            ("ANGUI_AI_PROVIDERS_JSON", "[]"),
        ])
        .expect("valid overrides should be accepted");

        assert_eq!(settings.address(), ("0.0.0.0".to_owned(), 9090));
        assert_eq!(
            settings.attachment_storage_directory,
            PathBuf::from("private/case-media")
        );
        assert_eq!(settings.amap_webservice_key.as_deref(), Some("test-key"));
        assert_eq!(
            settings.poi_selection_token_secret,
            "0123456789abcdef0123456789abcdef"
        );
        assert_eq!(settings.attachment_max_image_bytes, 10 * 1024 * 1024);
        assert_eq!(settings.attachment_max_per_case, 24);
        assert_eq!(
            settings.case_place_types,
            vec!["frequent", "station", "clinic"]
        );
        assert_eq!(
            settings.amap_webservice_base_url,
            "https://maps.example.invalid"
        );
        assert_eq!(settings.amap_timeout_ms, 10_000);
    }

    #[test]
    fn blank_amap_key_is_treated_as_not_configured() {
        let settings = settings_with(&[("AMAP_WEBSERVICE_KEY", " \t ")])
            .expect("a blank optional key should not make settings invalid");

        assert_eq!(settings.amap_webservice_key, None);
    }

    #[test]
    fn environment_lookup_errors_are_not_treated_as_missing_values() {
        let error = Settings::from_values(|name| {
            if name == "ANGUI_HOST" {
                Err("ANGUI_HOST must contain valid UTF-8 text".to_owned())
            } else {
                Ok(None)
            }
        })
        .expect_err("environment lookup failures must be propagated");

        assert_eq!(error, "ANGUI_HOST must contain valid UTF-8 text");
    }

    #[test]
    fn numeric_configuration_rejects_malformed_and_out_of_range_values() {
        for (name, value, expected_error) in [
            (
                "ANGUI_PORT",
                "not-a-port",
                "ANGUI_PORT must be a valid TCP port",
            ),
            (
                "ANGUI_SESSION_TTL_HOURS",
                "zero",
                "must be a positive integer",
            ),
            ("ANGUI_SESSION_TTL_HOURS", "0", "must be between 1 and 168"),
            (
                "ANGUI_SESSION_TTL_HOURS",
                "169",
                "must be between 1 and 168",
            ),
            (
                "ANGUI_INTAKE_ANSWER_HARD_MAX",
                "zero",
                "must be a positive integer",
            ),
            (
                "ANGUI_INTAKE_ANSWER_HARD_MAX",
                "0",
                "must be between 1 and 10000",
            ),
            (
                "ANGUI_INTAKE_ANSWER_HARD_MAX",
                "10001",
                "must be between 1 and 10000",
            ),
            ("AMAP_TIMEOUT_MS", "slow", "must be a positive integer"),
            ("AMAP_TIMEOUT_MS", "99", "must be between 100 and 10000"),
            ("AMAP_TIMEOUT_MS", "10001", "must be between 100 and 10000"),
            (
                "ANGUI_POI_SELECTION_TOKEN_SECRET",
                "too-short",
                "must be at least 32 characters",
            ),
            (
                "ANGUI_ATTACHMENT_MAX_IMAGE_BYTES",
                "zero",
                "must be a positive integer",
            ),
            (
                "ANGUI_ATTACHMENT_MAX_IMAGE_BYTES",
                "1023",
                "must be between 1024 and 20971520",
            ),
            (
                "ANGUI_ATTACHMENT_MAX_IMAGE_BYTES",
                "20971521",
                "must be between 1024 and 20971520",
            ),
            (
                "ANGUI_ATTACHMENT_MAX_PER_CASE",
                "zero",
                "must be a positive integer",
            ),
            (
                "ANGUI_ATTACHMENT_MAX_PER_CASE",
                "0",
                "must be between 1 and 100",
            ),
            (
                "ANGUI_ATTACHMENT_MAX_PER_CASE",
                "101",
                "must be between 1 and 100",
            ),
        ] {
            let error = settings_with(&[(name, value)]).expect_err("invalid value must fail");
            assert!(
                error.contains(expected_error),
                "{name}={value:?} returned {error:?}"
            );
        }
    }

    #[test]
    fn security_and_storage_configuration_are_validated() {
        let insecure_map =
            settings_with(&[("AMAP_WEBSERVICE_BASE_URL", "http://maps.example.invalid")])
                .expect_err("AMap must use HTTPS");
        assert_eq!(insecure_map, "AMAP_WEBSERVICE_BASE_URL must use https");

        let empty_attachment_directory =
            settings_with(&[("ANGUI_ATTACHMENT_STORAGE_DIRECTORY", "")])
                .expect_err("an empty attachment directory must be rejected");
        assert_eq!(
            empty_attachment_directory,
            "ANGUI_ATTACHMENT_STORAGE_DIRECTORY must not be empty"
        );

        let parent_attachment_directory = settings_with(&[(
            "ANGUI_ATTACHMENT_STORAGE_DIRECTORY",
            "private/../attachments",
        )])
        .expect_err("parent-directory components must be rejected");
        assert_eq!(
            parent_attachment_directory,
            "ANGUI_ATTACHMENT_STORAGE_DIRECTORY must not contain '..' path components"
        );

        let malformed_providers = settings_with(&[("ANGUI_AI_PROVIDERS_JSON", "{")])
            .expect_err("provider configuration must be JSON");
        assert!(malformed_providers.starts_with("ANGUI_AI_PROVIDERS_JSON must be a JSON array"));

        for invalid_place_types in ["", "frequent, frequent", "frequent,invalid-type"] {
            let error = settings_with(&[("ANGUI_CASE_PLACE_TYPES", invalid_place_types)])
                .expect_err("invalid place type configuration must fail");
            assert!(error.starts_with("ANGUI_CASE_PLACE_TYPES"));
        }
    }

    #[test]
    fn configured_address_is_returned() {
        let settings = Settings {
            host: "127.0.0.1".to_owned(),
            port: 8080,
            frontend_origin: "http://localhost:5173".to_owned(),
            database_url: "sqlite::memory:".to_owned(),
            session_ttl_hours: 8,
            intake_answer_hard_max: 2_000,
            attachment_storage_directory: PathBuf::from("data/attachments"),
            attachment_max_image_bytes: 5 * 1024 * 1024,
            attachment_max_per_case: 12,
            case_place_types: vec!["frequent".to_owned()],
            poi_selection_token_secret: "0123456789abcdef0123456789abcdef".to_owned(),
            amap_webservice_key: None,
            amap_webservice_base_url: "https://restapi.amap.com".to_owned(),
            amap_timeout_ms: 2_500,
            ai_provider_configurations: Vec::new(),
        };

        assert_eq!(settings.address(), ("127.0.0.1".to_owned(), 8080));
    }
}
