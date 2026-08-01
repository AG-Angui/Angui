use std::{collections::HashSet, env, sync::Arc, time::Duration};

use chrono::{SecondsFormat, Utc};
use http::Request as HttpRequest;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
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

/// Reasoning budget requested from an OpenAI Responses-compatible provider.
///
/// This is intentionally provider-local: different fallback providers can use
/// different reasoning budgets without sharing a global runtime setting.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
    Xhigh,
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
    /// Optional reasoning budget, supported only by OpenAI Responses-compatible
    /// providers.
    #[serde(default)]
    pub reasoning_effort: Option<ReasoningEffort>,
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
    /// Provider-neutral JSON Schema for a structured model result. Providers
    /// that expose a native constrained-output feature receive this schema;
    /// every result is still validated again by the calling service.
    pub output_schema: Option<Value>,
    /// Stable schema name used by providers that require a named contract.
    pub output_schema_name: Option<String>,
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

/// The result of one provider execution. Business services must treat every
/// non-completed result as a signal to use their deterministic or manual path.
#[derive(Clone, Debug, PartialEq)]
pub enum AiExecutionResult {
    Completed {
        route: ProviderRoute,
        output: String,
    },
    Degraded {
        status: DegradationStatus,
        reason: DegradationReason,
    },
    Failed {
        route: ProviderRoute,
    },
}

impl AiExecutionResult {
    pub fn decision(&self) -> GatewayDecision {
        match self {
            Self::Completed { route, .. } | Self::Failed { route } => {
                GatewayDecision::Routed(route.clone())
            }
            Self::Degraded { status, reason } => GatewayDecision::Degraded {
                status: *status,
                reason: *reason,
            },
        }
    }

