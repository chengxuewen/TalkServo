//! Integration-test bridge: expose assembly WITHOUT leaking logic.
//! Tests rebuild the same router from the same modules — the only duplicated
//! line is the Router::new() chain itself, never behavior.

use crate::{auth, config::Config, ws};
use std::sync::Arc;

pub use ws::App as AppState;

pub fn config_for_test(secret: &str) -> Config {
    Config::for_test(secret)
}

pub fn build_router(state: Arc<AppState>) -> axum::Router {
    axum::Router::new()
        .route("/healthz", axum::routing::get(crate::healthz))
        .route("/ws", axum::routing::get(ws::ws_handler))
        .with_state(state)
}

pub fn issue_token(
    secret: &str,
    sub: &str,
    room: &str,
    role: talkservo_core::wire::Role,
    ttl_s: u64,
) -> String {
    auth::issue(secret, sub, room, role, ttl_s)
        .expect("token issuance cannot fail with a valid secret")
}

