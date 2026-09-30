//! talkservo-server — signaling server binary (plan-2).
//!
//! Process shape (modules/03): axum+WS signaling + room mailboxes + SFU host
//! (mediasoup supervised, or stub). `/healthz` is the compose/CI readiness.

use std::collections::HashMap;
use std::sync::Arc;

use talkservo_server::{config, obs, ws};
use talkservo_sfu::Sfu as _;

const VERSION: &str = env!("CARGO_PKG_VERSION");

async fn healthz() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "ok": true, "version": VERSION }))
}

#[tokio::main]
async fn main() {
    obs::init_tracing();
    let config = config::Config::from_env();

    // SFU backend: exactly one (feature-gated; stub under stub-media).
    #[cfg(feature = "stub-media")]
    let media_handle: std::sync::Arc<talkservo_sfu::StubSfu> =
        std::sync::Arc::new(talkservo_sfu::StubSfu::new());
    #[cfg(not(feature = "stub-media"))]
    let media_handle: std::sync::Arc<talkservo_sfu::MediasoupSfu> = {
        let supervisor = std::sync::Arc::new(talkservo_sfu::Supervisor::new());
        // W-sequence step 2: worker death → MediaRestart fanout per live room.
        // (E6 measures recovery via media_recovery_ms, modules/05 §Obs.)
        let mut restarts = supervisor.restart_rx();
        let rooms_for_restart = app_state.rooms.clone();
        tokio::spawn(async move {
            let mut exited_at: Option<std::time::Instant> = None;
            loop {
                // half-open sweep: worker death is announced by kill_worker's
                // notification; mark time, then the NEXT notification (respawn)
                // closes the measurement
                if restarts.recv().await.is_err() {
                    break; // supervisor gone
                }
                let now = std::time::Instant::now();
                let t0 = exited_at.unwrap_or(now);
                let recovery_ms = now.duration_since(t0).as_millis() as u64;
                // fanout MediaRestart to every live room + log recovery
                let rooms = rooms_for_restart.lock().await;
                for (room_id, tx) in rooms.iter() {
                    let _ = tx.send(talkservo_server::ws::RoomCommand::Wire {
                        from: talkservo_core::ids::PeerId::from("system"),
                        msg: talkservo_core::wire::SignalingMessage::MediaRestart {
                            room: room_id.clone(),
                            reason: "worker rebuild".into(),
                        },
                    });
                }
                drop(rooms);
                tracing::info!(
                    event = "media_restart_done",
                    ms = recovery_ms,
                    "media_recovery_ms measured"
                );
                exited_at = None;
            }
        });
        std::sync::Arc::new(talkservo_sfu::MediasoupSfu::new(supervisor))
    };
    tracing::info!(backend = media_handle.backend(), version = VERSION, "talkservo-server starting");

    let app_state = Arc::new(ws::App {
        config,
        rooms: tokio::sync::Mutex::new(HashMap::new()),
        media: media_handle,
    });

    let app = axum::Router::new()
        .route("/healthz", axum::routing::get(healthz))
        .route("/ws", axum::routing::get(ws::ws_handler));
    #[cfg(feature = "embedded-web")]
    let app = app
        .route("/", axum::routing::get(talkservo_server::embed::index))
        .route("/*path", axum::routing::get(talkservo_server::embed::asset));
    let app = app.with_state(app_state);

    let bind = std::env::var("TS_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {bind}: {e}"));
    tracing::info!(%bind, "listening");
    axum::serve(listener, app).await.expect("server run");
}

