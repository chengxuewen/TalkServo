//! Integration-test bridge: expose assembly WITHOUT leaking logic.
//! Tests rebuild the same router from the same modules — the only duplicated
//! line is the Router::new() chain itself, never behavior.

use crate::{auth, config::Config, ws};
use std::collections::HashMap;
use std::sync::Arc;

pub use ws::App as AppState;

pub fn config_for_test(secret: &str) -> Config {
    Config::for_test(secret)
}

/// Test convenience: stub-media App (tests never run under live features).
pub fn test_app(config: Config) -> Arc<AppState> {
    let hub = crate::admin_events::EventHub::new();
    let audit = config
        .audit_log
        .as_ref()
        .map(|p| crate::audit::spawn_for_with_hub(p.as_path(), hub.clone()));
    let app = Arc::new(AppState {
        config,
        rooms: tokio::sync::Mutex::new(HashMap::new()),
        started_at: std::time::Instant::now(),
        admin_rate: std::sync::Mutex::new((std::time::Instant::now(), 0)),
        media: std::sync::Arc::new(talkservo_sfu::StubSfu::new()),
        audit,
        admin_events: hub.clone(),
        admin_ws_rate: std::sync::Mutex::new((std::time::Instant::now(), 0)),
    });
    // same refresher the real binary runs, or the admin lane would never see a
    // RoomsLive push; the task is scoped to this test process
    crate::admin_events::spawn_rooms_refresher(hub, app.clone());
    app
}

pub fn build_router(state: Arc<AppState>) -> axum::Router {
    let app = axum::Router::new()
        .route("/healthz", axum::routing::get(crate::healthz))
        .route("/ws", axum::routing::get(ws::ws_handler));
    #[cfg(feature = "embedded-web")]
    let app = {
        use axum::routing::get;
        app.route("/", get(crate::embed::index))
            .route("/{*path}", get(crate::embed::asset))
    };
    let app = app.merge(crate::admin::router());
    app.with_state(state)
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