    pub fn audit_status(&self) -> AiTaskStatus {
        match self {
            Self::Completed { .. } => AiTaskStatus::Completed,
            Self::Degraded { .. } => AiTaskStatus::Degraded,
            Self::Failed { .. } => AiTaskStatus::Failed,
        }
    }
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
    #[error("AI provider environment variable {0} is unavailable")]
    MissingRuntimeConfiguration(String),
    #[error("AI provider response was invalid")]
    InvalidProviderResponse,
    #[error("AI provider output was invalid")]
    InvalidStructuredOutput,
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
                body: openai_chat_body(
                    json!({
                        "model": self.config.model,
                        "messages": openai_messages(instructions, request.input.as_str()),
                        "max_tokens": max_tokens,
                    }),
                    request,
                ),
            },
            ProviderProtocol::OpenAiResponses => ProviderHttpRequest {
                method: "POST",
                path: "/v1/responses".to_owned(),
                headers: vec![ProviderHeader {
                    name: "authorization",
                    value: credential,
                }],
                body: {
                    let mut body = json!({
                        "model": self.config.model,
                        "input": openai_messages(instructions, request.input.as_str()),
                        "max_output_tokens": max_tokens,
                    });
                    if let Some(reasoning_effort) = self.config.reasoning_effort {
                        body["reasoning"] = json!({ "effort": reasoning_effort });
                    }
                    add_openai_responses_schema(body, request)
                },
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
                if let Some(schema) = &request.output_schema {
                    body["generationConfig"]["responseMimeType"] = json!("application/json");
                    body["generationConfig"]["responseJsonSchema"] = schema.clone();
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

fn openai_chat_body(mut body: Value, request: &AiRequest) -> Value {
    if let (Some(schema), Some(name)) = (&request.output_schema, &request.output_schema_name) {
        body["response_format"] = json!({
            "type": "json_schema",
            "json_schema": { "name": name, "strict": true, "schema": schema },
        });
    }
    body
}

fn add_openai_responses_schema(mut body: Value, request: &AiRequest) -> Value {
    if let (Some(schema), Some(name)) = (&request.output_schema, &request.output_schema_name) {
        body["text"] = json!({
            "format": { "type": "json_schema", "name": name, "strict": true, "schema": schema },
        });
    }
    body
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
        self.eligible_routes(request)
            .into_iter()
            .next()
            .map(GatewayDecision::Routed)
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

    fn eligible_routes(&self, request: &AiRequest) -> Vec<ProviderRoute> {
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
        candidates.into_iter().map(|(route, _, _)| route).collect()
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

    /// Executes a policy-approved provider request. All runtime configuration
    /// is resolved here so application services never handle provider
    /// credentials, endpoints, or protocol payloads directly.
    pub async fn execute(&self, request: &AiRequest) -> AiExecutionResult {
        let decision = self.route(request);
        let route = match decision {
            GatewayDecision::Routed(route) => route,
            GatewayDecision::Degraded { status, reason } => {
                return AiExecutionResult::Degraded { status, reason };
            }
        };

        let routes = self.eligible_routes(request);
        for (index, candidate_route) in routes.iter().enumerate() {
            // A transport request is read-only. Retrying it once is safe and
            // keeps malformed/upstream transient failures out of business
            // workflows. Writes happen only after this method returns.
            for _ in 0..2 {
                if let Ok(output) = self.execute_route(candidate_route, request).await {
                    return AiExecutionResult::Completed {
                        route: candidate_route.clone(),
                        output,
                    };
                }
            }
            if !candidate_route.allow_fallback || index + 1 == routes.len() {
                return AiExecutionResult::Failed {
                    route: candidate_route.clone(),
                };
            }
        }
        AiExecutionResult::Failed { route }
    }

    async fn execute_route(
        &self,
        route: &ProviderRoute,
        request: &AiRequest,
    ) -> Result<String, AiGatewayError> {
        let outcome = async {
            let endpoint = env::var(&route.endpoint_env).map_err(|_| {
                AiGatewayError::MissingRuntimeConfiguration(route.endpoint_env.clone())
            })?;
            let credential = env::var(&route.credential_env).map_err(|_| {
                AiGatewayError::MissingRuntimeConfiguration(route.credential_env.clone())
            })?;
            let native = self.build_native_request(&route, request, &endpoint, &credential)?;
            let client = reqwest::Client::builder()
                .timeout(Duration::from_millis(route.timeout_ms))
                .build()
                .map_err(|_| {
                    AiGatewayError::InvalidNativeRequest(
                        "failed to build provider client".to_owned(),
                    )
                })?;
            let method =
                reqwest::Method::from_bytes(native.method().as_str().as_bytes()).map_err(|_| {
                    AiGatewayError::InvalidNativeRequest(
                        "unsupported provider HTTP method".to_owned(),
                    )
                })?;
            let mut provider_request = client.request(method, native.uri().to_string());
            for (name, value) in native.headers() {
                provider_request = provider_request.header(name.as_str(), value.as_bytes());
            }
            let response = provider_request
                .json(native.body())
                .send()
                .await
                .map_err(|_| AiGatewayError::InvalidProviderResponse)?;
            if !response.status().is_success() {
                return Err(AiGatewayError::InvalidProviderResponse);
            }
            let payload = response
                .json::<Value>()
                .await
                .map_err(|_| AiGatewayError::InvalidProviderResponse)?;
            let output = extract_provider_output(route.protocol, &payload)?;
            if request.output_schema.is_some() {
                serde_json::from_str::<Value>(&output)
                    .map_err(|_| AiGatewayError::InvalidStructuredOutput)?;
            }
            Ok(output)
        }
        .await;

        outcome
    }

    pub fn decode_json<T: DeserializeOwned>(&self, output: &str) -> Result<T, AiGatewayError> {
        serde_json::from_str(output).map_err(|_| AiGatewayError::InvalidStructuredOutput)
    }
}

fn extract_provider_output(
    protocol: ProviderProtocol,
    payload: &Value,
) -> Result<String, AiGatewayError> {
    let output = match protocol {
        ProviderProtocol::OpenAiChatCompletions => payload
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str),
        ProviderProtocol::OpenAiResponses => payload
            .get("output_text")
            .and_then(Value::as_str)
            .or_else(|| {
                payload
                    .pointer("/output/0/content/0/text")
                    .and_then(Value::as_str)
            }),
        ProviderProtocol::AnthropicMessages => {
            payload.pointer("/content/0/text").and_then(Value::as_str)
        }
        ProviderProtocol::GeminiGenerateContent => payload
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(Value::as_str),
    }
    .map(str::trim)
    .filter(|output| !output.is_empty())
    .ok_or(AiGatewayError::InvalidProviderResponse)?;
    Ok(output.to_owned())
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
        if config.reasoning_effort.is_some() && config.protocol != ProviderProtocol::OpenAiResponses
        {
            return Err(AiGatewayError::InvalidConfiguration(format!(
                "provider {:?} may only configure reasoning_effort for open_ai_responses",
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
            reasoning_effort: None,
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
            output_schema: None,
            output_schema_name: None,
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
    fn validation_rejects_reasoning_effort_for_non_responses_provider() {
        let mut configuration = provider("chat", ProviderProtocol::OpenAiChatCompletions);
        configuration.reasoning_effort = Some(ReasoningEffort::High);

        assert!(matches!(
            validate_provider_configurations(&[configuration]),
            Err(AiGatewayError::InvalidConfiguration(message))
                if message.contains("reasoning_effort")
        ));
    }

    #[test]
    fn responses_provider_serializes_configured_reasoning_effort() {
        let mut configuration = provider("reasoning", ProviderProtocol::OpenAiResponses);
        configuration.reasoning_effort = Some(ReasoningEffort::High);
        let gateway = AiGateway::from_configurations(vec![configuration])
            .expect("valid Responses configuration should be accepted");
        let route = match gateway.route(&request()) {
            GatewayDecision::Routed(route) => route,
            GatewayDecision::Degraded { .. } => panic!("configured provider should route"),
        };

        let outbound = gateway
            .build_provider_request(&route, &request())
            .expect("provider request should be created");

        assert_eq!(outbound.body["reasoning"]["effort"], "high");
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
    fn structured_requests_use_native_schema_controls_when_supported() {
        for protocol in [
            ProviderProtocol::OpenAiChatCompletions,
            ProviderProtocol::OpenAiResponses,
            ProviderProtocol::GeminiGenerateContent,
        ] {
            let gateway = AiGateway::from_configurations(vec![provider("structured", protocol)])
                .expect("fixture is valid");
            let route = match gateway.route(&request()) {
                GatewayDecision::Routed(route) => route,
                GatewayDecision::Degraded { .. } => panic!("fixture should route"),
            };
            let mut structured = request();
            structured.output_schema = Some(json!({
                "type": "object",
                "properties": { "answer": { "type": "string" } },
                "required": ["answer"],
                "additionalProperties": false
            }));
            structured.output_schema_name = Some("test_answer".to_owned());
            let outbound = gateway
                .build_provider_request(&route, &structured)
                .expect("structured request should be serializable");
            match protocol {
                ProviderProtocol::OpenAiChatCompletions => {
                    assert_eq!(outbound.body["response_format"]["type"], "json_schema");
                    assert_eq!(
                        outbound.body["response_format"]["json_schema"]["strict"],
                        true
                    );
                }
                ProviderProtocol::OpenAiResponses => {
                    assert_eq!(outbound.body["text"]["format"]["type"], "json_schema");
                    assert_eq!(outbound.body["text"]["format"]["strict"], true);
                }
                ProviderProtocol::GeminiGenerateContent => {
                    assert_eq!(
                        outbound.body["generationConfig"]["responseMimeType"],
                        "application/json"
                    );
                    assert!(
                        outbound.body["generationConfig"]
                            .get("responseJsonSchema")
                            .is_some()
                    );
                }
                ProviderProtocol::AnthropicMessages => unreachable!(),
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
