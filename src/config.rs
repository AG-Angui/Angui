use std::env;

#[derive(Clone, Debug)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    pub frontend_origin: String,
    pub database_url: String,
    pub session_ttl_hours: i64,
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

        Ok(Self {
            host,
            port,
            frontend_origin,
            database_url,
            session_ttl_hours,
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
        };

        assert_eq!(settings.address(), ("127.0.0.1".to_owned(), 8080));
    }
}
