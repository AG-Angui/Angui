use std::sync::LazyLock;

static OPENAPI: LazyLock<String> =
    LazyLock::new(|| include_str!("../../docs/openapi.yaml").replace("\r\n", "\n"));

fn operation_block<'a>(openapi: &'a str, path: &str) -> Option<&'a str> {
    let marker = format!("  {path}");
    let (_, after_marker) = openapi.split_once(&marker)?;
    let next_path = after_marker.find("\n  /");
    let next_top_level = after_marker
        .match_indices('\n')
        .find(|(index, _)| {
            after_marker
                .as_bytes()
                .get(*index + 1)
                .is_some_and(|next| !next.is_ascii_whitespace())
        })
        .map(|(index, _)| index);
    let end = [next_path, next_top_level]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(after_marker.len());
    Some(&after_marker[..end])
}

fn operation(path: &str) -> &str {
    operation_block(&OPENAPI, path).unwrap_or_else(|| panic!("OpenAPI path {path} must exist"))
}

fn schema(name: &str) -> &str {
    let marker = format!("\n    {name}:\n");
    let (_, after_marker) = OPENAPI
        .split_once(&marker)
        .unwrap_or_else(|| panic!("OpenAPI schema {name} must exist"));
    let next_schema = after_marker
        .match_indices("\n    ")
        .find(|(index, _)| after_marker.as_bytes().get(*index + 5) != Some(&b' '))
        .map(|(index, _)| index)
        .unwrap_or(after_marker.len());
    &after_marker[..next_schema]
}

fn assert_schema_contains(name: &str, expected_fragments: &[&str]) {
    let actual = schema(name);
    for expected in expected_fragments {
        assert!(
            actual.contains(expected),
            "OpenAPI schema {name} must declare {expected:?}"
        );
    }
}

#[test]
fn intake_openapi_contract_covers_runtime_requests_and_responses() {
    assert_schema_contains("IntakeInitialAnswers", &["suspicious_motive:"]);
    assert_schema_contains(
        "SubmitIntakeAnswerRequest",
        &[
            "replace:",
            "default: false",
            "structured:",
            "#/components/schemas/IntakeStructuredFacts",
        ],
    );
    assert_schema_contains(
        "IntakeStructuredFacts",
        &[
            "last_seen_at:",
            "last_seen_location:",
            "follow_up_at:",
            "follow_up_location:",
            "mobility:",
            "transport_modes:",
            "companion_status:",
            "belongings:",
        ],
    );
    assert_schema_contains(
        "IntakeLocation",
        &[
            "required: [name, longitude, latitude, coordinate_system]",
            "longitude:",
            "latitude:",
        ],
    );
    assert_schema_contains("IntakeCandidateField", &["source_text:", "confidence:"]);
    assert_schema_contains(
        "IntakeSession",
        &[
            "phase:",
            "completed_phase_one_fields:",
            "missing_phase_one_fields:",
            "phase_transition_ready:",
            "enum: [collecting, ready_for_confirmation]",
        ],
    );
    assert_schema_contains(
        "SubmitIntakeAnswerResponse",
        &[
            "phase:",
            "completed_phase_one_fields:",
            "missing_phase_one_fields:",
            "phase_transition_ready:",
            "assessments:",
            "#/components/schemas/IntakeAssessment",
        ],
    );
}

#[test]
fn intake_openapi_contract_covers_draft_provenance_assessments_and_confirmation() {
    assert_schema_contains(
        "IntakeAssessment",
        &[
            "field_path:",
            "conflict_type:",
            "severity:",
            "enum: [blocking, warning, info]",
            "evidence_summary:",
            "suggested_action:",
            "route_estimate:",
        ],
    );
    assert_schema_contains(
        "IntakeRouteEstimate",
        &[
            "distance_meters:",
            "available_seconds:",
            "minimum_seconds:",
            "basis:",
            "degraded:",
        ],
    );
    assert_schema_contains(
        "IntakeProfileDraft",
        &[
            "field_metadata:",
            "assessments:",
            "confirmation_blocked_reasons:",
            "direction_hypotheses:",
        ],
    );
    assert_schema_contains(
        "IntakeProfileDraftFieldMetadata",
        &[
            "field:",
            "source_field:",
            "source:",
            "status:",
            "generated_at:",
        ],
    );
    assert_schema_contains("IntakeProfileDraftFields", &["suspicious_motive:"]);
    assert_schema_contains(
        "IntakeDirectionHypothesis",
        &["source_fields:", "uncertainty_notice:", "description:"],
    );

    let confirmed_profile = schema("ConfirmedIntakeProfile");
    assert!(confirmed_profile.contains("required: [display_name, last_seen_location]"));
    assert!(!confirmed_profile.contains("format: date-time"));
    assert_schema_contains(
        "ConfirmIntakeSessionResponse",
        &["#/components/schemas/CaseStatus"],
    );
}

