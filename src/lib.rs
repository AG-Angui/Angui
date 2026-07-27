pub mod api;
pub mod application;
pub mod config;
pub mod domain;
pub mod integrations;
pub mod persistence;

// Preserve the crate's public module paths while the implementation is grouped
// by responsibility. This keeps binaries, integration tests, and downstream
// callers source-compatible with the pre-layout-refactor API.
pub use api::{auth, error, models, rate_limit, routes};
pub use application::{app_state, services};
pub use domain::{intake_assessment, roles};
pub use integrations::{ai_gateway, amap_service};
pub use persistence::entities;
