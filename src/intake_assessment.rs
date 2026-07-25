use chrono::DateTime;

use crate::{
    amap_service::{AmapService, Coordinate, RouteEstimate, RouteMode, RouteUnavailableReason},
    models::{IntakeAssessment, IntakeRouteEstimate, IntakeStructuredFacts},
};

pub async fn evaluate(
    facts: &IntakeStructuredFacts,
    amap_service: &AmapService,
) -> Vec<IntakeAssessment> {
    let mut assessments = validate_controlled_values(facts);
    let Some(last_seen_at) = parse_time(&facts.last_seen_at, "last_seen_at", &mut assessments)
    else {
        return assessments;
    };
    let Some(follow_up_at) = parse_time(&facts.follow_up_at, "follow_up_at", &mut assessments)
    else {
        return assessments;
    };
    let (Some(origin), Some(destination)) = (&facts.last_seen_location, &facts.follow_up_location)
    else {
        return assessments;
    };
    let Some(origin_coordinate) = coordinate(
        origin.longitude,
        origin.latitude,
        "last_seen_location",
        &mut assessments,
    ) else {
        return assessments;
    };
    let Some(destination_coordinate) = coordinate(
        destination.longitude,
        destination.latitude,
        "follow_up_location",
        &mut assessments,
    ) else {
        return assessments;
    };
    if origin.coordinate_system != "gcj02" || destination.coordinate_system != "gcj02" {
        assessments.push(info(
            "structured.location.coordinate_system",
            "coordinate_system_unverified",
            "The route estimate requires GCJ-02 coordinates; please confirm the source coordinate system.",
            "Confirm the coordinate system before relying on route estimates.",
        ));
        return assessments;
    }
    let available_seconds = follow_up_at.timestamp() - last_seen_at.timestamp();
    if available_seconds < 0 {
        assessments.push(blocking(
            "structured.follow_up_at",
            "time_order_impossible",
            "The follow-up time is earlier than the last-seen time.",
            "Check the two times or mark the uncertain time as unknown.",
        ));
        return assessments;
    }
    let straight_line_meters = haversine_meters(origin_coordinate, destination_coordinate);
    let walking_only = facts.transport_modes.len() == 1 && facts.transport_modes[0] == "walking";
    let mode = walking_only.then_some(RouteMode::Walking);
    let route = match mode {
        Some(mode) => {
            amap_service
                .estimate_route(origin_coordinate, destination_coordinate, mode)
                .await
        }
        None => RouteEstimate::Unavailable {
            reason: RouteUnavailableReason::UnsupportedMode,
        },
    };
    let (minimum_seconds, distance_meters, basis, degraded) = match route {
        RouteEstimate::Available {
            distance_meters,
            duration_seconds,
            provider,
            ..
        } => (
            Some(duration_seconds),
            distance_meters,
            provider.to_owned(),
            false,
        ),
        RouteEstimate::Unavailable { reason } => {
            let fallback_seconds = (straight_line_meters / 1.2).ceil() as u64;
            (
                Some(fallback_seconds),
                straight_line_meters as u64,
                format!("straight_line_fallback:{reason:?}"),
                true,
            )
        }
    };
    let estimate = IntakeRouteEstimate {
        distance_meters,
        available_seconds,
        minimum_seconds,
        basis,
        degraded,
    };
    if !walking_only {
        assessments.push(info_with_route(
            "structured.transport_modes",
            "transport_mode_unknown_or_non_walking",
            "The available transport mode does not support a reliable walking-only reachability conclusion.",
            "Confirm whether the person was walking, used transport, or had a companion.",
            estimate,
        ));
    } else if degraded {
        assessments.push(warning_with_route(
            "structured.follow_up_location",
            "route_service_degraded",
            "Route service is unavailable, so the estimate uses straight-line distance and must not be treated as a route result.",
            "Confirm the locations, transport mode, and time precision before relying on this estimate.",
            estimate,
        ));
    } else if minimum_seconds.is_some_and(|seconds| seconds > available_seconds as u64) {
        assessments.push(blocking_with_route(
            "structured.follow_up_location",
            "walking_reachability_conflict",
            "The estimated minimum walking time exceeds the available time between the two reports.",
            "Check the times, locations, transport mode, or whether another person provided transport.",
            estimate,
        ));
    }
    assessments
}

