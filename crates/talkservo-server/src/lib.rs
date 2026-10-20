//! talkservo-server library surface — modules shared between the binary and
//! integration tests (the bin is a thin `main()` over this lib).

pub async fn healthz() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

pub mod accounts;
pub mod admin;
pub mod audit;
pub mod auth;
pub mod config;
#[cfg(feature = "embedded-web")]
pub mod embed;
pub mod kdf;
pub mod lifecycle;
pub mod media;
pub mod obs;
pub mod room;
#[cfg(feature = "stub-media")]
pub mod test_bridge;
pub mod timers;
pub mod turn;
pub mod ws;
