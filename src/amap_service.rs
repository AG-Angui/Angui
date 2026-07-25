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

#[cfg(test)]
mod tests {
    use std::env;

    use super::{AmapService, Coordinate, RouteEstimate, RouteMode};

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
                    mode: actual_mode,
                    ..
                } => {
                    assert_eq!(actual_mode, mode);
                    assert!(distance_meters > 0, "AMap returned a zero-distance route");
                    assert!(duration_seconds > 0, "AMap returned a zero-duration route");
                }
                RouteEstimate::Unavailable { reason } => panic!(
                    "AMap live route integration failed for {mode:?}; check the non-production key's Web Service route entitlement and quota: {reason:?}"
                ),
            }
        }
    }
}
