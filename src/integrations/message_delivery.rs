use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Serialize;
use std::env;

#[derive(Clone, Debug)]
pub struct MessageDelivery {
    config: Option<SmtpConfig>,
}

#[derive(Clone, Debug)]
struct SmtpConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
    from: String,
    from_name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeliveryReceipt {
    pub status: String,
    pub reason: Option<String>,
}

impl MessageDelivery {
    pub fn from_env() -> Result<Self, String> {
        let host = env::var("ANGUI_EMAIL_SMTP_HOST")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let Some(host) = host else {
            return Ok(Self { config: None });
        };
        let port = env::var("ANGUI_EMAIL_SMTP_PORT")
            .unwrap_or_else(|_| "587".to_owned())
            .parse()
            .map_err(|_| "ANGUI_EMAIL_SMTP_PORT must be a valid port".to_owned())?;
        let username = env::var("ANGUI_EMAIL_SMTP_USERNAME").map_err(|_| {
            "ANGUI_EMAIL_SMTP_USERNAME is required when SMTP is configured".to_owned()
        })?;
        let password = env::var("ANGUI_EMAIL_SMTP_PASSWORD").map_err(|_| {
            "ANGUI_EMAIL_SMTP_PASSWORD is required when SMTP is configured".to_owned()
        })?;
        let from = env::var("ANGUI_EMAIL_SMTP_FROM_ADDRESS").map_err(|_| {
            "ANGUI_EMAIL_SMTP_FROM_ADDRESS is required when SMTP is configured".to_owned()
        })?;
        let from_name =
            env::var("ANGUI_EMAIL_SMTP_FROM_NAME").unwrap_or_else(|_| "安归".to_owned());
        Ok(Self {
            config: Some(SmtpConfig {
                host,
                port,
                username,
                password,
                from,
                from_name,
            }),
        })
    }
    pub fn disabled() -> Self {
        Self { config: None }
    }
    pub async fn send(&self, to: &str, subject: &str, body: &str) -> DeliveryReceipt {
        let Some(config) = self.config.as_ref() else {
            return DeliveryReceipt {
                status: "not_configured".to_owned(),
                reason: None,
            };
        };
        let from = match format!("{} <{}>", config.from_name, config.from).parse() {
            Ok(v) => v,
            Err(_) => {
                return DeliveryReceipt {
                    status: "failed".to_owned(),
                    reason: Some("invalid sender configuration".to_owned()),
                };
            }
        };
        let recipient = match to.parse() {
            Ok(v) => v,
            Err(_) => {
                return DeliveryReceipt {
                    status: "failed".to_owned(),
                    reason: Some("invalid recipient".to_owned()),
                };
            }
        };
        let message = match Message::builder()
            .from(from)
            .to(recipient)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_owned())
        {
            Ok(v) => v,
            Err(_) => {
                return DeliveryReceipt {
                    status: "failed".to_owned(),
                    reason: Some("message construction failed".to_owned()),
                };
            }
        };
        let transport = match AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host) {
            Ok(builder) => builder
                .port(config.port)
                .credentials(Credentials::new(
                    config.username.clone(),
                    config.password.clone(),
                ))
                .build(),
            Err(_) => {
                return DeliveryReceipt {
                    status: "failed".to_owned(),
                    reason: Some("smtp transport unavailable".to_owned()),
                };
            }
        };
        match transport.send(message).await {
            Ok(_) => DeliveryReceipt {
                status: "delivered".to_owned(),
                reason: None,
            },
            Err(_) => DeliveryReceipt {
                status: "failed".to_owned(),
                reason: Some("smtp delivery failed".to_owned()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MessageDelivery;

    #[tokio::test]
    async fn disabled_delivery_does_not_attempt_an_external_transport() {
        let receipt = MessageDelivery::disabled()
            .send("test@example.invalid", "subject", "body")
            .await;
        assert_eq!(receipt.status, "not_configured");
        assert_eq!(receipt.reason, None);
    }
}
