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
    let supervisor_handle: std::sync::Arc<talkservo_sfu::Supervisor> =
        std::sync::Arc::new(talkservo_sfu::Supervisor::new());
    #[cfg(not(feature = "stub-media"))]
    let media_handle: std::sync::Arc<talkservo_sfu::MediasoupSfu> =
        std::sync::Arc::new(talkservo_sfu::MediasoupSfu::new(Arc::clone(
            &supervisor_handle,
        )));
    tracing::info!(backend = media_handle.backend(), version = VERSION, "talkservo-server starting");

    let app_state = Arc::new(ws::App {
        config,
        rooms: tokio::sync::Mutex::new(HashMap::new()),
        media: media_handle,
    });

    // W-sequence step 2 (live host only): worker death → MediaRestart fanout
    // to every live room (media_recovery_ms reads the media_restart_done logs;
    // modules/05 §Obs). The 0.24 model notifies synchronously from kill —
    // the recovery measurement is the fanout latency itself.
    #[cfg(not(feature = "stub-media"))]
    {
        let rooms_registry = Arc::clone(&app_state);
        let mut restarts = supervisor_handle.restart_rx();
        tokio::spawn(async move {
            while let Ok(note) = restarts.recv().await {
                let rooms = rooms_registry.rooms.lock().await;
                for (room_id, tx) in rooms.iter() {
                    let _ = tx
                        .send(talkservo_server::ws::RoomCommand::Wire {
                            from: talkservo_core::ids::PeerId::from("system"),
                            msg: talkservo_core::wire::SignalingMessage::MediaRestart {
                                room: room_id.clone(),
                                reason: note.reason.clone(),
                            },
                        })
                        .await;
                }
                tracing::info!(
                    event = "media_restart_done",
                    reason = %note.reason,
                    rooms = rooms.len(),
                    "media restart fanned out"
                );
            }
        });
    }

    let app = axum::Router::new()
        .route("/healthz", axum::routing::get(healthz))
        .route("/ws", axum::routing::get(ws::ws_handler));
    #[cfg(feature = "embedded-web")]
    let app = app
        .route("/", axum::routing::get(talkservo_server::embed::index))
        .route("/{*path}", axum::routing::get(talkservo_server::embed::asset));
    let app = app.with_state(app_state);

    let bind = std::env::var("TS_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {bind}: {e}"));
    tracing::info!(%bind, "listening");
    axum::serve(listener, app).await.expect("server run");
}

