use std::time::Duration;

use serde::Deserialize;

#[derive(Clone)]
pub struct AmapService {
    key: Option<String>,
    base_url: String,
    client: reqwest::Client,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RouteEstimate {
    Available {
        distance_meters: u64,
        duration_seconds: u64,
        provider: &'static str,
        mode: RouteMode,
    },
    Unavailable {
        reason: RouteUnavailableReason,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteMode {
    Walking,
    Driving,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteUnavailableReason {
    NotConfigured,
    UnsupportedMode,
    TransportFailure,
    BusinessFailure,
    NoRoute,
    InvalidResponse,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PoiSearch {
    Available(Vec<Poi>),
    Unavailable,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Poi {
    pub id: String,
    pub name: String,
    pub category: String,
    pub address: Option<String>,
    pub coordinate: Option<Coordinate>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coordinate {
    pub longitude: f64,
    pub latitude: f64,
}

impl Coordinate {
    pub fn is_valid(self) -> bool {
        (-180.0..=180.0).contains(&self.longitude) && (-90.0..=90.0).contains(&self.latitude)
    }

    pub fn as_query_value(self) -> String {
        format!("{:.6},{:.6}", self.longitude, self.latitude)
    }
}

impl AmapService {
    pub fn new(
        key: Option<String>,
        base_url: String,
        timeout_ms: u64,
    ) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()?;
        Ok(Self {
            key,
            base_url: base_url.trim_end_matches('/').to_owned(),
            client,
        })
    }

    pub fn disabled() -> Self {
        Self {
            key: None,
            base_url: "https://restapi.amap.com".to_owned(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn estimate_route(
        &self,
        origin: Coordinate,
        destination: Coordinate,
        mode: RouteMode,
    ) -> RouteEstimate {
        if !origin.is_valid() || !destination.is_valid() {
            return RouteEstimate::Unavailable {
                reason: RouteUnavailableReason::InvalidResponse,
            };
        }
        let Some(key) = self.key.as_ref() else {
            return RouteEstimate::Unavailable {
                reason: RouteUnavailableReason::NotConfigured,
            };
        };
        let path = match mode {
            RouteMode::Walking => "/v3/direction/walking",
            RouteMode::Driving => "/v3/direction/driving",
        };
        let origin_value = origin.as_query_value();
        let destination_value = destination.as_query_value();
        let response = match self
            .client
            .get(format!("{}{path}", self.base_url))
            .query(&[
                ("key", key.as_str()),
                ("origin", origin_value.as_str()),
                ("destination", destination_value.as_str()),
            ])
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => response,
            Ok(_) | Err(_) => {
                return RouteEstimate::Unavailable {
                    reason: RouteUnavailableReason::TransportFailure,
                };
            }
        };
        let payload: AmapRouteResponse = match response.json().await {
            Ok(payload) => payload,
            Err(_) => {
                return RouteEstimate::Unavailable {
                    reason: RouteUnavailableReason::InvalidResponse,
                };
            }
        };
        if payload.status.as_deref() != Some("1") {
            return RouteEstimate::Unavailable {
                reason: RouteUnavailableReason::BusinessFailure,
            };
        }
        let Some(path) = payload
            .route
            .and_then(|route| route.paths.into_iter().next())
        else {
            return RouteEstimate::Unavailable {
                reason: RouteUnavailableReason::NoRoute,
            };
        };
        let (Ok(distance_meters), Ok(duration_seconds)) =
            (path.distance.parse::<u64>(), path.duration.parse::<u64>())
        else {
            return RouteEstimate::Unavailable {
                reason: RouteUnavailableReason::InvalidResponse,
            };
        };
        RouteEstimate::Available {
            distance_meters,
            duration_seconds,
            provider: "amap_webservice",
            mode,
        }
    }

    pub async fn search_nearby_pois(&self, center: Coordinate, category: &str) -> PoiSearch {
        if !center.is_valid()
            || !matches!(
                category,
                "hospital" | "police" | "transit" | "market" | "community_service"
            )
        {
            return PoiSearch::Unavailable;
        }
        let Some(key) = self.key.as_ref() else {
            return PoiSearch::Unavailable;
        };
        let types = match category {
            "hospital" => "090100",
            "police" => "130501",
            "transit" => "150500",
            "market" => "060101",
            "community_service" => "130104",
            _ => return PoiSearch::Unavailable,
        };
        let location = center.as_query_value();
        let response = match self
            .client
            .get(format!("{}/v3/place/around", self.base_url))
            .query(&[
                ("key", key.as_str()),
                ("location", location.as_str()),
                ("types", types),
                ("radius", "3000"),
                ("offset", "10"),
                ("page", "1"),
            ])
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => response,
            Ok(_) | Err(_) => return PoiSearch::Unavailable,
        };
        let payload: AmapPoiResponse = match response.json().await {
            Ok(payload) => payload,
            Err(_) => return PoiSearch::Unavailable,
        };
        if payload.status.as_deref() != Some("1") {
            return PoiSearch::Unavailable;
        }
        PoiSearch::Available(
            payload
                .pois
                .into_iter()
                .filter_map(|poi| {
                    let coordinate = poi.location.as_deref().and_then(parse_coordinate);
                    (!poi.id.trim().is_empty() && !poi.name.trim().is_empty()).then_some(Poi {
                        id: poi.id,
                        name: poi.name,
                        category: category.to_owned(),
                        address: (!poi.address.trim().is_empty()).then_some(poi.address),
                        coordinate,
                    })
                })
                .collect(),
        )
    }
}

#[derive(Deserialize)]
struct AmapRouteResponse {
    status: Option<String>,
    route: Option<AmapRoute>,
}

#[derive(Deserialize)]
struct AmapRoute {
    #[serde(default)]
    paths: Vec<AmapPath>,
}

#[derive(Deserialize)]
struct AmapPath {
    distance: String,
    duration: String,
}

#[derive(Deserialize)]
struct AmapPoiResponse {
    status: Option<String>,
    #[serde(default)]
    pois: Vec<AmapPoi>,
}

#[derive(Deserialize)]
struct AmapPoi {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    address: String,
    location: Option<String>,
}

fn parse_coordinate(value: &str) -> Option<Coordinate> {
    let (longitude, latitude) = value.split_once(',')?;
    let coordinate = Coordinate {
        longitude: longitude.parse().ok()?,
        latitude: latitude.parse().ok()?,
    };
    coordinate.is_valid().then_some(coordinate)
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::{AmapPoiResponse, AmapService, Coordinate, RouteEstimate, RouteMode};

    #[test]
    fn coordinates_use_longitude_before_latitude() {
        let coordinate = Coordinate {
            longitude: 114.48,
            latitude: 36.61,
        };
        assert!(coordinate.is_valid());
        assert_eq!(coordinate.as_query_value(), "114.480000,36.610000");
    }

    #[test]
    fn invalid_coordinates_are_rejected_before_routing() {
        assert!(
            !Coordinate {
                longitude: 181.0,
                latitude: 36.61
            }
            .is_valid()
        );
    }

    #[test]
    fn pois_with_missing_identity_fields_do_not_reject_the_whole_response() {
        let response: AmapPoiResponse = serde_json::from_str(
            r#"{
                "status": "1",
                "pois": [
                    { "id": "valid-poi", "name": "Fictional community clinic", "address": "Fictional public road", "location": "117.2272,31.8206" },
                    { "name": "Missing identifier" },
                    { "id": "missing-name" }
                ]
            }"#,
        )
        .expect("a malformed POI entry should not prevent the response from deserializing");

        assert_eq!(response.pois.len(), 3);
        assert_eq!(response.pois[0].id, "valid-poi");
        assert!(response.pois[1].id.is_empty());
        assert!(response.pois[2].name.is_empty());
    }

    #[actix_web::test]
    #[ignore = "requires AMAP_WEBSERVICE_KEY from the process environment or local .env"]
    async fn live_route_estimates_work_with_an_explicit_non_production_key() {
        crate::config::load_local_env_file()
            .expect("the local .env file must be parseable before the AMap integration test runs");
        let key = env::var("AMAP_WEBSERVICE_KEY").expect(
            "AMAP_WEBSERVICE_KEY must be set in the process environment or the local .env file",
        );
        if key.trim().is_empty() {
            panic!("AMAP_WEBSERVICE_KEY must not be empty");
        }

        let service = AmapService::new(key.into(), "https://restapi.amap.com".to_owned(), 10_000)
            .expect("a fixed AMap integration-test client should initialize");
        // Tiananmen East and Dongdan are public, nearby Beijing landmarks. The
        // coordinates are GCJ-02 and intentionally avoid application data.
        let origin = Coordinate {
            longitude: 116.404,
            latitude: 39.915,
        };
        let destination = Coordinate {
            longitude: 116.418,
            latitude: 39.914,
        };

        for mode in [RouteMode::Walking, RouteMode::Driving] {
            match service.estimate_route(origin, destination, mode).await {
                RouteEstimate::Available {
                    distance_meters,
                    duration_seconds,
                    provider,
                    mode: actual_mode,
                    ..
                } => {
                    assert_eq!(actual_mode, mode);
                    assert!(distance_meters > 0, "AMap returned a zero-distance route");
                    assert!(duration_seconds > 0, "AMap returned a zero-duration route");
                    eprintln!(
                        "AMap upstream route verified: provider={provider}, mode={}, distance_meters={distance_meters}, duration_seconds={duration_seconds}",
                        route_mode_name(actual_mode),
                    );
                }
                RouteEstimate::Unavailable { reason } => panic!(
                    "AMap live route integration failed for {mode:?}; check the non-production key's Web Service route entitlement and quota: {reason:?}"
                ),
            }
        }
    }

    fn route_mode_name(mode: RouteMode) -> &'static str {
        match mode {
            RouteMode::Walking => "walking",
            RouteMode::Driving => "driving",
        }
    }
}
