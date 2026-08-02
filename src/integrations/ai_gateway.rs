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
        attempts: Vec<AiExecutionAttempt>,
    },
    Degraded {
        status: DegradationStatus,
        reason: DegradationReason,
        attempts: Vec<AiExecutionAttempt>,
    },
    Failed {
        route: ProviderRoute,
        attempts: Vec<AiExecutionAttempt>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AiExecutionAttempt {
    pub number: u8,
    pub role: &'static str,
    pub route: ProviderRoute,
    pub status: AiTaskStatus,
    pub failure_kind: Option<&'static str>,
}

impl AiExecutionResult {
    pub fn decision(&self) -> GatewayDecision {
        match self {
            Self::Completed { route, .. } | Self::Failed { route, .. } => {
                GatewayDecision::Routed(route.clone())
            }
            Self::Degraded { status, reason, .. } => GatewayDecision::Degraded {
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

    pub fn attempts(&self) -> &[AiExecutionAttempt] {
        match self {
            Self::Completed { attempts, .. }
            | Self::Degraded { attempts, .. }
            | Self::Failed { attempts, .. } => attempts,
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
    pub attempt_number: u8,
    pub attempt_role: &'static str,
    pub failure_kind: Option<&'static str>,
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
            attempt_number: 1,
            attempt_role: "final",
            failure_kind: None,
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
            "attempt_number": self.attempt_number,
            "attempt_role": self.attempt_role,
            "failure_kind": self.failure_kind,
        })
    }
}

pub fn execution_attempt_audits(
    request: &AiRequest,
    result: &AiExecutionResult,
) -> Vec<AiExecutionAudit> {
    result
        .attempts()
        .iter()
        .map(|attempt| AiExecutionAudit {
            id: Uuid::new_v4().to_string(),
            provider_id: Some(attempt.route.provider_id.clone()),
            model: Some(attempt.route.model.clone()),
            template_version: request.template_version.clone(),
            input_scope_reference: request.input_scope_reference.clone(),
            input_hash: hex::encode(Sha256::digest(request.input.as_bytes())),
            redaction_policy_version: request.redaction_policy_version.clone(),
            status: attempt.status,
            attempt_number: attempt.number,
            attempt_role: attempt.role,
            failure_kind: attempt.failure_kind,
        })
        .collect()
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

pub async fn persist_execution_audits<C: ConnectionTrait>(
    db: &C,
    audits: &[AiExecutionAudit],
    actor: &str,
    case_id: Option<&str>,
) -> Result<(), sea_orm::DbErr> {
    for audit in audits {
        persist_execution_audit(db, audit, actor, case_id).await?;
    }
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
    #[error("AI provider request failed permanently")]
    PermanentProviderFailure,
    #[error("AI provider request failed transiently")]
    TransientProviderFailure,
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

    /// Validate provider endpoint and credential references without sending a
    /// request. This uses the same native request materialization path as
    /// execution, including endpoint URI and non-empty credential checks.
    pub fn validate_runtime_configuration(&self) -> Result<(), AiGatewayError> {
        let request = AiRequest {
            capability: AiCapability::Inquiry,
            data_level: DataLevel::Public,
            purpose: AiPurpose::IntakeDraft,
            data_region: "runtime-validation".to_owned(),
            system_instruction: None,
            output_schema: None,
            output_schema_name: None,
            input: "runtime configuration validation".to_owned(),
            requested_output_tokens: 1,
            template_version: "runtime-validation-v1".to_owned(),
            input_scope_reference: "runtime-validation".to_owned(),
            redaction_policy_version: "runtime-validation-v1".to_owned(),
        };

        for registered in &self.providers {
            let route = registered.provider.route();
            let endpoint = env::var(&route.endpoint_env).map_err(|_| {
                AiGatewayError::MissingRuntimeConfiguration(route.endpoint_env.clone())
            })?;
            let credential = env::var(&route.credential_env).map_err(|_| {
                AiGatewayError::MissingRuntimeConfiguration(route.credential_env.clone())
            })?;
            registered
                .provider
                .build_request(&request)?
                .into_native_request(&endpoint, &credential)?;
        }
        Ok(())
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
                return AiExecutionResult::Degraded {
                    status,
                    reason,
                    attempts: Vec::new(),
                };
            }
        };

        // One execution has a hard budget of two calls. A syntactically invalid
        // structured result receives one same-provider repair call; a timeout
        // or 5xx receives one newly-policy-checked fallback call. The paths are
        // intentionally mutually exclusive.
        let mut attempts = Vec::new();
        match self.execute_route(&route, request).await {
            Ok(output) => {
                attempts.push(AiExecutionAttempt {
                    number: 1,
                    role: "initial",
                    route: route.clone(),
                    status: AiTaskStatus::Completed,
                    failure_kind: None,
                });
                AiExecutionResult::Completed {
                    route,
                    output,
                    attempts,
                }
            }
            Err(AiGatewayError::InvalidStructuredOutput) => {
                attempts.push(AiExecutionAttempt {
                    number: 1,
                    role: "initial",
                    route: route.clone(),
                    status: AiTaskStatus::Failed,
                    failure_kind: Some("invalid_structured_output"),
                });
                let repair = repair_request(
                    request,
                    "provider output was not valid JSON for the requested schema",
                );
                match self.execute_route(&route, &repair).await {
                    Ok(output) => {
                        attempts.push(AiExecutionAttempt {
                            number: 2,
                            role: "json_repair",
                            route: route.clone(),
                            status: AiTaskStatus::Completed,
                            failure_kind: None,
                        });
                        AiExecutionResult::Completed {
                            route,
                            output,
                            attempts,
                        }
                    }
                    Err(error) => {
                        attempts.push(AiExecutionAttempt {
                            number: 2,
                            role: "json_repair",
                            route: route.clone(),
                            status: AiTaskStatus::Failed,
                            failure_kind: Some(failure_kind(&error)),
                        });
                        AiExecutionResult::Failed { route, attempts }
                    }
                }
            }
            Err(AiGatewayError::TransientProviderFailure) if route.allow_fallback => {
                attempts.push(AiExecutionAttempt {
                    number: 1,
                    role: "initial",
                    route: route.clone(),
                    status: AiTaskStatus::Failed,
                    failure_kind: Some("transient_provider_failure"),
                });
                let fallback = self
                    .eligible_routes(request)
                    .into_iter()
                    .find(|candidate| candidate.provider_id != route.provider_id);
                match fallback {
                    Some(fallback) => match self.execute_route(&fallback, request).await {
                        Ok(output) => {
                            attempts.push(AiExecutionAttempt {
                                number: 2,
                                role: "provider_failover",
                                route: fallback.clone(),
                                status: AiTaskStatus::Completed,
                                failure_kind: None,
                            });
                            AiExecutionResult::Completed {
                                route: fallback,
                                output,
                                attempts,
                            }
                        }
                        Err(error) => {
                            attempts.push(AiExecutionAttempt {
                                number: 2,
                                role: "provider_failover",
                                route: fallback.clone(),
                                status: AiTaskStatus::Failed,
                                failure_kind: Some(failure_kind(&error)),
                            });
                            AiExecutionResult::Failed {
                                route: fallback,
                                attempts,
                            }
                        }
                    },
                    None => AiExecutionResult::Failed { route, attempts },
                }
            }
            Err(error) => {
                attempts.push(AiExecutionAttempt {
                    number: 1,
                    role: "initial",
                    route: route.clone(),
                    status: AiTaskStatus::Failed,
                    failure_kind: Some(failure_kind(&error)),
                });
                AiExecutionResult::Failed { route, attempts }
            }
        }
    }

    async fn execute_route(
        &self,
        route: &ProviderRoute,
        request: &AiRequest,
    ) -> Result<String, AiGatewayError> {
        async {
            let endpoint = env::var(&route.endpoint_env).map_err(|_| {
                AiGatewayError::MissingRuntimeConfiguration(route.endpoint_env.clone())
            })?;
            let credential = env::var(&route.credential_env).map_err(|_| {
                AiGatewayError::MissingRuntimeConfiguration(route.credential_env.clone())
            })?;
            let native = self.build_native_request(route, request, &endpoint, &credential)?;
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
                .map_err(|error| {
                    if error.is_timeout() || error.is_connect() {
                        AiGatewayError::TransientProviderFailure
                    } else {
                        AiGatewayError::PermanentProviderFailure
                    }
                })?;
            if !response.status().is_success() {
                return Err(if response.status().is_server_error() {
                    AiGatewayError::TransientProviderFailure
                } else {
                    AiGatewayError::PermanentProviderFailure
                });
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
        .await
    }

    pub fn decode_json<T: DeserializeOwned>(&self, output: &str) -> Result<T, AiGatewayError> {
        serde_json::from_str(output).map_err(|_| AiGatewayError::InvalidStructuredOutput)
    }
}

fn failure_kind(error: &AiGatewayError) -> &'static str {
    match error {
        AiGatewayError::InvalidStructuredOutput => "invalid_structured_output",
        AiGatewayError::TransientProviderFailure => "transient_provider_failure",
        AiGatewayError::PermanentProviderFailure => "permanent_provider_failure",
        AiGatewayError::MissingRuntimeConfiguration(_) => "missing_runtime_configuration",
        AiGatewayError::InvalidProviderResponse => "invalid_provider_response",
        _ => "gateway_error",
    }
}

fn repair_request(request: &AiRequest, error: &str) -> AiRequest {
    AiRequest {
        capability: request.capability,
        data_level: request.data_level,
        purpose: request.purpose,
        data_region: request.data_region.clone(),
        system_instruction: Some(format!("Return only valid JSON matching the requested schema. Repair the supplied invalid model output. Error: {error}. Do not add information.")),
        output_schema: request.output_schema.clone(),
        output_schema_name: request.output_schema_name.clone(),
        // Deliberately excludes the original sensitive business input.
        input: "The preceding model result was invalid. Produce only schema-valid JSON with null or empty values when unsupported.".to_owned(),
        requested_output_tokens: request.requested_output_tokens.min(240),
        template_version: format!("{}:json-repair-v1", request.template_version),
        input_scope_reference: "ai-output-repair-no-business-input".to_owned(),
        redaction_policy_version: request.redaction_policy_version.clone(),
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
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, LazyLock, Mutex as StdMutex},
        thread,
        time::Duration,
    };

    static ENV_LOCK: LazyLock<tokio::sync::Mutex<()>> =
        LazyLock::new(|| tokio::sync::Mutex::new(()));

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

    fn mock_responses(responses: Vec<(u16, &'static str)>) -> (String, Arc<StdMutex<usize>>) {
        mock_responses_with_delays(
            responses
                .into_iter()
                .map(|(status, body)| (status, body, 0))
                .collect(),
        )
    }

    fn mock_responses_with_delays(
        responses: Vec<(u16, &'static str, u64)>,
    ) -> (String, Arc<StdMutex<usize>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock listener");
        let endpoint = format!("http://{}", listener.local_addr().expect("mock address"));
        let calls = Arc::new(StdMutex::new(0usize));
        let count = Arc::clone(&calls);
        thread::spawn(move || {
            for (status, body, delay_ms) in responses {
                let (mut stream, _) = listener.accept().expect("mock accepts request");
                let mut buffer = [0_u8; 16_384];
                let _ = stream.read(&mut buffer);
                *count.lock().expect("counter lock") += 1;
                if delay_ms > 0 {
                    thread::sleep(Duration::from_millis(delay_ms));
                }
                let phrase = if status == 200 {
                    "OK"
                } else if status >= 500 {
                    "Service Unavailable"
                } else {
                    "Bad Request"
                };
                let response = format!(
                    "HTTP/1.1 {status} {phrase}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (endpoint, calls)
    }

    #[tokio::test]
    async fn runtime_configuration_validation_requires_usable_references() {
        let _environment = ENV_LOCK.lock().await;
        unsafe {
            env::remove_var("ANGUI_TEST_AI_ENDPOINT");
            env::remove_var("ANGUI_TEST_AI_KEY");
        }
        let gateway = AiGateway::from_configurations(vec![provider(
            "runtime-validation",
            ProviderProtocol::OpenAiResponses,
        )])
        .expect("fixture is valid");

        assert!(matches!(
            gateway.validate_runtime_configuration(),
            Err(AiGatewayError::MissingRuntimeConfiguration(name)) if name == "ANGUI_TEST_AI_ENDPOINT"
        ));

        unsafe {
            env::set_var("ANGUI_TEST_AI_ENDPOINT", "https://ai.example.invalid");
            env::set_var("ANGUI_TEST_AI_KEY", "");
        }
        assert!(matches!(
            gateway.validate_runtime_configuration(),
            Err(AiGatewayError::InvalidNativeRequest(_))
        ));

        unsafe {
            env::set_var("ANGUI_TEST_AI_ENDPOINT", "https://invalid endpoint");
            env::set_var("ANGUI_TEST_AI_KEY", "test-key");
        }
        assert!(matches!(
            gateway.validate_runtime_configuration(),
            Err(AiGatewayError::InvalidNativeRequest(_))
        ));

        unsafe {
            env::set_var("ANGUI_TEST_AI_ENDPOINT", "https://ai.example.invalid");
        }
        assert!(gateway.validate_runtime_configuration().is_ok());

        unsafe {
            env::remove_var("ANGUI_TEST_AI_ENDPOINT");
            env::remove_var("ANGUI_TEST_AI_KEY");
        }
    }

    #[tokio::test]
    async fn invalid_json_runs_one_repair_request_and_stops_after_two_calls() {
        let _environment = ENV_LOCK.lock().await;
        let (endpoint, calls) = mock_responses(vec![
            (200, r#"{"output_text":"not-json"}"#),
            (200, r#"{"output_text":"{\"answer\":\"ok\"}"}"#),
        ]);
        unsafe {
            env::set_var("ANGUI_TEST_AI_ENDPOINT", endpoint);
            env::set_var("ANGUI_TEST_AI_KEY", "test-key");
        }
        let mut configuration = provider("repair", ProviderProtocol::OpenAiResponses);
        configuration.allow_fallback = true;
        let gateway = AiGateway::from_configurations(vec![configuration]).expect("gateway");
        let mut request = request();
        request.output_schema = Some(json!({"type":"object"}));
        request.output_schema_name = Some("answer".to_owned());
        let result = gateway.execute(&request).await;
        assert!(matches!(result, AiExecutionResult::Completed { .. }));
        assert_eq!(*calls.lock().expect("counter lock"), 2);
        assert_eq!(
            result
                .attempts()
                .iter()
                .map(|attempt| attempt.role)
                .collect::<Vec<_>>(),
            vec!["initial", "json_repair"]
        );
        unsafe {
            env::remove_var("ANGUI_TEST_AI_ENDPOINT");
            env::remove_var("ANGUI_TEST_AI_KEY");
        }
    }

    #[tokio::test]
    async fn client_failure_does_not_retry_or_fail_over() {
        let _environment = ENV_LOCK.lock().await;
        let (endpoint, calls) = mock_responses(vec![(400, r#"{"error":"bad request"}"#)]);
        unsafe {
            env::set_var("ANGUI_TEST_AI_ENDPOINT", endpoint);
            env::set_var("ANGUI_TEST_AI_KEY", "test-key");
        }
        let mut configuration = provider("client-error", ProviderProtocol::OpenAiResponses);
        configuration.allow_fallback = true;
        let gateway = AiGateway::from_configurations(vec![configuration]).expect("gateway");
        let result = gateway.execute(&request()).await;
        assert!(matches!(result, AiExecutionResult::Failed { .. }));
        assert_eq!(*calls.lock().expect("counter lock"), 1);
        assert_eq!(result.attempts()[0].role, "initial");
        assert_eq!(
            result.attempts()[0].failure_kind,
            Some("permanent_provider_failure")
        );
        unsafe {
            env::remove_var("ANGUI_TEST_AI_ENDPOINT");
            env::remove_var("ANGUI_TEST_AI_KEY");
        }
    }

    #[tokio::test]
    async fn server_failure_uses_one_compliant_fallback() {
        let _environment = ENV_LOCK.lock().await;
        let (primary_endpoint, primary_calls) =
            mock_responses(vec![(503, r#"{"error":"temporary"}"#)]);
        let (fallback_endpoint, fallback_calls) =
            mock_responses(vec![(200, r#"{"output_text":"ok"}"#)]);
        unsafe {
            env::set_var("ANGUI_TEST_AI_ENDPOINT", primary_endpoint);
            env::set_var("ANGUI_TEST_AI_KEY", "test-key");
            env::set_var("ANGUI_TEST_AI_FALLBACK_ENDPOINT", fallback_endpoint);
            env::set_var("ANGUI_TEST_AI_FALLBACK_KEY", "test-key");
        }
        let mut primary = provider("primary", ProviderProtocol::OpenAiResponses);
        primary.allow_fallback = true;
        primary.priority = 20;
        let mut fallback = provider("fallback", ProviderProtocol::OpenAiResponses);
        fallback.endpoint_env = "ANGUI_TEST_AI_FALLBACK_ENDPOINT".to_owned();
        fallback.credential_env = "ANGUI_TEST_AI_FALLBACK_KEY".to_owned();
        fallback.priority = 10;
        let gateway = AiGateway::from_configurations(vec![primary, fallback]).expect("gateway");
        let result = gateway.execute(&request()).await;
        assert!(matches!(result, AiExecutionResult::Completed { .. }));
        assert_eq!(*primary_calls.lock().expect("counter lock"), 1);
        assert_eq!(*fallback_calls.lock().expect("counter lock"), 1);
        assert_eq!(
            result
                .attempts()
                .iter()
                .map(|attempt| attempt.role)
                .collect::<Vec<_>>(),
            vec!["initial", "provider_failover"]
        );
        unsafe {
            env::remove_var("ANGUI_TEST_AI_ENDPOINT");
            env::remove_var("ANGUI_TEST_AI_KEY");
            env::remove_var("ANGUI_TEST_AI_FALLBACK_ENDPOINT");
            env::remove_var("ANGUI_TEST_AI_FALLBACK_KEY");
        }
    }

    #[tokio::test]
    async fn timeout_uses_one_provider_failover_without_third_request() {
        let _environment = ENV_LOCK.lock().await;
        let (primary_endpoint, primary_calls) =
            mock_responses_with_delays(vec![(200, r#"{"output_text":"late"}"#, 120)]);
        let (fallback_endpoint, fallback_calls) =
            mock_responses(vec![(200, r#"{"output_text":"ok"}"#)]);
        unsafe {
            env::set_var("ANGUI_TEST_AI_ENDPOINT", primary_endpoint);
            env::set_var("ANGUI_TEST_AI_KEY", "test-key");
            env::set_var("ANGUI_TEST_AI_FALLBACK_ENDPOINT", fallback_endpoint);
            env::set_var("ANGUI_TEST_AI_FALLBACK_KEY", "test-key");
        }
        let mut primary = provider("timeout-primary", ProviderProtocol::OpenAiResponses);
        primary.allow_fallback = true;
        primary.timeout_ms = 20;
        primary.priority = 20;
        let mut fallback = provider("timeout-fallback", ProviderProtocol::OpenAiResponses);
        fallback.endpoint_env = "ANGUI_TEST_AI_FALLBACK_ENDPOINT".to_owned();
        fallback.credential_env = "ANGUI_TEST_AI_FALLBACK_KEY".to_owned();
        fallback.priority = 10;
        let gateway = AiGateway::from_configurations(vec![primary, fallback]).expect("gateway");
        let result = gateway.execute(&request()).await;
        assert!(matches!(result, AiExecutionResult::Completed { .. }));
        assert_eq!(*primary_calls.lock().expect("counter lock"), 1);
        assert_eq!(*fallback_calls.lock().expect("counter lock"), 1);
        assert_eq!(
            result
                .attempts()
                .iter()
                .map(|attempt| attempt.role)
                .collect::<Vec<_>>(),
            vec!["initial", "provider_failover"]
        );
        assert_eq!(
            result.attempts()[0].failure_kind,
            Some("transient_provider_failure")
        );
        unsafe {
            env::remove_var("ANGUI_TEST_AI_ENDPOINT");
            env::remove_var("ANGUI_TEST_AI_KEY");
            env::remove_var("ANGUI_TEST_AI_FALLBACK_ENDPOINT");
            env::remove_var("ANGUI_TEST_AI_FALLBACK_KEY");
        }
    }

    #[tokio::test]
    async fn failed_repair_never_makes_a_third_request() {
        let _environment = ENV_LOCK.lock().await;
        let (endpoint, calls) = mock_responses(vec![
            (200, r#"{"output_text":"not-json"}"#),
            (200, r#"{"output_text":"still-not-json"}"#),
        ]);
        unsafe {
            env::set_var("ANGUI_TEST_AI_ENDPOINT", endpoint);
            env::set_var("ANGUI_TEST_AI_KEY", "test-key");
        }
        let gateway = AiGateway::from_configurations(vec![provider(
            "repair-fails",
            ProviderProtocol::OpenAiResponses,
        )])
        .expect("gateway");
        let mut request = request();
        request.output_schema = Some(json!({"type":"object"}));
        request.output_schema_name = Some("answer".to_owned());
        let result = gateway.execute(&request).await;
        assert!(matches!(result, AiExecutionResult::Failed { .. }));
        assert_eq!(*calls.lock().expect("counter lock"), 2);
        assert_eq!(
            result
                .attempts()
                .iter()
                .map(|attempt| attempt.role)
                .collect::<Vec<_>>(),
            vec!["initial", "json_repair"]
        );
        unsafe {
            env::remove_var("ANGUI_TEST_AI_ENDPOINT");
            env::remove_var("ANGUI_TEST_AI_KEY");
        }
    }

    #[test]
    fn attempt_audits_never_include_raw_input_or_output() {
        let route = route_for(&provider("audit", ProviderProtocol::OpenAiResponses));
        let result = AiExecutionResult::Failed {
            route: route.clone(),
            attempts: vec![AiExecutionAttempt {
                number: 1,
                role: "initial",
                route,
                status: AiTaskStatus::Failed,
                failure_kind: Some("invalid_structured_output"),
            }],
        };
        let mut request = request();
        request.input = "sensitive family health and exact location text".to_owned();
        let audits = execution_attempt_audits(&request, &result);
        assert_eq!(audits.len(), 1);
        let metadata = audits[0].metadata_json().to_string();
        assert!(metadata.contains("attempt_number"));
        assert!(metadata.contains("invalid_structured_output"));
        assert!(!metadata.contains(&request.input));
        assert!(!metadata.contains("model-output-body"));
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
