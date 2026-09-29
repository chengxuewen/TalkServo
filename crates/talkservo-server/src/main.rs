//! talkservo-server — signaling server binary (plan-2).
//!
//! Process shape (modules/03): axum+WS signaling + room mailboxes + SFU host
//! (mediasoup supervised, or stub). `/healthz` is the compose/CI readiness.

use std::collections::HashMap;
use std::sync::Arc;

use talkservo_server::{auth, config, obs, ws};

const VERSION: &str = env!("CARGO_PKG_VERSION");

async fn healthz() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "ok": true, "version": VERSION }))
}

#[tokio::main]
async fn main() {
    obs::init_tracing();
    let config = config::Config::from_env();

    // SFU backend: exactly one (feature-gated; stub under stub-media).
    let sfu = sfu_backend();
    tracing::info!(backend = sfu, version = VERSION, "talkservo-server starting");

    let app_state = Arc::new(ws::App {
        config,
        rooms: tokio::sync::Mutex::new(HashMap::new()),
    });

    let app = axum::Router::new()
        .route("/healthz", axum::routing::get(healthz))
        .route("/ws", axum::routing::get(ws::ws_handler))
        .with_state(app_state);

    let bind = std::env::var("TS_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {bind}: {e}"));
    tracing::info!(%bind, "listening");
    axum::serve(listener, app).await.expect("server run");
}

/// Build the active SFU host (exactly-one gate enforced by the sfu crate).
fn sfu_backend() -> &'static str {
    #[cfg(feature = "stub-media")]
    {
        "stub"
    }
    #[cfg(not(feature = "stub-media"))]
    {
        "mediasoup"
    }
}
