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

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
// (no futures_util import: axum's WebSocket has inherent recv/send, and an unused
// trait import fails gate-clippy's -D warnings)
use crate::admin_events::{
    ADMIN_WS_CONNECTS_PER_MIN, AUTH_DEADLINE, AUTH_FRAME_MAX_BYTES, AdminIn, EventHub,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::oneshot;

use talkservo_sfu::Sfu as _;

use crate::auth;
use crate::ws::{App, RoomCommand, RoomReport};

/// Route group (merged before `.with_state(Arc<App>)` by main/test_bridge).
pub fn router() -> Router<Arc<App>> {
    Router::new()
        .route("/admin/api/overview", get(overview))
        .route("/admin/api/rooms", get(rooms_registry).post(rooms_create))
        .route("/admin/api/rooms/live", get(rooms_live))
        .route("/admin/ws", get(admin_ws))
        .route(
            "/admin/api/rooms/{room}",
            axum::routing::delete(rooms_delete),
        )
        .route(
            "/admin/api/rooms/{room}/peers/{peer}",
            axum::routing::delete(peers_kick),
        )
        .route(
            "/admin/api/rooms/{room}/peers/{peer}/force-release",
            post(peers_force_release),
        )
        .route("/admin/api/tokens", post(tokens))
        .route("/admin/api/audit", get(audit_query))
        .route("/admin/api/login", post(login))
        .route(
            "/admin/api/accounts",
            get(accounts_list).post(accounts_create),
        )
        .route(
            "/admin/api/accounts/{user}",
            axum::routing::delete(accounts_delete),
        )
}

fn guard<'a>(app: &'a App, headers: &'a HeaderMap) -> Result<(), (StatusCode, &'static str)> {
    guard_role(app, headers).map(|_| ())
}

/// S5: tiered guard - full claims on a valid console session (S6 needs `sub`
/// for audit attribution; tier checks read `.role`).
fn guard_claims(
    app: &App,
    headers: &HeaderMap,
) -> Result<auth::AdminClaims, (StatusCode, &'static str)> {
    let Some(secret) = app.config.admin_secret.as_deref() else {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "admin surface disabled"));
    };
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token"))?;
    let claims = auth::validate_admin(secret, token)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid admin token"))?;
    Ok(claims)
}

fn guard_role(app: &App, headers: &HeaderMap) -> Result<String, (StatusCode, &'static str)> {
    guard_claims(app, headers).map(|c| c.role)
}

/// ADMIN-only write gate (accounts + rooms registry config): operator 403.
/// Fail-closed: an unknown/absent role string is NOT admin.
fn require_admin(app: &App, headers: &HeaderMap) -> Result<(), (StatusCode, &'static str)> {
    let role = guard_role(app, headers)?;
    if role == "admin" {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "admin tier required"))
    }
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

