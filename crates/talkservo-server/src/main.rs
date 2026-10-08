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

const USAGE: &str = "usage: talkservo-server <init|start|stop|restart|status|logs|monit|ps|run|version> [<dir>]  (instance dir default .server/)
  init <dir>              assemble an instance dir: etc/server.yaml + identity/env (0600, idempotent secrets) + run/oxfile.toml
  start|stop|restart <dir>  drive the process cluster via oxmgr (shipped by the deploy package / PATH)
  status <dir>            healthz probe (exit 0 healthy / 2 not running)
  logs|monit|ps <dir> [args...]  oxmgr pass-through
  run [--config path]     foreground/daemon direct start — systemd/compose tracks untouched (env > yaml > defaults)
  version | -h";

#[tokio::main]
async fn main() {
    obs::init_tracing();
    // ---- D19 facade dispatch (run / legacy flags fall through to server mode) ----
    {
        let argv: Vec<String> = std::env::args().skip(1).collect();
        let sub = argv.first().map(|s| s.as_str()).unwrap_or("run");
        let dir = std::path::PathBuf::from(argv.get(1).cloned().unwrap_or_else(|| ".server".into()));
        let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("talkservo-server"));
        let code = match sub {
            "init" => {
                let dir = std::path::PathBuf::from(argv.get(1).cloned().unwrap_or_else(|| ".server".into()));
                match crate_ok(talkservo_server::lifecycle::init_instance(&dir, &exe)) {
                    Ok(lines) => {
                        for l in lines {
                            println!("init: {l}");
                        }
                        0
                    }
                    Err(e) => {
                        eprintln!("init failed: {e}");
                        1
                    }
                }
            }
            "start" => talkservo_server::lifecycle::oxmgr_forward(&dir, &["apply"]),
            "stop" => talkservo_server::lifecycle::oxmgr_stop_cluster(&dir),
            "restart" => {
                let _ = talkservo_server::lifecycle::oxmgr_stop_cluster(&dir);
                talkservo_server::lifecycle::oxmgr_forward(&dir, &["apply"])
            }
            "status" => {
                let bind = talkservo_server::lifecycle::instance_bind(&dir);
                if talkservo_server::lifecycle::probe_healthz(&bind, std::time::Duration::from_millis(800)) {
                    println!("status: healthy ({bind})");
                    0
                } else {
                    println!("status: NOT running (bind {bind})");
                    2
                }
            }
            "monit" | "ui" => talkservo_server::lifecycle::oxmgr_forward(&dir, &["ui"]),
            "ps" => talkservo_server::lifecycle::oxmgr_forward(&dir, &["list"]),
            "logs" => {
                let svc = argv.get(2).cloned().unwrap_or_else(|| talkservo_server::lifecycle::SERVER_APP.into());
                talkservo_server::lifecycle::oxmgr_forward(&dir, &["logs", &svc])
            }
            "version" => {
                println!("talkservo-server {VERSION}");
                0
            }
            "-h" | "--help" | "help" => {
                println!("{USAGE}");
                0
            }
            "run" => {
                // server mode: --config <path> → YAML overlay, else pure env
                let cfg_arg = argv
                    .iter()
                    .position(|a| a == "--config")
                    .and_then(|i| argv.get(i + 1))
                    .cloned();
                let config = match cfg_arg {
                    Some(p) => config::Config::load_yaml(std::path::Path::new(&p))
                        .unwrap_or_else(|e| {
                            eprintln!("FATAL: {e}");
                            std::process::exit(1);
                        }),
                    None => config::Config::from_env(),
                };
                serve(config).await;
                return;
            }
            other => {
                // legacy flags straight through (--config kept working)
                let _ = other;
                let config = if argv.iter().any(|a| a == "--config") {
                    let p = argv
                        .iter()
                        .position(|a| a == "--config")
                        .and_then(|i| argv.get(i + 1))
                        .expect("--config needs a value");
                    config::Config::load_yaml(std::path::Path::new(p))
                        .unwrap_or_else(|e| {
                            eprintln!("FATAL: {e}");
                            std::process::exit(1);
                        })
                } else {
                    config::Config::from_env()
                };
                serve(config).await;
                return;
            }
        };
        std::process::exit(code);
    }
}

fn crate_ok<T, E: std::fmt::Display>(r: Result<T, E>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

async fn serve(config: config::Config) {

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

    if config.admin_secret.is_some() && config.admin_secret.as_deref() == Some(config.jwt_secret.as_str()) {
        eprintln!("FATAL: TS_ADMIN_SECRET must differ from TS_JWT_SECRET (independent rotation, D18)");
        std::process::exit(1);
    }
    let app_state = Arc::new(ws::App {
        config,
        rooms: tokio::sync::Mutex::new(HashMap::new()),
        started_at: std::time::Instant::now(),
        admin_rate: std::sync::Mutex::new((std::time::Instant::now(), 0)),
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
                // clone-senders -> drop lock -> send: room tasks reap
                // themselves under the SAME registry lock (ws.rs IdleTick);
                // holding it across send().await is a deadlock window
                let snapshot: Vec<_> = {
                    let rooms = rooms_registry.rooms.lock().await;
                    rooms.iter().map(|(id, tx)| (id.clone(), tx.clone())).collect()
                };
                for (room_id, tx) in &snapshot {
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
                    rooms = snapshot.len(),
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
    // admin surface always mounted (fail-closed by guard, plan-5 S1)
    let app = app.merge(talkservo_server::admin::router()).with_state(app_state);

    let bind = std::env::var("TS_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {bind}: {e}"));
    tracing::info!(%bind, "listening");
    axum::serve(listener, app).await.expect("server run");
}

