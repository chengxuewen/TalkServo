//! talkservo-server — signaling server binary (PoC entry point).
//!
//! Skeleton slice (plan-1): process boots, serves `/healthz`, prints version.
//! WS fan-out + JWT + SFU orchestration land with plan-2.

use axum::routing::get;
use axum::{Json, Router};

const VERSION: &str = env!("CARGO_PKG_VERSION");

async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true, "version": VERSION }))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/healthz", get(healthz));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("bind 0.0.0.0:8080");
    println!("talkservo-server v{VERSION} listening on :8080");
    axum::serve(listener, app).await.expect("server run");
}