async fn overview(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    if let Err(e) = guard(&app, &headers) {
        return e.into_response();
    }
    let reports = fan_out(&app).await;
    let peers: usize = reports
        .iter()
        .map(|(_, r)| {
            r.get("peers")
                .and_then(|p| p.as_array().map(Vec::len))
                .unwrap_or(0)
        })
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

async fn rooms_live(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    if let Err(e) = guard(&app, &headers) {
        return e.into_response();
    }
    let body = rooms_body(&app).await;
    (StatusCode::OK, Json(body)).into_response()
}

/// The `/admin/api/rooms/live` body, shared by the REST handler and the /admin/ws
/// RoomsLive pusher (plan-9 W1): one serializer for both, so a pushed snapshot and
/// a polled one can never disagree about shape.
pub async fn rooms_body(app: &Arc<App>) -> Vec<serde_json::Value> {
    fan_out(app)
        .await
        .into_iter()
        .map(|(room, mut r)| {
            if let Some(obj) = r.as_object_mut() {
                obj.insert("room".into(), serde_json::json!(room));
            }
            r
        })
        .collect()
}

#[derive(Deserialize)]
struct TokenReq {
    /// OMITTED (not empty) = server generates u-xxxxxx at issue time (D29-#4). The
    /// serde default is the contract: the wire shape is absence, and the empty-string
    /// branch below is belt-and-suspenders for callers that send `""`.
    #[serde(default)]
    sub: String,
    room: String,
    role: talkservo_core::wire::Role,
    ttl_s: Option<u64>,
    /// S4: rooms this token may govern (each name validated like `room`).
    #[serde(default)]
    mod_rooms: Vec<String>,
}

/// ids land in URL PATHS and tokens: keep the charset boring (plan-5 C3).
fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// D29-#4: server-side subject autogeneration — `u-` + 6 base36 chars (base36 =
/// lowercase+digits; no role in the identity: a participant changing roles must not
/// change identity; no sequence: it leaks issuance volume). Collision-checked against
/// `occupied` (live seats + registry rows) with a bounded retry; exhaustion is a 500,
/// not a silent duplicate. Randomness = std hash of (time nanos, address entropy, and
/// an atomic counter) — no new dependency for a 36^6 space with an 8-try bound.
fn gen_subject(occupied: &std::collections::HashSet<String>) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    for _ in 0..8 {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let mix = t
            ^ (COUNTER.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed))
            ^ (&COUNTER as *const _ as u64);
        // splitmix64 finish for diffusion
        let mut z = mix.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        const ALPHABET: &[u8; 36] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let s: String = (0..6)
            .map(|i| ALPHABET[((z >> (i * 6)) & 35) as usize] as char)
            .collect();
        let candidate = format!("u-{s}");
        if !occupied.contains(&candidate) {
            return candidate;
        }
    }
    // caller turns this into a 500 — never a silent duplicate seat
    String::new()
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

#[derive(Deserialize)]
struct LoginReq {
    user: String,
    password: String,
}

/// Password login (D22, sister `POST /api/auth/login` shape): verify the
/// PBKDF2 hash from server.yaml, mint a SHORT-lived admin JWT server-side.
/// Rate-limited ahead of verification (brute-force budget); fail-closed 503
/// when hash or signing secret is unset. The password is never logged.
async fn login(
    State(app): State<Arc<App>>,
    body: Result<Json<LoginReq>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if !rate_ok(&app) {
        return (StatusCode::TOO_MANY_REQUESTS, "login rate exceeded").into_response();
    }
    let Json(req) = match body {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "malformed body").into_response(),
    };
    let Some(secret) = app.config.admin_secret.as_deref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "admin surface disabled").into_response();
    };
    if app.config.admin_password_hash.is_none()
        && app
            .config
            .accounts_file
            .as_ref()
            .is_none_or(|p| crate::accounts::load(p).is_empty())
    {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "no admin credentials configured",
        )
            .into_response();
    }
    // user/pwd against the accounts registry (D22 sister shape: implicit
    // "admin" from server.yaml until accounts.yaml is materialized)
    if !crate::accounts::authenticate(&app.config, &req.user, &req.password) {
        tracing::warn!(event = "admin_login_failed", user = %req.user, "rejected credentials");
        if let Some(a) = &app.audit {
            a.emit(
                crate::audit::AuditEvent::new(crate::audit::AuditKind::LoginFail).actor(req.user),
            );
        }
        return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
    }
    // S5: mint with the registry tier (role is whatever the row says; rows
    // lacking a role field deserialize to "admin" via accounts default).
    // implicit-admin fallback (server.yaml hash, no registry row yet) is tier
    // "admin" by construction; registry rows answer with their own role.
    let role = app
        .config
        .accounts_file
        .as_ref()
        .and_then(|p| {
            crate::accounts::load(p)
                .into_iter()
                .find(|a| a.user == req.user)
        })
        .map(|a| a.role)
        .unwrap_or_else(|| "admin".into());
    match auth::issue_admin_role(secret, &req.user, app.config.admin_login_ttl_s, &role) {
        Ok(jwt) => {
            tracing::info!(
                event = "admin_login",
                ttl_s = app.config.admin_login_ttl_s,
                "session minted"
            );
            Json(serde_json::json!({ "jwt": jwt, "role": role })).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "token mint failed").into_response(),
    }
}

