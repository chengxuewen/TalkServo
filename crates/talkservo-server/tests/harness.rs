//! Shared test harness: spin the axum app on an ephemeral port with stub SFU.
//!
//! `talkservo-server` is a binary crate — integration tests link it via the
//! `test_bridge` module gated behind `cfg(test)`-free feature `test-util`... 
//! Simplest honest approach: the harness re-builds the router from the same
//! modules (they are `pub(crate)`), so this harness duplicates ONLY the
//! assembly line (5 lines), not any logic.

#![cfg(feature = "stub-media")]

use std::collections::HashMap;
use std::sync::Arc;
use talkservo_server::test_bridge;

pub struct TestServer {
    pub url: String,
    pub secret: String,
}

impl TestServer {
    pub async fn start() -> Self {
        let secret = format!("test-secret-{}", std::process::id());
        let config = test_bridge::config_for_test(&secret);
        let app_state = Arc::new(test_bridge::AppState {
            config,
            rooms: tokio::sync::Mutex::new(HashMap::new()),
        });
        let app = test_bridge::build_router(app_state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        Self {
            url: format!("ws://{addr}/ws"),
            secret,
        }
    }

    pub fn ws_url(&self) -> &str {
        &self.url
    }
}
