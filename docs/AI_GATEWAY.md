# AI Gateway

The AI Gateway is the sole boundary between Angui business services and model-provider protocols. Business services submit an `AiRequest`; they do not import a provider SDK or assemble provider JSON payloads.

## Supported Protocols

`protocol` selects one of four wire formats:

| Value | Native HTTP request |
| --- | --- |
| `open_ai_chat_completions` | `POST /v1/chat/completions` |
| `open_ai_responses` | `POST /v1/responses` |
| `anthropic_messages` | `POST /v1/messages` |
| `gemini_generate_content` | `POST /v1beta/models/{model}:generateContent` |

The adapter builds an `http::Request<serde_json::Value>` at the transport boundary. Endpoint and credential values are supplied there from the runtime environment. They are not present in the checked-in configuration, protocol payload metadata, or audit data.

## Configuration

`ANGUI_AI_PROVIDERS_JSON` is optional. Its default is `[]`, which returns the deterministic `rule_based` degradation path. A configured provider is validated during application startup.

```json
[
  {
    "id": "demo-openai-compatible",
    "vendor": "demo-provider",
    "protocol": "open_ai_responses",
    "region": "cn-demo-1",
    "model": "demo-model",
    "capabilities": ["inquiry", "structured_extraction"],
    "allowed_data_levels": ["internal"],
    "allowed_purposes": ["intake_draft", "clue_draft"],
    "input_limit_chars": 4000,
    "output_limit_tokens": 600,
    "timeout_ms": 8000,
    "reasoning_effort": "high",
    "allow_fallback": false,
    "priority": 100,
    "weight": 1,
    "emergency_disabled": false,
    "compliance_scopes": [],
    "endpoint_env": "ANGUI_DEMO_AI_ENDPOINT",
    "credential_env": "ANGUI_DEMO_AI_KEY"
  }
]
```

This example intentionally contains neither a production URL nor a credential. `endpoint_env` and `credential_env` must be uppercase environment-variable names. Unknown properties, including `api_key`, `secret`, and direct URLs, are rejected by the configuration schema.

`reasoning_effort` is optional and applies only to `open_ai_responses`. Its valid values are `low`, `medium`, `high`, and `xhigh`; when omitted, no reasoning budget is sent and the provider's default applies. A provider using any other protocol with this field is rejected during startup rather than silently ignoring the setting.

Providers that permit `sensitive` data must declare at least one non-empty `compliance_scopes` value. Empty model identifiers, zero timeouts, empty data policies, duplicate IDs, zero limits, and zero weights are also rejected at startup.

### Capability and Policy Values

`capabilities`, `allowed_data_levels`, and `allowed_purposes` are arrays of exact `snake_case` enum values. The gateway rejects unknown values during startup, and a provider is eligible only when all three arrays authorize the request.

| `capabilities` value | Permitted model capability |
| --- | --- |
| `inquiry` | Guided follow-up questions for an intake |
| `structured_extraction` | Candidate field extraction from authorized records |
| `case_summary` | Case-summary draft generation |
| `knowledge_answer` | Answer generation for a future authorized knowledge-retrieval flow |
| `case_organization` | Case-archive and retrospective organization drafts |

| `allowed_data_levels` value | Maximum data classification the provider may receive |
| --- | --- |
| `public` | Public information |
| `collaborative` | Case collaboration information shared with authorized participants |
| `internal` | Internal case information restricted to authorized staff |
| `sensitive` | Sensitive information; requires at least one non-empty `compliance_scopes` value |

| `allowed_purposes` value | Business purpose |
| --- | --- |
| `intake_draft` | Intake follow-up and profile-draft assistance |
| `clue_draft` | Structured clue-draft assistance |
| `case_summary_draft` | Case-summary draft assistance |
| `knowledge_answer` | Future knowledge-answer assistance |
| `case_archive_draft` | Case archive and retrospective-draft assistance |

Purpose values are routing-policy values, not prompt-template names. A purpose can use more than one versioned prompt template; for example, `intake_draft` can use separate templates for a next question and a profile draft.

### Compliance Scope Naming

`compliance_scopes` should contain stable approval identifiers rather than free-form descriptions, credentials, URLs, or personal information. The naming convention is:

```text
<jurisdiction>-<data-class>-<control-or-approval>-v<version>
```

For the preview provider configuration, the initial identifiers are:

| Scope ID | Required approval or control evidence |
| --- | --- |
| `cn-sensitive-ai-dpia-v1` | Approved privacy and security impact assessment for sensitive AI processing in China |
| `cn-sensitive-provider-dpa-v1` | Confirmed provider data-processing agreement |
| `cn-sensitive-no-training-v1` | Confirmed provider commitment not to use submitted data for model training |
| `cn-sensitive-retention-30d-v1` | Confirmed provider retention and deletion policy with a 30-day maximum retention period |
| `cn-sensitive-cn-residency-v1` | Confirmed China data-residency and processing arrangement |

These identifiers are governance references. The current gateway only verifies that at least one non-empty scope is present when a provider permits `sensitive` data; it does not yet validate the identifiers against a registry, expiry date, provider agreement, or request-level authorization. Those controls must be implemented before using sensitive data in a real deployment.

## Routing and Degradation

Before a provider can become a candidate, the Gateway checks all of the following:

- requested capability;
- request data level;
- allowed business purpose;
- exact data-residency region;
- emergency-disable state;
- configured input and output limits.

