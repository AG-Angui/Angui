use std::env;

#[derive(Clone, Debug)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    pub frontend_origin: String,
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

        Ok(Self {
            host,
            port,
            frontend_origin,
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
        };

        assert_eq!(settings.address(), ("127.0.0.1".to_owned(), 8080));
    }
}