fn validate_controlled_values(facts: &IntakeStructuredFacts) -> Vec<IntakeAssessment> {
    let mut assessments = Vec::new();
    for mode in &facts.transport_modes {
        if !matches!(
            mode.as_str(),
            "walking" | "driving" | "public_transit" | "unknown"
        ) {
            assessments.push(blocking(
                "structured.transport_modes",
                "unsupported_transport_mode",
                "A transport mode is outside the supported controlled values.",
                "Use walking, driving, public_transit, or unknown.",
            ));
        }
    }
    if let Some(mobility) = &facts.mobility
        && !matches!(mobility.as_str(), "limited" | "independent" | "unknown")
    {
        assessments.push(blocking(
            "structured.mobility",
            "unsupported_mobility",
            "The mobility value is outside the supported controlled values.",
            "Use limited, independent, or unknown.",
        ));
    }
    if let Some(companion_status) = &facts.companion_status
        && !matches!(
            companion_status.as_str(),
            "alone" | "accompanied" | "unknown"
        )
    {
        assessments.push(blocking(
            "structured.companion_status",
            "unsupported_companion_status",
            "The companion status is outside the supported controlled values.",
            "Use alone, accompanied, or unknown.",
        ));
    }
    if facts.mobility.as_deref() == Some("limited")
        && facts.transport_modes.len() == 1
        && facts.transport_modes[0] == "walking"
    {
        assessments.push(warning(
            "structured.transport_modes",
            "mobility_transport_tension",
            "Limited mobility and walking-only travel may need clarification.",
            "Confirm mobility, assistance, companion status, and transport before relying on travel assumptions.",
        ));
    }
    let duplicates = facts
        .belongings
        .iter()
        .map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if duplicates.len()
        != duplicates
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
    {
        assessments.push(warning(
            "structured.belongings",
            "duplicate_belonging",
            "The belongings list contains duplicate descriptions.",
            "Remove duplicates or clarify whether the items are distinct.",
        ));
    }
    assessments
}

fn parse_time(
    value: &Option<String>,
    field: &str,
    assessments: &mut Vec<IntakeAssessment>,
) -> Option<DateTime<chrono::FixedOffset>> {
    let value = value.as_ref()?;
    match DateTime::parse_from_rfc3339(value) {
        Ok(time) => Some(time),
        Err(_) => {
            assessments.push(blocking(
                &format!("structured.{field}"),
                "invalid_timestamp",
                "The timestamp must use RFC 3339 with an explicit offset.",
                "Correct the time or omit the structured value and describe the uncertainty in the answer.",
            ));
            None
        }
    }
}

fn coordinate(
    longitude: f64,
    latitude: f64,
    field: &str,
    assessments: &mut Vec<IntakeAssessment>,
) -> Option<Coordinate> {
    let coordinate = Coordinate {
        longitude,
        latitude,
    };
    if coordinate.is_valid() {
        Some(coordinate)
    } else {
        assessments.push(blocking(
            &format!("structured.{field}"),
            "coordinate_out_of_range",
            "The supplied longitude or latitude is outside its valid range.",
            "Correct the coordinate or remove it and provide the place name as unconfirmed text.",
        ));
        None
    }
}

fn haversine_meters(origin: Coordinate, destination: Coordinate) -> f64 {
    let latitude_delta = (destination.latitude - origin.latitude).to_radians();
    let longitude_delta = (destination.longitude - origin.longitude).to_radians();
    let origin_latitude = origin.latitude.to_radians();
    let destination_latitude = destination.latitude.to_radians();
    let value = (latitude_delta / 2.0).sin().powi(2)
        + origin_latitude.cos()
            * destination_latitude.cos()
            * (longitude_delta / 2.0).sin().powi(2);
    6_371_000.0 * 2.0 * value.sqrt().atan2((1.0 - value).sqrt())
}

