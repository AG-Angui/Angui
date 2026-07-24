use std::{collections::HashSet, sync::Arc};

use chrono::{SecondsFormat, Utc};
use http::Request as HttpRequest;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::entities::audit_events;

/// The only model-provider protocols supported by the gateway. Business code
/// uses `AiRequest`, so it never needs a provider SDK or protocol payload.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderProtocol {
    OpenAiChatCompletions,
    OpenAiResponses,
    AnthropicMessages,
    GeminiGenerateContent,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiCapability {
    Inquiry,
    StructuredExtraction,
    CaseSummary,
    KnowledgeAnswer,
    CaseOrganization,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataLevel {
    Public,
    Collaborative,
    Internal,
    Sensitive,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiPurpose {
    IntakeDraft,
    ClueDraft,
    CaseSummaryDraft,
    KnowledgeAnswer,
    CaseArchiveDraft,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    pub id: String,
    pub vendor: String,
    pub protocol: ProviderProtocol,
    pub region: String,
    pub model: String,
    pub capabilities: Vec<AiCapability>,
    pub allowed_data_levels: Vec<DataLevel>,
    pub allowed_purposes: Vec<AiPurpose>,
    pub input_limit_chars: usize,
    pub output_limit_tokens: usize,
    pub timeout_ms: u64,
    pub allow_fallback: bool,
    pub priority: u16,
    pub weight: u16,
    pub emergency_disabled: bool,
    pub compliance_scopes: Vec<String>,
    /// Name of an environment variable containing the provider base URL.
    /// The URL itself is deliberately not committed in provider configuration.
    pub endpoint_env: String,
    /// Name of an environment variable containing the provider credential.
    /// This configuration contains a reference only, never a secret value.
    pub credential_env: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AiRequest {
    pub capability: AiCapability,
    pub data_level: DataLevel,
    pub purpose: AiPurpose,
    pub data_region: String,
    pub system_instruction: Option<String>,
    pub input: String,
    pub requested_output_tokens: usize,
    pub template_version: String,
    pub input_scope_reference: String,
    pub redaction_policy_version: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProviderRoute {
    pub provider_id: String,
    pub vendor: String,
    pub protocol: ProviderProtocol,
    pub model: String,
    pub endpoint_env: String,
    pub credential_env: String,
    pub timeout_ms: u64,
    pub allow_fallback: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DegradationStatus {
    RuleBased,
    ManualRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DegradationReason {
    NoProviderConfigured,
    NoCompliantProvider,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GatewayDecision {
    Routed(ProviderRoute),
    Degraded {
        status: DegradationStatus,
        reason: DegradationReason,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProviderHttpRequest {
    pub method: &'static str,
    pub path: String,
    pub headers: Vec<ProviderHeader>,
    pub body: Value,
}

impl ProviderHttpRequest {
    /// Materializes a protocol request only at the transport boundary. The
    /// endpoint and credential come from the runtime environment, not the
    /// checked-in provider configuration or audit metadata.
    pub fn into_native_request(
        self,
        endpoint: &str,
        credential: &str,
    ) -> Result<HttpRequest<Value>, AiGatewayError> {
        let endpoint = endpoint.trim_end_matches('/');
        if endpoint.is_empty() || credential.is_empty() {
            return Err(AiGatewayError::InvalidNativeRequest(
                "endpoint and credential must be supplied by the runtime environment".to_owned(),
            ));
        }
        let uri = format!("{endpoint}{}", self.path)
            .parse::<http::Uri>()
            .map_err(|_| {
                AiGatewayError::InvalidNativeRequest("invalid provider endpoint".to_owned())
            })?;
        let mut builder = HttpRequest::builder()
            .method(self.method)
            .uri(uri)
            .header("content-type", "application/json");
        for header in self.headers {
            let value = match header.value {
                HeaderValue::Literal(value) => value.to_owned(),
                HeaderValue::EnvironmentSecret(_) if header.name == "authorization" => {
                    format!("Bearer {credential}")
                }
                HeaderValue::EnvironmentSecret(_) => credential.to_owned(),
            };
            builder = builder.header(header.name, value);
        }
        builder.body(self.body).map_err(|_| {
            AiGatewayError::InvalidNativeRequest("invalid provider request headers".to_owned())
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProviderHeader {
    pub name: &'static str,
    pub value: HeaderValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HeaderValue {
    Literal(&'static str),
    EnvironmentSecret(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiTaskStatus {
    Routed,
    Completed,
    Degraded,
    Failed,
}

impl AiTaskStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Routed => "routed",
            Self::Completed => "completed",
            Self::Degraded => "degraded",
            Self::Failed => "failed",
        }
    }
}

/// Metadata safe for the existing audit event store. It intentionally has no
/// raw prompt, response, medical history, contact details, or location data.
#[derive(Clone, Debug, PartialEq)]
pub struct AiExecutionAudit {
    pub id: String,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub template_version: String,
    pub input_scope_reference: String,
    pub input_hash: String,
    pub redaction_policy_version: String,
    pub status: AiTaskStatus,
}

impl AiExecutionAudit {
    pub fn for_request(
        request: &AiRequest,
        decision: &GatewayDecision,
        status: AiTaskStatus,
    ) -> Self {
        let (provider_id, model) = match decision {
            GatewayDecision::Routed(route) => {
                (Some(route.provider_id.clone()), Some(route.model.clone()))
            }
            GatewayDecision::Degraded { .. } => (None, None),
        };

        Self {
            id: Uuid::new_v4().to_string(),
            provider_id,
            model,
            template_version: request.template_version.clone(),
            input_scope_reference: request.input_scope_reference.clone(),
            input_hash: hex::encode(Sha256::digest(request.input.as_bytes())),
            redaction_policy_version: request.redaction_policy_version.clone(),
            status,
        }
    }

    pub fn metadata_json(&self) -> Value {
        json!({
            "provider_id": self.provider_id,
            "model": self.model,
            "template_version": self.template_version,
            "input_scope_reference": self.input_scope_reference,
            "input_hash": self.input_hash,
            "redaction_policy_version": self.redaction_policy_version,
            "status": self.status.as_str(),
        })
    }
}

pub async fn persist_execution_audit<C: ConnectionTrait>(
    db: &C,
    audit: &AiExecutionAudit,
    actor: &str,
    case_id: Option<&str>,
) -> Result<(), sea_orm::DbErr> {
    audit_events::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        case_id: Set(case_id.map(str::to_owned)),
        actor: Set(actor.to_owned()),
        action: Set("ai.execution".to_owned()),
        entity_type: Set("ai_execution".to_owned()),
        entity_id: Set(audit.id.clone()),
        metadata_json: Set(Some(audit.metadata_json().to_string())),
        created_at: Set(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
    }
    .insert(db)
    .await?;
    Ok(())
}

#[derive(Debug, Error, PartialEq)]
pub enum AiGatewayError {
    #[error("invalid AI provider configuration: {0}")]
    InvalidConfiguration(String),
    #[error("request must not be empty")]
    EmptyRequest,
    #[error("invalid native AI request: {0}")]
    InvalidNativeRequest(String),
}

/// Provider adapters only produce protocol requests. A later transport layer
/// resolves the two environment references and performs the network call.
pub trait AiProvider: Send + Sync {
    fn route(&self) -> ProviderRoute;
    fn build_request(&self, request: &AiRequest) -> Result<ProviderHttpRequest, AiGatewayError>;
}

#[derive(Clone)]
struct ProtocolProvider {
    config: ProviderConfig,
}

impl AiProvider for ProtocolProvider {
    fn route(&self) -> ProviderRoute {
        route_for(&self.config)
    }

    fn build_request(&self, request: &AiRequest) -> Result<ProviderHttpRequest, AiGatewayError> {
        if request.input.trim().is_empty() {
            return Err(AiGatewayError::EmptyRequest);
        }

        let instructions = request.system_instruction.as_deref().unwrap_or_default();
        let max_tokens = request
            .requested_output_tokens
            .min(self.config.output_limit_tokens);
        let credential = HeaderValue::EnvironmentSecret(self.config.credential_env.clone());
        let request = match self.config.protocol {
            ProviderProtocol::OpenAiChatCompletions => ProviderHttpRequest {
                method: "POST",
                path: "/v1/chat/completions".to_owned(),
                headers: vec![ProviderHeader {
                    name: "authorization",
                    value: credential,
                }],
                body: json!({
                    "model": self.config.model,
                    "messages": openai_messages(instructions, request.input.as_str()),
                    "max_tokens": max_tokens,
                }),
            },
            ProviderProtocol::OpenAiResponses => ProviderHttpRequest {
                method: "POST",
                path: "/v1/responses".to_owned(),
                headers: vec![ProviderHeader {
                    name: "authorization",
                    value: credential,
                }],
                body: json!({
                    "model": self.config.model,
                    "input": openai_messages(instructions, request.input.as_str()),
                    "max_output_tokens": max_tokens,
                }),
            },
            ProviderProtocol::AnthropicMessages => ProviderHttpRequest {
                method: "POST",
                path: "/v1/messages".to_owned(),
                headers: vec![
                    ProviderHeader {
                        name: "x-api-key",
                        value: credential,
                    },
                    ProviderHeader {
                        name: "anthropic-version",
                        value: HeaderValue::Literal("2023-06-01"),
                    },
                ],
                body: json!({
                    "model": self.config.model,
                    "system": instructions,
                    "messages": [{ "role": "user", "content": request.input.as_str() }],
                    "max_tokens": max_tokens,
                }),
            },
            ProviderProtocol::GeminiGenerateContent => {
                let mut body = json!({
                    "contents": [{
                        "role": "user",
                        "parts": [{ "text": request.input.as_str() }],
                    }],
                    "generationConfig": { "maxOutputTokens": max_tokens },
                });
                if !instructions.is_empty() {
                    body["systemInstruction"] = json!({ "parts": [{ "text": instructions }] });
                }
                ProviderHttpRequest {
                    method: "POST",
                    path: format!("/v1beta/models/{}:generateContent", self.config.model),
                    headers: vec![ProviderHeader {
                        name: "x-goog-api-key",
                        value: credential,
                    }],
                    body,
                }
            }
        };
        Ok(request)
    }
}

fn openai_messages(instructions: &str, input: &str) -> Vec<Value> {
    let mut messages = Vec::new();
    if !instructions.is_empty() {
        messages.push(json!({ "role": "system", "content": instructions }));
    }
    messages.push(json!({ "role": "user", "content": input }));
    messages
}

#[derive(Clone)]
pub struct AiGateway {
    providers: Vec<RegisteredProvider>,
}

#[derive(Clone)]
struct RegisteredProvider {
    config: ProviderConfig,
    provider: Arc<dyn AiProvider>,
}

impl AiGateway {
    pub fn from_configurations(
        configurations: Vec<ProviderConfig>,
    ) -> Result<Self, AiGatewayError> {
        validate_provider_configurations(&configurations)?;
        let providers = configurations
            .into_iter()
            .map(|config| RegisteredProvider {
                provider: Arc::new(ProtocolProvider {
                    config: config.clone(),
                }) as Arc<dyn AiProvider>,
                config,
            })
            .collect();
        Ok(Self { providers })
    }

    pub fn route(&self, request: &AiRequest) -> GatewayDecision {
        let mut candidates: Vec<_> = self
            .providers
            .iter()
            .filter_map(|registered| {
                let route = registered.provider.route();
                provider_is_eligible(&registered.config, request).then_some((
                    route,
                    registered.config.priority,
                    registered.config.weight,
                ))
            })
            .collect();

        candidates.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| left.0.provider_id.cmp(&right.0.provider_id))
        });

        candidates
            .into_iter()
            .next()
            .map(|(route, _, _)| GatewayDecision::Routed(route))
            .unwrap_or_else(|| {
                if self.providers.is_empty() {
                    GatewayDecision::Degraded {
                        status: DegradationStatus::RuleBased,
                        reason: DegradationReason::NoProviderConfigured,
                    }
                } else {
                    GatewayDecision::Degraded {
                        status: DegradationStatus::ManualRequired,
                        reason: DegradationReason::NoCompliantProvider,
                    }
                }
            })
    }

    pub fn build_provider_request(
        &self,
        route: &ProviderRoute,
        request: &AiRequest,
    ) -> Result<ProviderHttpRequest, AiGatewayError> {
        let registered = self
            .providers
            .iter()
            .find(|registered| registered.provider.route().provider_id == route.provider_id)
            .ok_or_else(|| {
                AiGatewayError::InvalidConfiguration(
                    "selected provider is not registered".to_owned(),
                )
            })?;
        if registered.provider.route() != *route {
            return Err(AiGatewayError::InvalidConfiguration(
                "selected provider route does not match its registered configuration".to_owned(),
            ));
        }
        if !provider_is_eligible(&registered.config, request) {
            return Err(AiGatewayError::InvalidConfiguration(
                "selected provider does not satisfy the request policy".to_owned(),
            ));
        }
        registered.provider.build_request(request)
    }

    pub fn build_native_request(
        &self,
        route: &ProviderRoute,
        request: &AiRequest,
        endpoint: &str,
        credential: &str,
    ) -> Result<HttpRequest<Value>, AiGatewayError> {
        self.build_provider_request(route, request)?
            .into_native_request(endpoint, credential)
    }
}

fn route_for(config: &ProviderConfig) -> ProviderRoute {
    ProviderRoute {
        provider_id: config.id.clone(),
        vendor: config.vendor.clone(),
        protocol: config.protocol,
        model: config.model.clone(),
        endpoint_env: config.endpoint_env.clone(),
        credential_env: config.credential_env.clone(),
        timeout_ms: config.timeout_ms,
        allow_fallback: config.allow_fallback,
    }
}

fn provider_is_eligible(config: &ProviderConfig, request: &AiRequest) -> bool {
    !config.emergency_disabled
        && config.capabilities.contains(&request.capability)
        && config.allowed_data_levels.contains(&request.data_level)
        && config.allowed_purposes.contains(&request.purpose)
        && config.region == request.data_region
        && !request.input.trim().is_empty()
        && request.input.chars().count() <= config.input_limit_chars
        && request.requested_output_tokens <= config.output_limit_tokens
}

pub fn validate_provider_configurations(
    configurations: &[ProviderConfig],
) -> Result<(), AiGatewayError> {
    let mut identifiers = HashSet::new();
    for config in configurations {
        let invalid_text = [
            ("id", config.id.as_str()),
            ("vendor", config.vendor.as_str()),
            ("region", config.region.as_str()),
            ("model", config.model.as_str()),
        ]
        .into_iter()
        .find(|(_, value)| value.trim().is_empty());
        if let Some((field, _)) = invalid_text {
            return Err(AiGatewayError::InvalidConfiguration(format!(
                "provider {} must not be empty",
                field
            )));
        }
        if !identifiers.insert(config.id.as_str()) {
            return Err(AiGatewayError::InvalidConfiguration(format!(
                "provider id {:?} is duplicated",
                config.id
            )));
        }
        if config.capabilities.is_empty()
            || config.allowed_data_levels.is_empty()
            || config.allowed_purposes.is_empty()
        {
            return Err(AiGatewayError::InvalidConfiguration(format!(
                "provider {:?} must declare capabilities and data policy",
                config.id
            )));
        }
        if config.input_limit_chars == 0
            || config.output_limit_tokens == 0
            || config.timeout_ms == 0
        {
            return Err(AiGatewayError::InvalidConfiguration(format!(
                "provider {:?} must declare positive input, output, and timeout limits",
                config.id
            )));
        }
        if config.weight == 0 {
            return Err(AiGatewayError::InvalidConfiguration(format!(
                "provider {:?} must have a non-zero weight",
                config.id
            )));
        }
        if config.allowed_data_levels.contains(&DataLevel::Sensitive)
            && config
                .compliance_scopes
                .iter()
                .all(|scope| scope.trim().is_empty())
        {
            return Err(AiGatewayError::InvalidConfiguration(format!(
                "provider {:?} permits sensitive data without a compliance scope",
                config.id
            )));
        }
        for (field, value) in [
            ("endpoint_env", config.endpoint_env.as_str()),
            ("credential_env", config.credential_env.as_str()),
        ] {
            if !is_environment_variable_name(value) {
                return Err(AiGatewayError::InvalidConfiguration(format!(
                    "provider {:?} has an invalid {} reference",
                    config.id, field
                )));
            }
        }
    }
    Ok(())
}

fn is_environment_variable_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte == b'_' || byte.is_ascii_uppercase() || byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(id: &str, protocol: ProviderProtocol) -> ProviderConfig {
        ProviderConfig {
            id: id.to_owned(),
            vendor: "test-vendor".to_owned(),
            protocol,
            region: "cn-test-1".to_owned(),
            model: "test-model".to_owned(),
            capabilities: vec![AiCapability::Inquiry],
            allowed_data_levels: vec![DataLevel::Internal],
            allowed_purposes: vec![AiPurpose::IntakeDraft],
            input_limit_chars: 1_000,
            output_limit_tokens: 128,
            timeout_ms: 5_000,
            allow_fallback: false,
            priority: 10,
            weight: 1,
            emergency_disabled: false,
            compliance_scopes: Vec::new(),
            endpoint_env: "ANGUI_TEST_AI_ENDPOINT".to_owned(),
            credential_env: "ANGUI_TEST_AI_KEY".to_owned(),
        }
    }

    fn request() -> AiRequest {
        AiRequest {
            capability: AiCapability::Inquiry,
            data_level: DataLevel::Internal,
            purpose: AiPurpose::IntakeDraft,
            data_region: "cn-test-1".to_owned(),
            system_instruction: Some("Follow the approved template.".to_owned()),
            input: "A simulated intake answer".to_owned(),
            requested_output_tokens: 100,
            template_version: "intake-v1".to_owned(),
            input_scope_reference: "intake-session:simulated".to_owned(),
            redaction_policy_version: "redaction-v1".to_owned(),
        }
    }

    #[test]
    fn routes_only_to_an_enabled_compliant_provider() {
        let mut wrong_region = provider("wrong-region", ProviderProtocol::OpenAiResponses);
        wrong_region.priority = 100;
        wrong_region.region = "us-test-1".to_owned();
        let mut disabled = provider("disabled", ProviderProtocol::AnthropicMessages);
        disabled.priority = 90;
        disabled.emergency_disabled = true;
        let eligible = provider("eligible", ProviderProtocol::GeminiGenerateContent);
        let gateway = AiGateway::from_configurations(vec![wrong_region, disabled, eligible])
            .expect("fixtures are valid");

        assert_eq!(
            gateway.route(&request()),
            GatewayDecision::Routed(ProviderRoute {
                provider_id: "eligible".to_owned(),
                vendor: "test-vendor".to_owned(),
                protocol: ProviderProtocol::GeminiGenerateContent,
                model: "test-model".to_owned(),
                endpoint_env: "ANGUI_TEST_AI_ENDPOINT".to_owned(),
                credential_env: "ANGUI_TEST_AI_KEY".to_owned(),
                timeout_ms: 5_000,
                allow_fallback: false,
            })
        );
    }

    #[test]
    fn no_provider_configuration_uses_rule_based_degradation() {
        let gateway =
            AiGateway::from_configurations(Vec::new()).expect("empty configuration is valid");

        assert_eq!(
            gateway.route(&request()),
            GatewayDecision::Degraded {
                status: DegradationStatus::RuleBased,
                reason: DegradationReason::NoProviderConfigured,
            }
        );
    }

    #[test]
    fn empty_input_degrades_before_selecting_a_provider() {
        let gateway = AiGateway::from_configurations(vec![provider(
            "eligible",
            ProviderProtocol::GeminiGenerateContent,
        )])
        .expect("fixture is valid");
        let mut request = request();
        request.input = "  \n\t ".to_owned();

        assert!(matches!(
            gateway.route(&request),
            GatewayDecision::Degraded { .. }
        ));
    }

    #[test]
    fn validation_rejects_sensitive_provider_without_compliance_scope() {
        let mut configuration = provider("sensitive", ProviderProtocol::OpenAiChatCompletions);
        configuration.allowed_data_levels.push(DataLevel::Sensitive);

        assert!(matches!(
            validate_provider_configurations(&[configuration]),
            Err(AiGatewayError::InvalidConfiguration(message))
                if message.contains("without a compliance scope")
        ));
    }

    #[test]
    fn protocol_adapters_generate_the_supported_wire_formats_without_secrets() {
        for protocol in [
            ProviderProtocol::OpenAiChatCompletions,
            ProviderProtocol::OpenAiResponses,
            ProviderProtocol::AnthropicMessages,
            ProviderProtocol::GeminiGenerateContent,
        ] {
            let gateway = AiGateway::from_configurations(vec![provider("format", protocol)])
                .expect("fixture is valid");
            let route = match gateway.route(&request()) {
                GatewayDecision::Routed(route) => route,
                GatewayDecision::Degraded { .. } => panic!("fixture should route"),
            };
            let outbound = gateway
                .build_provider_request(&route, &request())
                .expect("request should be serializable");

            assert_eq!(outbound.method, "POST");
            assert!(outbound
                .headers
                .iter()
                .any(|header| matches!(&header.value, HeaderValue::EnvironmentSecret(name) if name == "ANGUI_TEST_AI_KEY")));
            assert!(!outbound.body.to_string().contains("ANGUI_TEST_AI_KEY"));
            match protocol {
                ProviderProtocol::OpenAiChatCompletions => {
                    assert_eq!(outbound.path, "/v1/chat/completions");
                    assert!(outbound.body.get("messages").is_some());
                }
                ProviderProtocol::OpenAiResponses => {
                    assert_eq!(outbound.path, "/v1/responses");
                    assert!(outbound.body.get("input").is_some());
                }
                ProviderProtocol::AnthropicMessages => {
                    assert_eq!(outbound.path, "/v1/messages");
                    assert!(outbound.body.get("system").is_some());
                }
                ProviderProtocol::GeminiGenerateContent => {
                    assert_eq!(outbound.path, "/v1beta/models/test-model:generateContent");
                    assert!(outbound.body.get("systemInstruction").is_some());
                }
            }
        }
    }

    #[test]
    fn gemini_omits_system_instruction_when_it_is_absent_or_empty() {
        let gateway = AiGateway::from_configurations(vec![provider(
            "gemini",
            ProviderProtocol::GeminiGenerateContent,
        )])
        .expect("fixture is valid");
        let route = match gateway.route(&request()) {
            GatewayDecision::Routed(route) => route,
            GatewayDecision::Degraded { .. } => panic!("fixture should route"),
        };

        for system_instruction in [None, Some(String::new())] {
            let mut request = request();
            request.system_instruction = system_instruction;
            let outbound = gateway
                .build_provider_request(&route, &request)
                .expect("request should be serializable");

            assert!(outbound.body.get("systemInstruction").is_none());
        }
    }

    #[test]
    fn native_request_materializes_credentials_only_at_the_transport_boundary() {
        let gateway = AiGateway::from_configurations(vec![provider(
            "native",
            ProviderProtocol::OpenAiResponses,
        )])
        .expect("fixture is valid");
        let route = match gateway.route(&request()) {
            GatewayDecision::Routed(route) => route,
            GatewayDecision::Degraded { .. } => panic!("fixture should route"),
        };
        let native = gateway
            .build_native_request(
                &route,
                &request(),
                "https://ai.example.invalid/",
                "simulated-runtime-secret",
            )
            .expect("valid runtime values should build a request");

        assert_eq!(native.method(), "POST");
        assert_eq!(native.uri(), "https://ai.example.invalid/v1/responses");
        assert_eq!(
            native.headers()["authorization"],
            "Bearer simulated-runtime-secret"
        );
        assert!(
            !native
                .body()
                .to_string()
                .contains("simulated-runtime-secret")
        );
    }

    #[test]
    fn native_request_refuses_a_handcrafted_route_that_bypasses_policy_selection() {
        let gateway = AiGateway::from_configurations(vec![provider(
            "restricted",
            ProviderProtocol::OpenAiResponses,
        )])
        .expect("fixture is valid");
        let forged = ProviderRoute {
            provider_id: "restricted".to_owned(),
            vendor: "test-vendor".to_owned(),
            protocol: ProviderProtocol::OpenAiResponses,
            model: "different-model".to_owned(),
            endpoint_env: "ANGUI_TEST_AI_ENDPOINT".to_owned(),
            credential_env: "ANGUI_TEST_AI_KEY".to_owned(),
            timeout_ms: 5_000,
            allow_fallback: false,
        };

        assert!(matches!(
            gateway.build_native_request(
                &forged,
                &request(),
                "https://ai.example.invalid",
                "simulated-runtime-secret",
            ),
            Err(AiGatewayError::InvalidConfiguration(message))
                if message.contains("does not match")
        ));
    }

    #[test]
    fn execution_audit_hashes_the_input_and_never_copies_it() {
        let request = AiRequest {
            input: "simulated health detail that must not enter audit metadata".to_owned(),
            ..request()
        };
        let gateway = AiGateway::from_configurations(vec![provider(
            "audit-provider",
            ProviderProtocol::OpenAiResponses,
        )])
        .expect("fixture is valid");
        let decision = gateway.route(&request);
        let audit = AiExecutionAudit::for_request(&request, &decision, AiTaskStatus::Routed);
        let metadata = audit.metadata_json().to_string();

        assert_eq!(audit.input_hash.len(), 64);
        assert!(!metadata.contains(&request.input));
        assert!(!metadata.contains("system_instruction"));
        assert_eq!(audit.metadata_json()["status"], "routed");
    }
}
