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

Providers that permit `sensitive` data must declare at least one non-empty `compliance_scopes` value. Empty model identifiers, zero timeouts, empty data policies, duplicate IDs, zero limits, and zero weights are also rejected at startup.

## Routing and Degradation

Before a provider can become a candidate, the Gateway checks all of the following:

- requested capability;
- request data level;
- allowed business purpose;
- exact data-residency region;
- emergency-disable state;
- configured input and output limits.

Candidates are ordered deterministically by priority, then weight, then provider ID. A gateway with no configured providers returns `rule_based / no_provider_configured`. When providers exist but none meet every policy requirement, it returns `manual_required / no_compliant_provider`. Automatic retry, circuit-breaking, and cross-provider failover are intentionally outside this component and belong to the later reliability issues.

## Audit Boundary

Each future execution is expected to persist the Gateway's `AiExecutionAudit` through `persist_execution_audit`. The record includes Provider ID, model, template version, input-scope reference, SHA-256 input hash, redaction-policy version, and task status. It never contains the raw request, response, health history, contact data, or precise locations.

This PR provides the shared boundary and audit writer only. It does not yet call external AI services or allow an AI result to create a confirmed clue, publish a case update, or dispatch a task.

## Prompt Templates

Approved system instructions are stored in the versioned `ai_prompt_templates` database table, not in provider environment variables or an MCP server. The common table is keyed by purpose, so it can serve `intake_next_question`, `intake_profile_draft`, `clue_draft`, `case_summary_draft`, `knowledge_answer`, and `case_archive_draft` without duplicating publication and audit rules for every module. The currently seeded intake template is reserved for the future model-backed path; the shipped intake flow is deterministic and rule-based.

The future management API must restrict draft, publication, and retirement actions to the appropriate administrative capability, create a corresponding audit event, preserve published versions for reproducibility, and prohibit direct client-supplied prompt text in normal business requests. MCP may be useful as a separate, authenticated operator integration, but it is not the runtime source of prompt configuration.