#[test]
fn clue_timeline_openapi_contract_covers_pagination_and_visibility() {
    let (_, operation) = OPENAPI
        .split_once("  /api/cases/{case_id}/clues:\n")
        .expect("OpenAPI clue path must exist");
    let get_operation = operation
        .split_once("    get:\n")
        .and_then(|(_, after_get)| after_get.split_once("    post:\n").map(|(get, _)| get))
        .expect("OpenAPI clue path must declare GET before POST");
    for expected in [
        "operationId: listCaseClues",
        "name: page",
        "name: page_size",
        "name: status",
        "name: sort",
        "name: order",
        "#/components/schemas/ClueTimelinePage",
    ] {
        assert!(
            get_operation.contains(expected),
            "OpenAPI clue timeline operation must declare {expected:?}"
        );
    }
    assert_schema_contains(
        "ClueTimelinePage",
        &[
            "required: [items, page, page_size, total]",
            "items:",
            "total:",
        ],
    );
}

#[test]
fn case_places_openapi_contract_covers_role_filtered_reads() {
    let (_, operation) = OPENAPI
        .split_once("  /api/cases/{case_id}/places:\n")
        .expect("OpenAPI places path must exist");
    let get_operation = operation
        .split_once("    get:\n")
        .and_then(|(_, after_get)| after_get.split_once("    post:\n").map(|(get, _)| get))
        .expect("OpenAPI places path must declare GET before POST");
    for expected in [
        "operationId: listCasePlaces",
        "x-case-roles: [family, commander, volunteer]",
        "#/components/schemas/CasePlace",
        "\"404\": { $ref: \"#/components/responses/NotFound\" }",
    ] {
        assert!(
            get_operation.contains(expected),
            "OpenAPI places operation must declare {expected:?}"
        );
    }
}

#[test]
fn case_summary_openapi_contract_covers_role_filtered_deterministic_output() {
    let (_, operation) = OPENAPI
        .split_once("  /api/cases/{case_id}/summary:\n")
        .expect("OpenAPI case summary path must exist");
    let get_operation = operation
        .split_once("    get:\n")
        .and_then(|(_, get)| {
            get.split_once("\n  /api/cases/{case_id}/places:")
                .map(|(get, _)| get)
        })
        .expect("OpenAPI case summary path must declare GET");
    for expected in [
        "operationId: getCaseSummary",
        "x-case-roles: [family, commander, volunteer]",
        "#/components/schemas/CaseSummary",
        "\"404\": { $ref: \"#/components/responses/NotFound\" }",
    ] {
        assert!(
            get_operation.contains(expected),
            "OpenAPI case summary operation must declare {expected:?}"
        );
    }
    assert_schema_contains(
        "CaseSummary",
        &[
            "generated_at:",
            "source_scope:",
            "last_confirmed_information:",
            "pending_verification:",
            "excluded_directions:",
            "current_focus:",
            "task_status:",
            "safety_reminders:",
        ],
    );
}