fn assessment(
    field_path: impl Into<String>,
    conflict_type: impl Into<String>,
    severity: &str,
    evidence_summary: impl Into<String>,
    suggested_action: impl Into<String>,
    route_estimate: Option<IntakeRouteEstimate>,
) -> IntakeAssessment {
    IntakeAssessment {
        field_path: field_path.into(),
        conflict_type: conflict_type.into(),
        severity: severity.to_owned(),
        evidence_summary: evidence_summary.into(),
        suggested_action: suggested_action.into(),
        route_estimate,
    }
}

fn blocking(field: &str, kind: &str, evidence: &str, action: &str) -> IntakeAssessment {
    assessment(field, kind, "blocking", evidence, action, None)
}
fn warning(field: &str, kind: &str, evidence: &str, action: &str) -> IntakeAssessment {
    assessment(field, kind, "warning", evidence, action, None)
}
fn info(field: &str, kind: &str, evidence: &str, action: &str) -> IntakeAssessment {
    assessment(field, kind, "info", evidence, action, None)
}
fn blocking_with_route(
    field: &str,
    kind: &str,
    evidence: &str,
    action: &str,
    route: IntakeRouteEstimate,
) -> IntakeAssessment {
    assessment(field, kind, "blocking", evidence, action, Some(route))
}
fn warning_with_route(
    field: &str,
    kind: &str,
    evidence: &str,
    action: &str,
    route: IntakeRouteEstimate,
) -> IntakeAssessment {
    assessment(field, kind, "warning", evidence, action, Some(route))
}
fn info_with_route(
    field: &str,
    kind: &str,
    evidence: &str,
    action: &str,
    route: IntakeRouteEstimate,
) -> IntakeAssessment {
    assessment(field, kind, "info", evidence, action, Some(route))
}

#[cfg(test)]
mod tests {
    use super::evaluate;
    use crate::{
        amap_service::AmapService,
        models::{IntakeLocation, IntakeStructuredFacts},
    };

    fn location(name: &str, longitude: f64, latitude: f64) -> IntakeLocation {
        IntakeLocation {
            name: name.to_owned(),
            longitude,
            latitude,
            coordinate_system: "gcj02".to_owned(),
        }
    }

    #[actix_web::test]
    async fn walking_time_conflict_uses_explicit_degraded_warning_when_map_is_disabled() {
        let assessment = evaluate(
            &IntakeStructuredFacts {
                last_seen_at: Some("2026-07-25T15:00:00+08:00".to_owned()),
                follow_up_at: Some("2026-07-25T15:20:00+08:00".to_owned()),
                last_seen_location: Some(location("Fictional origin", 114.48, 36.61)),
                follow_up_location: Some(location("Fictional destination", 114.58, 36.61)),
                transport_modes: vec!["walking".to_owned()],
                ..Default::default()
            },
            &AmapService::disabled(),
        )
        .await;
        assert!(
            assessment
                .iter()
                .any(|result| result.conflict_type == "route_service_degraded")
        );
        assert!(
            !assessment
                .iter()
                .any(|result| result.severity == "blocking")
        );
    }

    #[actix_web::test]
    async fn invalid_structured_values_are_blocking_while_mobility_tension_is_a_warning() {
        let assessment = evaluate(
            &IntakeStructuredFacts {
                mobility: Some("limited".to_owned()),
                companion_status: Some("unverified".to_owned()),
                transport_modes: vec!["walking".to_owned()],
                ..Default::default()
            },
            &AmapService::disabled(),
        )
        .await;

        assert!(assessment.iter().any(|result| {
            result.conflict_type == "unsupported_companion_status" && result.severity == "blocking"
        }));
        assert!(assessment.iter().any(|result| {
            result.conflict_type == "mobility_transport_tension" && result.severity == "warning"
        }));
    }
}