Candidates are ordered deterministically by priority, then weight, then provider ID. A gateway with no configured providers returns `rule_based / no_provider_configured`. When providers exist but none meet every policy requirement, it returns `manual_required / no_compliant_provider`.

For a routed request, the Gateway makes at most **two total HTTP calls** for one business execution. The budget is shared across all recovery paths; it is not two attempts per provider. The execution state machine is deliberately small and deterministic:

| Initial result | Optional second call | Terminal behavior |
| --- | --- | --- |
| Successful provider response | None | Return the response for business-schema validation |
| Invalid JSON for a requested structured output | One same-provider JSON repair | Return the repaired JSON or fail |
| Connection failure, request timeout, or HTTP 5xx | One policy-eligible provider failover, only when the selected route enables `allow_fallback` | Return the fallback response or fail |
| HTTP 4xx, authentication/configuration error, or other permanent transport failure | None | Fail immediately |

JSON repair and provider failover are mutually exclusive. A repair request contains only the invalid-output diagnosis and the required schema, never the original business input; it has a bounded output-token limit and a distinct `:json-repair-v1` template version. A failed repair never triggers a provider failover or a third request. A fallback candidate must independently satisfy the original capability, data-level, purpose, residency, and limit policy.

The Gateway classifies failure attempts as `invalid_structured_output`, `transient_provider_failure`, `permanent_provider_failure`, `missing_runtime_configuration`, `invalid_provider_response`, or `gateway_error`. Business writes occur only after execution returns, so these bounded transport attempts cannot create duplicate cases, clues, summaries, or archive records. Circuit breaking remains a deployment/observability concern and is not represented as persistent mutable Gateway state.

`AiRequest` can carry a purpose-specific JSON Schema. The OpenAI Chat Completions and Responses adapters send strict named JSON-schema controls; Gemini sends `application/json` and its response JSON schema. Anthropic remains prompt-plus-server-validation because this adapter currently uses its text-message protocol rather than a tool-use response contract. All providers still receive server-side parsing and purpose-specific semantic validation before any draft is saved.

## Audit Boundary

Each controlled execution persists one Gateway `AiExecutionAudit` for every attempted HTTP call through `persist_execution_audits`. The record includes the selected Provider ID, model, template version, input-scope reference, SHA-256 input hash, redaction-policy version, task status, `attempt_number`, `attempt_role` (`initial`, `json_repair`, or `provider_failover`), and the stable `failure_kind` when applicable. It never contains the raw prompt/request, raw model response, provider health history, contact data, or precise locations.

The Gateway resolves provider endpoint and credential references only at its transport boundary, executes a policy-approved request with the configured timeout, and extracts response text for the supported protocols. Business services must still validate the response against their purpose-specific schema before saving a draft. Transport failures, non-success responses, empty responses, and invalid structured output must use the deterministic or manual fallback path.

The Gateway never allows an AI result to create a confirmed clue, publish a case update, or dispatch a task. Those transitions remain separate, human-reviewed business operations.

## Implemented controlled-assistance paths

- Intake follow-up uses `inquiry / intake_draft` only for the session creator's currently authorized answers. The response is a single optional JSON question with its purpose and missing fields. Invalid output, unavailable providers, timeout, and policy mismatch return the fixed question set; a family member can always mark an answer unknown, edit it, or use the static flow.
- Clue extraction uses `structured_extraction / clue_draft`. Candidate fields never create facts directly. A commander must accept, edit, clear, or reject fields; acceptance creates a normal `pending_review` clue which remains subject to the existing clue-review state machine.
- Case summaries use `case_summary / case_summary_draft`. A model candidate must contain `confirmed_information`, `pending_verification`, `excluded_directions`, `safety_reminders`, and a non-empty `uncertainty_notice`. Each section is length-bounded and source-checked against the commander's authorized, already-reviewed summary scope. Rule-based output remains available when execution or semantic validation fails. Drafts are immutable versions with a parent reference; commanders can list versions, compare line-level changes, submit, publish, reject, or withdraw them.
- Archive organization uses `case_organization / case_archive_draft` only after an administrator has confirmed a separate de-identified review-material version. The initial archive draft is a non-reusable placeholder; its controlled review material is limited to confirmed clue and completed-task review content and stays outside the AI input boundary until that human confirmation. Raw conversations, identities, contacts, health data, exact locations, routes, attachments, and unreviewed clues are excluded. The resulting organization draft remains non-reusable until the existing administrator review lifecycle completes.

## Prompt Templates

Approved system instructions are stored in the versioned `ai_prompt_templates` database table, not in provider environment variables or an MCP server. The common table is keyed by purpose, so it can serve `intake_next_question`, `intake_profile_draft`, `clue_draft`, `case_summary_draft`, `knowledge_answer`, and `case_archive_draft` without duplicating publication and audit rules for every module. The current controlled runtime paths retain their security-critical instructions and schema contracts in reviewed Rust code; the seed template is not yet a runtime override for those safety constraints. A future template-management path must compose published wording only with, never instead of, the non-bypassable server safety contract.

The future management API must restrict draft, publication, and retirement actions to the appropriate administrative capability, create a corresponding audit event, preserve published versions for reproducibility, and prohibit direct client-supplied prompt text in normal business requests. MCP may be useful as a separate, authenticated operator integration, but it is not the runtime source of prompt configuration.