#[test]
fn case_collaboration_openapi_contract_covers_public_progress_drafts_and_pois() {
    for (path, operation_id, roles, schema_name) in [
        (
            "/api/cases/{case_id}/public-progress:\n",
            "operationId: getCasePublicProgress",
            "x-case-roles: [family]",
            "#/components/schemas/CasePublicProgress",
        ),
        (
            "/api/cases/{case_id}/clue-drafts:\n",
            "operationId: createClueDrafts",
            "x-case-roles: [family, commander, volunteer]",
            "#/components/schemas/CreateClueDraftRequest",
        ),
        (
            "/api/cases/{case_id}/pois:\n",
            "operationId: listCasePois",
            "x-case-roles: [commander, volunteer]",
            "#/components/schemas/CasePois",
        ),
        (
            "/api/cases/{case_id}/summary-drafts:\n",
            "operationId: createSummaryDraft",
            "x-case-roles: [commander]",
            "#/components/schemas/SummaryDraft",
        ),
        (
            "/api/cases/{case_id}/summary-drafts/{draft_id}/review:\n",
            "operationId: reviewSummaryDraft",
            "x-case-roles: [commander]",
            "#/components/schemas/ReviewSummaryDraftRequest",
        ),
        (
            "/api/cases/{case_id}/archive-drafts:\n",
            "operationId: createCaseArchiveDraft",
            "x-case-roles: [commander]",
            "#/components/schemas/ArchiveDraft",
        ),
    ] {
        let operation = operation(path);
        for expected in [operation_id, roles, schema_name] {
            assert!(
                operation.contains(expected),
                "OpenAPI operation {path} must declare {expected:?}"
            );
        }
    }
    assert_schema_contains(
        "SummaryDraft",
        &[
            "enum: [draft, pending_review, published, rejected, withdrawn, superseded]",
            "publication_eligible:",
            "source_scope:",
        ],
    );
    assert_schema_contains(
        "CasePois",
        &["maxItems: 10", "fixed_demo_fallback", "degraded"],
    );
    assert_schema_contains(
        "ClueDraft",
        &[
            "raw_record_reference:",
            "uncertainty_notice:",
            "rule_based_fallback",
        ],
    );
    assert_schema_contains(
        "ArchiveDraft",
        &[
            "enum: [draft, pending_review, published, rejected, withdrawn]",
            "source_scope:",
            "enum: [manual_review_required, deidentified, rejected]",
            "usage_scope:",
            "retention_status:",
            "version:",
            "Internal deterministic draft content containing no raw case materials.",
        ],
    );
    let public_progress_item = schema("CasePublicProgressItem");
    assert!(public_progress_item.contains("progress_type:"));
    assert!(public_progress_item.contains("Raw clue text is never returned."));
    assert!(!public_progress_item.contains("content:"));
}

#[test]
fn archive_review_openapi_contract_requires_admin_and_manual_deidentification() {
    for (path, operation_id, schema_name, request_schema) in [
        (
            "/api/admin/archive-drafts/{draft_id}/deidentify:\n",
            "operationId: deidentifyArchiveDraft",
            "#/components/schemas/ArchiveDraft",
            "#/components/schemas/DeidentifyArchiveDraftRequest",
        ),
        (
            "/api/admin/archive-drafts/{draft_id}/review:\n",
            "operationId: reviewArchiveDraft",
            "#/components/schemas/ArchiveDraft",
            "#/components/schemas/ReviewArchiveDraftRequest",
        ),
    ] {
        let operation = operation(path);
        for expected in [
            operation_id,
            "x-global-capabilities: [admin]",
            "x-data-classification: restricted-admin",
            schema_name,
            request_schema,
        ] {
            assert!(
                operation.contains(expected),
                "OpenAPI operation {path} must declare {expected:?}"
            );
        }
    }
    assert_schema_contains(
        "DeidentifyArchiveDraftRequest",
        &["enum: [confirm, reject]", "reason:"],
    );
    assert_schema_contains(
        "ReviewArchiveDraftRequest",
        &["enum: [publish, reject, withdraw]", "reason:"],
    );
    let review_operation = operation("/api/admin/archive-drafts/{draft_id}/review:\n");
    assert!(review_operation.contains("does not expose a RAG, export, print, public"));
}