#[derive(Deserialize)]
struct AccountReq {
    user: String,
    password: String,
    /// S5 create-time tier (two-value; absent = "admin" default). Ignored on
    /// reset - existing rows keep their stored role.
    #[serde(default)]
    role: Option<String>,
}

/// Registry listing — users and roles, never hashes (sister parity).
async fn accounts_list(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    if let Err(e) = guard(&app, &headers) {
        return e.into_response();
    }
    Json(serde_json::json!(crate::accounts::list_view(&app.config))).into_response()
}

/// Create-or-reset (upsert = password change; PoC has no separate PUT).
async fn accounts_create(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    body: Result<Json<AccountReq>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Err(e) = require_admin(&app, &headers) {
        return e.into_response();
    }
    let Json(req) = match body {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "malformed body").into_response(),
    };
    if !crate::accounts::valid_user(&req.user) {
        return (
            StatusCode::BAD_REQUEST,
            "user must be 1-64 chars of [A-Za-z0-9._-]",
        )
            .into_response();
    }
    if req.password.len() < 8 {
        return (StatusCode::BAD_REQUEST, "password must be at least 8 chars").into_response();
    }
    if let Some(r) = req.role.as_deref()
        && r != "admin"
        && r != "operator"
    {
        return (StatusCode::BAD_REQUEST, "role must be admin or operator").into_response();
    }
    match crate::accounts::upsert_role(&app.config, &req.user, &req.password, req.role.as_deref()) {
        Ok(()) => {
            tracing::info!(event = "admin_account_upsert", user = %req.user, "registry written");
            (StatusCode::CREATED, "stored").into_response()
        }
        Err(e) => {
            tracing::warn!(event = "admin_account_write_failed", "{e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "registry write failed").into_response()
        }
    }
}

async fn accounts_delete(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    axum::extract::Path(user): axum::extract::Path<String>,
) -> Response {
    if let Err(e) = require_admin(&app, &headers) {
        return e.into_response();
    }
    match crate::accounts::remove(&app.config, &user) {
        Ok(0) => (StatusCode::NOT_FOUND, "no such account").into_response(),
        Ok(_) => {
            tracing::info!(event = "admin_account_removed", user, "registry written");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            tracing::warn!(event = "admin_account_write_failed", "{e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "registry write failed").into_response()
        }
    }
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
    // D29-#4: empty sub = server-generated subject (fixed at issue time — it travels
    // with the link so the D23 chain — audit/kick/mute — keeps working). Explicit
    // invalid subs still 400 below.
    let sub = if req.sub.is_empty() {
        let mut occupied: std::collections::HashSet<String> = crate::rooms::load(&app.config)
            .into_iter()
            .map(|r| format!("u-{}", r.room))
            .collect();
        // live seats too: a generated sub equal to a joined subject would hijack
        // that identity's re-entry. rooms_body() already fans out every live room's
        // peer table — harvest subject ids from it (same source the console shows).
        for row in rooms_body(&app).await {
            if let Some(peers) = row.get("peers").and_then(|p| p.as_array()) {
                for peer in peers {
                    if let Some(pid) = peer.get("id").and_then(|v| v.as_str()) {
                        occupied.insert(pid.to_string());
                    }
                }
            }
        }
        let g = gen_subject(&occupied);
        if g.is_empty() {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "subject generation failed (collision bound)",
            )
                .into_response();
        }
        g
    } else {
        req.sub.clone()
    };
    if !valid_id(&sub) || !valid_id(&req.room) {
        return (
            StatusCode::BAD_REQUEST,
            "sub/room must be 1-64 chars of [A-Za-z0-9._-]",
        )
            .into_response();
    }
    if !rate_ok(&app) {
        return (StatusCode::TOO_MANY_REQUESTS, "60 issuances/min exceeded").into_response();
    }
    let ttl = req
        .ttl_s
        .unwrap_or(app.config.jwt_ttl_s)
        .clamp(1, app.config.jwt_ttl_s);
    for r in &req.mod_rooms {
        if !valid_id(r) {
            return (
                StatusCode::BAD_REQUEST,
                "mod_rooms entries must match the id charset",
            )
                .into_response();
        }
    }
    let jwt = match auth::issue_with_mods(
        &app.config.jwt_secret,
        &sub,
        &req.room,
        req.role,
        ttl,
        &req.mod_rooms,
    ) {
        Ok(j) => j,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let base = base_url(&app, &headers);
    // the ONLY audit trail D18 allows — never the token value
    tracing::info!(
        event = "admin_token_issued",
        sub = %sub,
        room = %req.room,
        role = ?req.role,
        ttl_s = ttl,
    );
    if let Some(a) = &app.audit {
        a.emit(
            crate::audit::AuditEvent::new(crate::audit::AuditKind::TokenIssue)
                .room(req.room.clone())
                .actor(sub.clone())
                .change(
                    "grant",
                    serde_json::Value::Null,
                    serde_json::json!({"role": format!("{:?}", req.role), "ttl_s": ttl}),
                ),
        );
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "jwt": jwt,
            "field_url": format!("{base}/field/{}?jwt={jwt}", req.room),
            "dispatch_url": format!("{base}/dispatch/{}?jwt={jwt}", req.room),
            "participant_url": format!("{base}/room/{}?jwt={jwt}", req.room),
        })),
    )
        .into_response()
}

