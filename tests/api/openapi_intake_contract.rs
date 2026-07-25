use std::sync::LazyLock;

static OPENAPI: LazyLock<String> =
    LazyLock::new(|| include_str!("../../docs/openapi.yaml").replace("\r\n", "\n"));

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
