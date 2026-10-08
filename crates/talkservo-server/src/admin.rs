//! Out-of-band admin REST surface (plan-5 S1, D18).
//!
//! Routes are ALWAYS mounted; fail-closed = 503 from the guard when
//! `TS_ADMIN_SECRET` is unset — conditional registration would let the
//! embedded-SPA wildcard answer `200 index.html` for /admin/api/*, which is
//! fail-OPEN wearing a closed door's costume (review BLOCKER C1).
//!
//! Fan-out discipline (shared with main.rs after S0.2): clone senders under
//! the registry lock, DROP the lock, then send+await with a per-room 1s
//! timeout; a wedged room task gets `"stale": true`, never a hung poll.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::oneshot;

use talkservo_sfu::Sfu as _;

use crate::auth;
use crate::ws::{App, RoomCommand, RoomReport};

/// Route group (merged before `.with_state(Arc<App>)` by main/test_bridge).
pub fn router() -> Router<Arc<App>> {
    Router::new()
        .route("/admin/api/overview", get(overview))
        .route("/admin/api/rooms", get(rooms))
        .route("/admin/api/tokens", post(tokens))
}

fn guard<'a>(app: &'a App, headers: &'a HeaderMap) -> Result<(), (StatusCode, &'static str)> {
    let Some(secret) = app.config.admin_secret.as_deref() else {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "admin surface disabled"));
    };
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token"))?;
    auth::validate_admin(secret, token)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid admin token"))?;
    Ok(())
}

async fn fan_out(app: &App) -> Vec<(String, serde_json::Value)> {
    let snapshot: Vec<(String, _)> = {
        let rooms = app.rooms.lock().await;
        rooms
            .iter()
            .map(|(id, tx)| (id.0.to_string(), tx.clone()))
            .collect()
    }; // lock dropped here — senders are cloned handles
    let mut out = Vec::with_capacity(snapshot.len());
    for (room, tx) in snapshot {
        let (rsp, rcp) = oneshot::channel::<RoomReport>();
        if tx.send(RoomCommand::AdminInspect(rsp)).await.is_err() {
            out.push((room, serde_json::json!({ "stale": true })));
            continue;
        }
        match tokio::time::timeout(std::time::Duration::from_secs(1), rcp).await {
            Ok(Ok(report)) => match serde_json::to_value(&report) {
                Ok(v) => out.push((room, v)),
                Err(_) => out.push((room, serde_json::json!({ "stale": true }))),
            },
            _ => out.push((room, serde_json::json!({ "stale": true }))),
        }
    }
    out
}

async fn overview(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
) -> Response {
    if let Err(e) = guard(&app, &headers) {
        return e.into_response();
    }
    let reports = fan_out(&app).await;
    let peers: usize = reports
        .iter()
        .map(|(_, r)| r.get("peers").and_then(|p| p.as_array().map(Vec::len)).unwrap_or(0))
        .sum();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "rooms": reports.len(),
            "peers": peers,
            "uptime_s": app.started_at.elapsed().as_secs(),
            "backend": app.media.backend(),
        })),
    )
        .into_response()
}

async fn rooms(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    if let Err(e) = guard(&app, &headers) {
        return e.into_response();
    }
    let reports = fan_out(&app).await;
    let body: Vec<serde_json::Value> = reports
        .into_iter()
        .map(|(room, mut r)| {
            if let Some(obj) = r.as_object_mut() {
                obj.insert("room".into(), serde_json::json!(room));
            }
            r
        })
        .collect();
    (StatusCode::OK, Json(body)).into_response()
}

#[derive(Deserialize)]
struct TokenReq {
    sub: String,
    room: String,
    role: talkservo_core::wire::Role,
    ttl_s: Option<u64>,
}

/// ids land in URL PATHS and tokens: keep the charset boring (plan-5 C3).
fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

fn rate_ok(app: &App) -> bool {
    // 60 issues/min, process-wide (adjudicated #4A; precedent: floor
    // request cooldown knob). Over-issue is the misconfig the LAN trusts.
    let mut w = app.admin_rate.lock().expect("rate window");
    if w.0.elapsed() > std::time::Duration::from_secs(60) {
        w.0 = std::time::Instant::now();
        w.1 = 0;
    }
    w.1 += 1;
    w.1 <= 60
}

fn base_url(app: &App, headers: &HeaderMap) -> String {
    if let Some(base) = &app.config.public_base {
        return base.trim_end_matches('/').to_string();
    }
    // Host-header fallback is loopback-dev ONLY (review C2): behind Caddy TLS
    // it would mint http:// links and getUserMedia dies outside secure
    // contexts. Set TS_PUBLIC_BASE with scheme for any real deployment.
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1:8080");
    tracing::warn!(
        event = "admin_public_base_unset",
        host,
        "issuing links from Host header — dev-loopback only; set TS_PUBLIC_BASE for LAN/TLS"
    );
    format!("http://{host}")
}

async fn tokens(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    body: Result<Json<TokenReq>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Err(e) = guard(&app, &headers) {
        return e.into_response();
    }
    let Json(req) = match body {
        Ok(v) => v,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "malformed body").into_response();
        }
    };
    if !valid_id(&req.sub) || !valid_id(&req.room) {
        return (
            StatusCode::BAD_REQUEST,
            "sub/room must be 1-64 chars of [A-Za-z0-9._-]",
        )
            .into_response();
    }
    if !rate_ok(&app) {
        return (StatusCode::TOO_MANY_REQUESTS, "60 issuances/min exceeded").into_response();
    }
    let ttl = req.ttl_s.unwrap_or(app.config.jwt_ttl_s).clamp(1, app.config.jwt_ttl_s);
    let jwt = match auth::issue(&app.config.jwt_secret, &req.sub, &req.room, req.role, ttl) {
        Ok(j) => j,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let base = base_url(&app, &headers);
    // the ONLY audit trail D18 allows — never the token value
    tracing::info!(
        event = "admin_token_issued",
        sub = %req.sub,
        room = %req.room,
        role = ?req.role,
        ttl_s = ttl,
    );
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "jwt": jwt,
            "field_url": format!("{base}/f/{}?jwt={jwt}", req.room),
            "dispatch_url": format!("{base}/d/{}?jwt={jwt}", req.room),
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_validation_blocks_url_structures() {
        assert!(valid_id("night-shift_1.a"));
        assert!(!valid_id(""));
        assert!(!valid_id(&"x".repeat(65)));
        assert!(!valid_id("a/b"));
        assert!(!valid_id("a?b"));
        assert!(!valid_id("a b"));
    }
}