#[test]
fn admin_openapi_contract_limits_access_and_sensitive_fields() {
    for (path, operation_id, schema_name, expected_parameters) in [
        (
            "/api/admin/audit-events:\n",
            "operationId: listAdminAuditEvents",
            "#/components/schemas/AdminAuditEventPage",
            ["name: case_id", "name: from", "name: to", "name: sort"],
        ),
        (
            "/api/admin/users:\n",
            "operationId: listAdminUsers",
            "#/components/schemas/AdminUserPage",
            [
                "name: status",
                "name: account_type",
                "name: sort",
                "name: order",
            ],
        ),
        (
            "/api/admin/users/{user_id}/status:\n",
            "operationId: updateAdminUserStatus",
            "#/components/schemas/UpdateAdminUserStatusRequest",
            [
                "name: user_id",
                "x-global-capabilities: [admin]",
                "requestBody:",
                "patch:",
            ],
        ),
    ] {
        let operation = operation(path);
        for expected in [operation_id, "x-global-capabilities: [admin]", schema_name] {
            assert!(
                operation.contains(expected),
                "OpenAPI operation {path} must declare {expected:?}"
            );
        }
        for expected in expected_parameters {
            assert!(
                operation.contains(expected),
                "OpenAPI operation {path} must declare {expected:?}"
            );
        }
    }

    let audit_event = schema("AdminAuditEvent");
    assert!(!audit_event.contains("metadata_json"));
    assert_schema_contains(
        "AdminAuditEvent",
        &["actor_user_id:", "action:", "entity_type:", "created_at:"],
    );

    let admin_user = schema("AdminUser");
    assert!(!admin_user.contains("password_hash"));
    assert!(!admin_user.contains("token_hash"));
    assert_schema_contains(
        "AdminUser",
        &[
            "global_capabilities:",
            "status:",
            "created_at:",
            "last_session_at:",
        ],
    );
    assert_schema_contains(
        "UpdateAdminUserStatusRequest",
        &["enum: [active, disabled, locked]", "reason:"],
    );
}

#[test]
fn clue_attachment_openapi_contract_keeps_evidence_case_restricted() {
    let attachment_operation = operation("/api/clues/{clue_id}/attachments:\n");
    for expected in [
        "operationId: uploadClueAttachment",
        "x-case-roles: [family, commander, volunteer]",
        "x-data-classification: case-restricted",
        "multipart/form-data:",
        "#/components/schemas/CaseAttachment",
        "re-encodes the image to remove EXIF/GPS metadata",
    ] {
        assert!(
            attachment_operation.contains(expected),
            "OpenAPI clue attachment operation must declare {expected:?}"
        );
    }
}

#[test]
fn operation_blocks_stop_before_later_paths_and_top_level_sections() {
    let document = "openapi: 3.0.0\npaths:\n  /current:\n    get:\n      operationId: currentOperation\n  /later:\n    get:\n      operationId: laterOperation\ncomponents:\n  schemas:\n    LaterSchema:\n      type: object\n";

    let current =
        operation_block(document, "/current:\n").expect("current path should be extracted");
    assert!(current.contains("currentOperation"));
    assert!(!current.contains("laterOperation"));

    let later = operation_block(document, "/later:\n").expect("later path should be extracted");
    assert!(later.contains("laterOperation"));
    assert!(!later.contains("LaterSchema"));

    let final_document = "openapi: 3.0.0\npaths:\n  /final:\n    get:\n      operationId: finalOperation\ncomponents:\n  schemas:\n    FinalSchema:\n      type: object\n";
    let final_operation = operation_block(final_document, "/final:\n")
        .expect("final path should be extracted without a following path");
    assert!(final_operation.contains("finalOperation"));
    assert!(!final_operation.contains("FinalSchema"));
}

#[test]
fn task_safety_and_navigation_openapi_contract_preserves_authorized_fallbacks() {
    for (path, operation_id, schema_name) in [
        (
            "/api/tasks/{task_id}/safety-briefing:\n",
            "operationId: getTaskSafetyBriefing",
            "#/components/schemas/TaskSafetyBriefing",
        ),
        (
            "/api/tasks/{task_id}/navigation:\n",
            "operationId: getTaskNavigation",
            "#/components/schemas/TaskNavigation",
        ),
    ] {
        let operation = operation(path);
        for expected in [
            operation_id,
            "x-case-roles: [commander, volunteer]",
            schema_name,
        ] {
            assert!(
                operation.contains(expected),
                "OpenAPI task operation {path} must declare {expected:?}"
            );
        }
    }
    assert_schema_contains(
        "TaskSafetyBriefing",
        &["rule_based_fallback", "emergency_stop_message:"],
    );
    assert_schema_contains(
        "TaskNavigation",
        &["text_fallback", "navigation_url:", "fallback_message:"],
    );
}
