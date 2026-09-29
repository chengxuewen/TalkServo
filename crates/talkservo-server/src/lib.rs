//! talkservo-server library surface — modules shared between the binary and
//! integration tests (the bin is a thin `main()` over this lib).

pub async fn healthz() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

pub mod auth;
pub mod config;
pub mod obs;
pub mod room;
pub mod test_bridge;
pub mod ws;