/// GET /admin/api/audit - JSONL trail reader behind the same admin guard.
/// Cursor paging (seq before/after) + room/actor/event filters; limit is
/// clamped 1..=1000 (doc-audit GAP-8); `dropped` surfaces writer failures (R4).
async fn audit_query(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    if let Err(e) = guard(&app, &headers) {
        return e.into_response();
    }
    let Some(path) = &app.config.audit_log else {
        return (StatusCode::SERVICE_UNAVAILABLE, "audit log disabled").into_response();
    };
    let num = |k: &str| params.get(k).and_then(|v| v.parse::<u64>().ok());
    let q = crate::audit::AuditQuery {
        before: num("before"),
        after: num("after"),
        room: params.get("room").cloned(),
        actor: params.get("actor").cloned(),
        event: params.get("event").cloned(),
        limit: num("limit"),
    };
    let events = crate::audit::query(path, &q);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "events": events,
            "dropped": crate::audit::AUDIT_DROPPED.load(std::sync::atomic::Ordering::Relaxed),
        })),
    )
        .into_response()
}

/// S6 intervention kit (D23-④ / R2 contract): optional reason lands in the
/// AUDIT line only; the survivor broadcast is the S1 leave relay with the
/// additive kind=removed marker - no reason/actor over the wire.
/// Console tier: operator may intervene (moderator duty); accounts/registry
/// writes stay admin-only (S5).
#[derive(Deserialize)]
struct ActionReq {
    #[serde(default)]
    reason: Option<String>,
}

/// R2/Jitsi clause: reason <=512; over-limit reasons are rejected 400 at
/// the endpoint (Box-free small error keeps clippy result_large_err quiet).
fn action_reason(body: &Option<Json<ActionReq>>) -> Option<String> {
    body.as_ref().and_then(|Json(a)| a.reason.clone())
}
fn reason_overlong(reason: &Option<String>) -> bool {
    reason.as_ref().is_some_and(|r| r.len() > 512)
}

async fn peers_kick(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Path((room, peer)): Path<(String, String)>,
    body: Option<Json<ActionReq>>,
) -> Response {
    let claims = match guard_claims(&app, &headers) {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    if !crate::rooms::valid_room(&room) || !valid_id(&peer) {
        return (StatusCode::NOT_FOUND, "unknown room or peer").into_response();
    }
    let reason = action_reason(&body);
    if reason_overlong(&reason) {
        return (
            StatusCode::BAD_REQUEST,
            "reason too long (max 512, Discord clause)",
        )
            .into_response();
    }
    let tx = {
        let guard = app.rooms.lock().await;
        guard
            .get(&talkservo_core::ids::RoomId::from(room.as_str()))
            .cloned()
    };
    let Some(tx) = tx else {
        return (StatusCode::NOT_FOUND, "room is not live").into_response();
    };
    let (rsp, rcp) = tokio::sync::oneshot::channel::<bool>();
    if tx
        .send(crate::ws::RoomCommand::KickPeer {
            peer: talkservo_core::ids::PeerId::from(peer.as_str()),
            reply: rsp,
        })
        .await
        .is_err()
    {
        return (StatusCode::NOT_FOUND, "room is not live").into_response();
    }
    match rcp.await {
        Ok(true) => {
            if let Some(a) = &app.audit {
                a.emit(
                    crate::audit::AuditEvent::new(crate::audit::AuditKind::Kick)
                        .room(room)
                        .actor(claims.sub)
                        .target(peer)
                        .reason(reason.unwrap_or_default()),
                );
            }
            (StatusCode::NO_CONTENT, "").into_response()
        }
        _ => (StatusCode::NOT_FOUND, "peer not in room").into_response(),
    }
}

async fn peers_force_release(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Path((room, peer)): Path<(String, String)>,
    body: Option<Json<ActionReq>>,
) -> Response {
    let claims = match guard_claims(&app, &headers) {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    if !crate::rooms::valid_room(&room) || !valid_id(&peer) {
        return (StatusCode::NOT_FOUND, "unknown room or peer").into_response();
    }
    let reason = action_reason(&body);
    if reason_overlong(&reason) {
        return (
            StatusCode::BAD_REQUEST,
            "reason too long (max 512, Discord clause)",
        )
            .into_response();
    }
    let tx = {
        let guard = app.rooms.lock().await;
        guard
            .get(&talkservo_core::ids::RoomId::from(room.as_str()))
            .cloned()
    };
    let Some(tx) = tx else {
        return (StatusCode::NOT_FOUND, "room is not live").into_response();
    };
    let (rsp, rcp) = tokio::sync::oneshot::channel::<bool>();
    if tx
        .send(crate::ws::RoomCommand::ForceRelease {
            peer: talkservo_core::ids::PeerId::from(peer.as_str()),
            reply: rsp,
        })
        .await
        .is_err()
    {
        return (StatusCode::NOT_FOUND, "room is not live").into_response();
    }
    match rcp.await {
        Ok(true) => {
            if let Some(a) = &app.audit {
                a.emit(
                    crate::audit::AuditEvent::new(crate::audit::AuditKind::ForceRelease)
                        .room(room)
                        .actor(claims.sub)
                        .target(peer)
                        .reason(reason.unwrap_or_default()),
                );
            }
            (StatusCode::NO_CONTENT, "").into_response()
        }
        _ => (StatusCode::NOT_FOUND, "peer not in room").into_response(),
    }
}

/// Registry views (plan-7 S2). GET lists rooms.yaml rows; POST upserts one;
/// DELETE removes the row AND evicts a live room via the disconnect teardown.
/// R1 rename: the live floor poll moved to /admin/api/rooms/live.
async fn rooms_registry(State(app): State<Arc<App>>, headers: HeaderMap) -> Response {
    if let Err(e) = guard(&app, &headers) {
        return e.into_response();
    }
    Json(crate::rooms::list_view(&app.config)).into_response()
}

#[derive(Deserialize)]
struct RoomReq {
    room: String,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    mode: Option<talkservo_core::wire::FloorMode>,
    #[serde(default)]
    max_peers: Option<usize>,
    #[serde(default)]
    max_queue: Option<usize>,
    #[serde(default)]
    locked: bool,
}

async fn rooms_create(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    body: Result<Json<RoomReq>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Err(e) = require_admin(&app, &headers) {
        return e.into_response();
    }
    let Json(req) = match body {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "malformed body").into_response(),
    };
    let row = crate::rooms::RoomRow {
        room: req.room,
        owner: req.owner,
        mode: req.mode,
        max_peers: req.max_peers,
        max_queue: req.max_queue,
        locked: req.locked,
    };
    match crate::rooms::upsert(&app.config, row) {
        Ok(_) => (StatusCode::CREATED, "").into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

async fn rooms_delete(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Path(room): Path<String>,
) -> Response {
    if let Err(e) = require_admin(&app, &headers) {
        return e.into_response();
    }
    if !crate::rooms::valid_room(&room) {
        return (StatusCode::NOT_FOUND, "unknown room").into_response();
    }
    match crate::rooms::remove(&app.config, &room) {
        Ok(false) => (StatusCode::NOT_FOUND, "room not in registry").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        Ok(true) => {
            // live eviction: fire-and-forget; a dead/absent channel just means
            // nobody was inside (the registry row is the authority).
            let rid = talkservo_core::ids::RoomId::from(room.as_str());
            let tx = {
                let guard = app.rooms.lock().await;
                guard.get(&rid).cloned()
            };
            if let Some(tx) = tx {
                let _ = tx.send(crate::ws::RoomCommand::Evict).await;
            }
            (StatusCode::NO_CONTENT, "").into_response()
        }
    }
}

// ── plan-9 W1: the admin push channel ──────────────────────────────────────

/// A separate endpoint on purpose: an admin token cannot pass `/ws` (no `room`
/// claim, see auth.rs) and must not be able to, so the admin plane gets its own
/// contract, its own version counter and its own connect window. The signaling
/// wire stays at 27 variants and plan-7 §4 is untouched.
async fn admin_ws(State(app): State<Arc<App>>, ws: WebSocketUpgrade) -> Response {
    if app.config.admin_secret.is_none() {
        return (StatusCode::SERVICE_UNAVAILABLE, "admin surface disabled").into_response();
    }
    if !ws_rate_ok(&app) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "admin ws connect budget exceeded",
        )
            .into_response();
    }
    let hub = app.admin_events.clone();
    ws.on_upgrade(move |socket| run_admin_socket(socket, app, hub))
}

fn ws_rate_ok(app: &Arc<App>) -> bool {
    // its own window on its own constant (R5=A): reconnect backoff from a few tabs
    // is normal traffic and must not consume the login/token budget
    let mut w = app.admin_ws_rate.lock().expect("ws rate window");
    if w.0.elapsed() > std::time::Duration::from_secs(60) {
        w.0 = std::time::Instant::now();
        w.1 = 0;
    }
    w.1 += 1;
    w.1 <= ADMIN_WS_CONNECTS_PER_MIN
}

async fn run_admin_socket(mut socket: WebSocket, app: Arc<App>, hub: Arc<EventHub>) {
    // Credential first, and only first. A browser cannot attach a header to a
    // WebSocket, and the token must not ride the URL either (access logs would keep
    // a copy of a credential), so the first frame is the auth surface.
    let token = match first_frame(&mut socket).await {
        Some(AdminIn::Auth { token }) => token,
        _ => {
            let _ = socket
                .send(Message::Close(Some(CloseFrame {
                    code: 1008,
                    reason: "expected an auth frame".into(),
                })))
                .await;
            return;
        }
    };
    let Some(secret) = app.config.admin_secret.as_deref() else {
        let _ = socket.send(Message::Close(None)).await;
        return;
    };
    if auth::validate_admin(secret, &token).is_err() {
        // a close code survives the browser API; a JSON body after a refused
        // upgrade does not, which is why §3.2's 401 note became 1008 here
        let _ = socket
            .send(Message::Close(Some(CloseFrame {
                code: 1008,
                reason: "unauthorized".into(),
            })))
            .await;
        return;
    }
    let Some(hello) = json_of(&hub.hello()) else {
        return;
    };
    if socket.send(Message::Text(hello.into())).await.is_err() {
        return;
    }

    let mut rx = hub.subscribe();
    loop {
        tokio::select! {
            ev = rx.recv() => match ev {
                Ok(ev) => {
                    let Some(text) = json_of(&ev) else { continue };
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // this tab fell behind the hub bound: hang up (it degrades to
                    // its reconcile poll) and count the eviction so the next Hello
                    // reports it instead of it being silent
                    hub.note_eviction();
                    break;
                }
                Err(_) => break,
            },
            msg = socket.recv() => match msg {
                None => break,
                Some(Err(_)) => break,
                Some(Ok(Message::Text(t))) => {
                    if matches!(serde_json::from_str::<AdminIn>(&t), Ok(AdminIn::Ping)) {
                        let Some(pong) = json_of(&hub.pong()) else { continue };
                        if socket.send(Message::Text(pong.into())).await.is_err() {
                            break;
                        }
                    }
                    // Subscribe and any later Auth are inert: both streams are
                    // always on and the credential is a one-shot
                }
                Some(Ok(Message::Close(_))) => break,
                Some(Ok(_)) => {}
            }
        }
    }
}

async fn first_frame(socket: &mut WebSocket) -> Option<AdminIn> {
    let raw = tokio::time::timeout(AUTH_DEADLINE, socket.recv())
        .await
        .ok()??;
    let Message::Text(t) = raw.ok()? else {
        return None;
    };
    if t.len() > AUTH_FRAME_MAX_BYTES {
        return None;
    }
    serde_json::from_str(&t).ok()
}

fn json_of<T: serde::Serialize>(value: &T) -> Option<String> {
    // serializing our own enums cannot realistically fail; returning Option keeps
    // that a control-flow branch instead of an expect() on a socket path
    serde_json::to_string(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // D29-#4 (entry-flow wave): generated subjects are `u-` + 6 base36 chars,
    // collision-checked against live seats + registry rows before issuance.
    #[test]
    fn gen_subject_format_is_u_plus_six_base36() {
        let s = gen_subject(&std::collections::HashSet::new());
        assert!(s.starts_with("u-"), "prefix: {s}");
        let rest = &s[2..];
        assert_eq!(rest.len(), 6, "len: {s}");
        assert!(
            rest.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
            "charset: {s}"
        );
    }

    #[test]
    fn gen_subject_avoids_registered_names() {
        // seed every possible 6-char space? no — seed a few names and prove the
        // generator never returns one (loop gives it 200 chances to collide).
        let occupied: std::collections::HashSet<String> = ["u-aaa000", "u-zzz999"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        for _ in 0..200 {
            let s = gen_subject(&occupied);
            assert!(!occupied.contains(&s), "collision: {s}");
        }
    }

    #[test]
    fn gen_subject_unique_across_calls() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..50 {
            seen.insert(gen_subject(&seen));
        }
        assert_eq!(seen.len(), 50);
    }

    #[test]
    fn token_req_missing_sub_deserializes_to_empty() {
        // the D29 wire shape is sub-ABSENT (entryActions omits it); serde must accept
        // that and land it in the empty-string branch, not reject the whole body
        // (regression: the 400 "malformed body" integration gap, 2026-10-28)
        let req: TokenReq = serde_json::from_str(r#"{"room":"tst","role":"dispatch"}"#)
            .expect("missing sub must deserialize");
        assert_eq!(req.sub, "");
        assert_eq!(req.room, "tst");
        // explicit empty string takes the same branch
        let req2: TokenReq = serde_json::from_str(r#"{"sub":"","room":"tst","role":"field"}"#)
            .expect("empty sub must deserialize");
        assert_eq!(req2.sub, "");
    }

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
