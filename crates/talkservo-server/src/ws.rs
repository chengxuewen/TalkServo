//! WS entry: axum route, Join sequence (modules/02 §J), frame limits.

use crate::auth;
use crate::config::Config;
use crate::obs;
use crate::room::{RoomState, Sink};
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use talkservo_core::ids::{PeerId, RoomId};
use talkservo_core::wire::SignalingMessage;
use talkservo_sfu::Sfu as _;
use tokio::sync::mpsc;

/// Shared app state.
pub struct App {
    pub config: Config,
    pub rooms: tokio::sync::Mutex<HashMap<RoomId, mpsc::Sender<RoomCommand>>>,
    /// uptime source + token rate-limit window anchor (plan-5 S1,
    /// adjudicated #4A: 60 issues/min, 429 beyond)
    pub started_at: std::time::Instant,
    pub admin_rate: std::sync::Mutex<(std::time::Instant, u32)>,
    /// audit writer channel (plan-7 S1; None = writer not spawned, sites no-op)
    pub audit: Option<crate::audit::AuditTx>,
    /// admin-plane fan-out hub (plan-9 W1). Always present: whether a socket may
    /// subscribe is decided by `admin_secret` + auth, not by the hub. Hints only
    /// flow when the audit writer exists, because the audit emit is the one
    /// producer (no audit_log => no push, and the console keeps reconciling).
    pub admin_events: std::sync::Arc<crate::admin_events::EventHub>,
    /// own connect window for /admin/ws, deliberately NOT shared with
    /// `admin_rate` (plan-9 R5=A): a reconnect storm must not be able to lock the
    /// operator out of login and token issuance.
    pub admin_ws_rate: std::sync::Mutex<(std::time::Instant, u32)>,
    /// Concrete SFU backend (exactly one; feature-selected, modules/03).
    #[cfg(feature = "stub-media")]
    pub media: std::sync::Arc<talkservo_sfu::StubSfu>,
    #[cfg(not(feature = "stub-media"))]
    pub media: std::sync::Arc<talkservo_sfu::MediasoupSfu>,
}

pub enum RoomCommand {
    /// A validated connection wants in: (peer, role, mod claims, inbound msg
    /// rx, outbound tx sink)
    Join {
        peer: PeerId,
        role: talkservo_core::wire::Role,
        mod_rooms: Vec<String>,
        sink: Sink,
        reply: tokio::sync::oneshot::Sender<Result<mpsc::Receiver<RoomCommand>, String>>,
    },
    /// Floor-protocol message from a live member.
    Wire { from: PeerId, msg: SignalingMessage },
    /// Connection closed.
    Left { peer: PeerId },
    /// E11 watchdog tick: probe granted peers' activity through the backend.
    WatchdogTick,
    /// Max-hold elapsed: the room decides whether a holder exceeds the cap.
    MaxHoldTick,
    /// TokenRefresh sweep tick (re-signs tokens near expiry).
    RefreshTick,
    /// Room-idle sweep tick (D16/CM-1: empty room past TTL → reap).
    IdleTick,
    /// Registry delete fan-out (plan-7 S2): evict every member via the
    /// disconnect teardown and stop the room task.
    Evict,
    /// S6 kick one peer (admin API); reply found(true)/absent(false). Teardown
    /// equals the Evict iteration for ONE peer; delta is kind=removed.
    KickPeer {
        peer: PeerId,
        reply: tokio::sync::oneshot::Sender<bool>,
    },
    /// S6 forced floor release (admin API): server-injects the peer's own
    /// FloorRelease through the authorized path (R8-safe no-op if ungranted);
    /// reply reflects membership, not grant state.
    ForceRelease {
        peer: PeerId,
        reply: tokio::sync::oneshot::Sender<bool>,
    },
    /// Read-only admin inspect (plan-5 S1): answer from RoomState ONLY —
    /// zero SFU calls, zero mutation; the fan-out timeout lives in admin.rs.
    AdminInspect(tokio::sync::oneshot::Sender<RoomReport>),
}

/// Internal room snapshot for /admin/api (plan-5 S1). The `speaking` list
/// reads the watchdog activity cache: per-holder truth of "last probe saw
/// RTP" — granted-but-silent peers are what an operator must see (E11
/// precursors).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RoomReport {
    pub room: String,
    pub mode: talkservo_core::wire::FloorMode,
    pub holders: Vec<String>,
    pub queue_len: usize,
    pub peers: Vec<PeerEntry>,
    pub muted: Vec<String>,
    pub speaking: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PeerEntry {
    pub id: String,
    pub connected_since_ms: u64,
    /// S6.2 detail-drawer fields (all RoomState-owned; the zero-SFU-call
    /// inspect principle holds). bitrate/ICE stats deferred (wave-2).
    pub role: talkservo_core::wire::Role,
    pub mod_rooms: Vec<String>,
    pub granted: bool,
    pub muted: bool,
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(app): State<Arc<App>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, app))
}

async fn handle_socket(socket: WebSocket, app: Arc<App>) {
    let (mut tx, mut rx) = socket.split();
    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<SignalingMessage>();

    // ── Join gate: the FIRST message must be Join{v, jwt} ──────────────
    let first = match tokio::time::timeout(std::time::Duration::from_secs(10), rx.next()).await {
        Ok(Some(Ok(m))) => m,
        _ => return, // timeout or socket error
    };

    let raw = match first {
        Message::Text(t) => t,
        _ => return,
    };
    if raw.len() > app.config.max_frame_bytes {
        let _ = tx
            .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                code: 1009,
                reason: "frame too large".into(),
            })))
            .await;
        return;
    }
    let msg: SignalingMessage = match serde_json::from_str(&raw) {
        Ok(m) => m,
        Err(_) => {
            // undeserializable → bad_version + close 4400 (D16/CM-5)
            let err = SignalingMessage::Error {
                code: "bad_version".into(),
                detail: "undeserializable message or unsupported version".into(),
            };
            let _ = tx
                .send(Message::Text(
                    serde_json::to_string(&err).expect("ser").into(),
                ))
                .await;
            let _ = tx
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 4400,
                    reason: "bad_version".into(),
                })))
                .await;
            return;
        }
    };

    let (jwt, wire_v) = match &msg {
        SignalingMessage::Join { v, jwt } => (jwt.clone(), *v),
        _ => {
            let err = SignalingMessage::Error {
                code: "not_joined".into(),
                detail: "first message must be Join".into(),
            };
            let _ = tx
                .send(Message::Text(
                    serde_json::to_string(&err).expect("ser").into(),
                ))
                .await;
            return;
        }
    };
    if wire_v != 1 {
        let err = SignalingMessage::Error {
            code: "bad_version".into(),
            detail: format!("unsupported protocol version {wire_v}"),
        };
        let _ = tx
            .send(Message::Text(
                serde_json::to_string(&err).expect("ser").into(),
            ))
            .await;
        let _ = tx
            .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                code: 4400,
                reason: "bad_version".into(),
            })))
            .await;
        return;
    }

    // JWT must carry the room in aud — Join carries no room field, so the
    // token's own claim decides where the peer lands (modules/06 room-scoped aud).
    let claims = match extract_room_and_validate(&app.config, &jwt) {
        Ok(c) => c,
        Err(e) => {
            let code = match e {
                auth::AuthError::Expired => "expired_token",
                _ => "invalid_token",
            };
            let err = SignalingMessage::Error {
                code: code.to_string(),
                detail: e.to_string(),
            };
            let _ = tx
                .send(Message::Text(
                    serde_json::to_string(&err).expect("ser").into(),
                ))
                .await;
            let _ = tx
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 4401,
                    reason: code.into(),
                })))
                .await;
            return;
        }
    };

    let peer = PeerId::from(claims.sub.clone());
    let room_id = RoomId::from(claims.room.clone());

    // S2: registry lock gate - hot-read, no new wire variant (Error.code).
    if crate::rooms::is_locked(&app.config, &room_id) {
        let err = SignalingMessage::Error {
            code: "room_locked".into(),
            detail: "room is locked by its operator".into(),
        };
        let _ = tx
            .send(Message::Text(
                serde_json::to_string(&err).expect("ser").into(),
            ))
            .await;
        return;
    }

    // Get/create the room's writer channel.
    let room_tx = {
        let mut rooms = app.rooms.lock().await;
        rooms
            .entry(room_id.clone())
            .or_insert_with(|| spawn_room(room_id.clone(), app.clone()))
            .clone()
    };

    // Ask the room to register us (AlreadyJoined check inside).
    let (reply_tx, mut reply_rx) = tokio::sync::oneshot::channel();
    // R-F4: the cloned sender can be dead if the room self-reaped between
    // our registry lookup and this send — retry ONCE through a fresh lookup
    // (the reaper removed the stale entry, so the retry spawns a live room).
    let join_cmd = RoomCommand::Join {
        peer: peer.clone(),
        role: claims.role,
        mod_rooms: claims.mod_rooms.clone(),
        sink: outbound_tx.clone(),
        reply: reply_tx,
    };
    let room_tx = if room_tx.send(join_cmd).await.is_err() {
        if crate::rooms::is_locked(&app.config, &room_id) {
            // config flipped locked between the attempts - same wire answer
            let err = SignalingMessage::Error {
                code: "room_locked".into(),
                detail: "room is locked by its operator".into(),
            };
            let _ = tx
                .send(Message::Text(
                    serde_json::to_string(&err).expect("ser").into(),
                ))
                .await;
            return;
        }
        let mut rooms = app.rooms.lock().await;
        let room_tx = rooms
            .entry(room_id.clone())
            .or_insert_with(|| spawn_room(room_id.clone(), app.clone()))
            .clone();
        drop(rooms);
        // fresh reply channel — the old oneshot's other end died with the task
        let (reply_tx2, reply_rx2) = tokio::sync::oneshot::channel();
        reply_rx = reply_rx2;
        room_tx
            .send(RoomCommand::Join {
                peer: peer.clone(),
                role: claims.role,
                mod_rooms: claims.mod_rooms.clone(),
                sink: outbound_tx.clone(),
                reply: reply_tx2,
            })
            .await
            .expect("freshly spawned room is alive");
        room_tx
    } else {
        room_tx
    };
    let _ = &room_tx;

    let cmd_rx = match reply_rx.await {
        Ok(Ok(rx)) => rx,
        Ok(Err(code)) => {
            let err = SignalingMessage::Error {
                code,
                detail: "identity already connected".into(),
            };
            let _ = tx
                .send(Message::Text(
                    serde_json::to_string(&err).expect("ser").into(),
                ))
                .await;
            return;
        }
        Err(_) => return,
    };

    // Welcome/RouterCaps/PeerList/Snapshot are all sent by the room task in
    // one deterministic order (single writer owns join sequencing).

    // ── pump loop: socket → room, room → socket ────────────────────────
    let mut cmd_rx = cmd_rx;
    loop {
        tokio::select! {
            inbound = rx.next() => {
                match inbound {
                    Some(Ok(Message::Text(t))) => {
                        if t.len() > app.config.max_frame_bytes {
                            let _ = tx.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                                code: 1009, reason: "frame too large".into(),
                            }))).await;
                            break;
                        }
                        match serde_json::from_str::<SignalingMessage>(&t) {
                            Ok(m) => {
                                let _ = room_tx
                                    .send(RoomCommand::Wire { from: peer.clone(), msg: m })
                                    .await;
                            }
                            Err(_) => {
                                let err = SignalingMessage::Error {
                                    code: "bad_version".into(),
                                    detail: "undeserializable message".into(),
                                };
                                let _ = tx.send(Message::Text(serde_json::to_string(&err).expect("ser").into())).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        let _ = room_tx.send(RoomCommand::Left { peer: peer.clone() }).await;
                        break;
                    }
                    _ => {}
                }
            }
            Some(out_msg) = outbound_rx.recv() => {
                let text: axum::extract::ws::Utf8Bytes = serde_json::to_string(&out_msg).expect("ser").into();
                if tx.send(Message::Text(text)).await.is_err() {
                    let _ = room_tx.send(RoomCommand::Left { peer: peer.clone() }).await;
                    break;
                }
            }
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    RoomCommand::Wire { msg, .. } => {
                        let text: axum::extract::ws::Utf8Bytes = serde_json::to_string(&msg).expect("ser").into();
                        let _ = tx.send(Message::Text(text)).await;
                    }
                    RoomCommand::Left { .. } => {
                        let _ = tx.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                            code: 1000, reason: "server closing".into(),
                        }))).await;
                        break;
                    }
                    RoomCommand::Join { .. } | RoomCommand::WatchdogTick
                    | RoomCommand::MaxHoldTick | RoomCommand::RefreshTick
                    | RoomCommand::IdleTick | RoomCommand::Evict
                    | RoomCommand::KickPeer { .. } | RoomCommand::ForceRelease { .. } => {}
                    // pre-admission inspect: dropping the oneshot surfaces
                    // as `stale` on the admin side — correct semantics
                    RoomCommand::AdminInspect(_) => {}
                }
            }
        }
    }
    let _ = &mut tx;
}

/// Validate JWT; returns claims (room comes from the aud claim).
fn extract_room_and_validate(config: &Config, jwt: &str) -> Result<auth::Claims, auth::AuthError> {
    // Two-step: decode WITHOUT aud check to read the room, then re-validate
    // with the right audience (JWT carries room in `room` claim; aud mirrors).
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_aud = false;
    let data = jsonwebtoken::decode::<auth::Claims>(
        jwt,
        &jsonwebtoken::DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => auth::AuthError::Expired,
        _ => auth::AuthError::Invalid(e.to_string()),
    })?;
    let room = data.claims.room.clone();
    auth::validate(&config.jwt_secret, jwt, &room)
}

/// Spawn the single-writer task for a room.
fn spawn_room(room_id: RoomId, app: Arc<App>) -> mpsc::Sender<RoomCommand> {
    let (tx, mut rx) = mpsc::channel::<RoomCommand>(256);

    // timer loops (T4): watchdog, max-hold, token-refresh — all funnel into
    // the same channel so the single writer keeps total ordering
    {
        let tx = tx.clone();
        let cfg = app.config.clone();
        tokio::spawn(async move {
            let mut watchdog =
                tokio::time::interval(std::time::Duration::from_millis(cfg.no_rtp_watchdog_ms));
            let mut maxhold = tokio::time::interval(std::time::Duration::from_millis(
                cfg.floor_max_hold_ms.unwrap_or(u64::MAX),
            ));
            let mut refresh = tokio::time::interval(std::time::Duration::from_secs(
                cfg.jwt_ttl_s.saturating_sub(300).max(1),
            ));
            // tokio intervals fire the FIRST tick immediately — skip it so the
            // room doesn't get timer commands before any member exists
            // idle sweep: fine-grained enough to respect TTL within 30s
            let mut idle =
                tokio::time::interval(std::time::Duration::from_secs(cfg.room_idle_ttl_s.min(30)));
            watchdog.tick().await;
            maxhold.tick().await;
            refresh.tick().await;
            idle.tick().await;
            loop {
                tokio::select! {
                    _ = watchdog.tick() => { if tx.send(RoomCommand::WatchdogTick).await.is_err() { break; } }
                    _ = maxhold.tick() => { if tx.send(RoomCommand::MaxHoldTick).await.is_err() { break; } }
                    _ = refresh.tick() => { if tx.send(RoomCommand::RefreshTick).await.is_err() { break; } }
                    _ = idle.tick() => { if tx.send(RoomCommand::IdleTick).await.is_err() { break; } }
                }
            }
        });
    }

    let tx_probe = tx.clone(); // channel-identity guard for the reaper
    tokio::spawn(async move {
        let spec = crate::rooms::resolve(&app.config, &room_id);
        let mut state = RoomState::new(room_id.clone(), spec.mode, spec.max_peers, spec.max_queue);
        state.set_audit(app.audit.clone());
        let media = app.media.clone();
        let mut media_ctx = crate::media::MediaCtx::default();
        let cfg_media_grace_ms = app.config.floor_media_grace_ms;
        let cfg_room_idle_ttl_s = app.config.room_idle_ttl_s;
        let refresh_secret = app.config.jwt_secret.clone();
        let refresh_ttl_s = app.config.jwt_ttl_s;
        let mut watch = crate::timers::WatchState::default();
        while let Some(cmd) = rx.recv().await {
            match cmd {
                RoomCommand::Join {
                    peer,
                    role,
                    mod_rooms,
                    sink,
                    reply,
                } => {
                    let since_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    let conn_sink = sink.clone();
                    // actor->connection control channel: created BEFORE join
                    // (Member stores the tx; rx travels back via reply)
                    let (wire_tx, wire_rx) = mpsc::channel::<RoomCommand>(64);
                    let res = state.join(peer.clone(), role, sink, wire_tx, mod_rooms, since_ms);
                    match res {
                        Ok(()) => {
                            state.relay_join(&peer);
                            let generation = state.floor.generation();
                            obs::event(
                                state.id.0.as_ref(),
                                peer.0.as_ref(),
                                generation,
                                obs::Event::Join,
                                serde_json::json!({}),
                            );
                            // J burst, deterministic order (modules/02 §J):
                            // Welcome → RouterCaps → PeerJoined(delta) → PeerList → Snapshot
                            // TURN short-term creds (modules/06 rung 4);
                            // disabled → empty uris (dev runs without coturn)
                            let turn_creds = crate::turn::issue(
                                app.config.turn_secret.as_deref().unwrap_or(""),
                                app.config.turn_ttl_s,
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0),
                                &app.config.turn_uri,
                            )
                            .unwrap_or_else(|| serde_json::json!({"uris": []}));
                            let _ = conn_sink.send(SignalingMessage::Welcome {
                                v: 1,
                                peer_id: peer.clone(),
                                turn_creds,
                            });
                            let caps = media.router_caps(&state.id).await;
                            let _ =
                                conn_sink.send(SignalingMessage::RouterCaps { media_codecs: caps });
                            let delta = SignalingMessage::PeerJoined {
                                info: state.members[&peer].info.clone(),
                                generation,
                            };
                            state.broadcast_except(&delta, Some(&peer));
                            let peers: Vec<talkservo_core::wire::PeerInfo> =
                                state.members.values().map(|m| m.info.clone()).collect();
                            let _ =
                                conn_sink.send(SignalingMessage::PeerList { peers, generation });
                            let snap = state.snapshot_for(&peer);
                            let _ = conn_sink.send(snap);
                            // late-joiner catch-up: announce every EXISTING
                            // producer so the newcomer can consume (measured:
                            // announcements broadcast at produce time miss
                            // peers who join afterwards)
                            for (owner, producer_id) in state.producers_list() {
                                let _ = conn_sink.send(SignalingMessage::ProducerAvailable {
                                    peer: owner.clone(),
                                    producer_id: serde_json::json!(producer_id),
                                });
                            }
                            let _ = reply.send(Ok(wire_rx));
                        }
                        Err(code) => {
                            let _ = reply.send(Err(code.to_string()));
                        }
                    }
                }
                RoomCommand::Wire { from, msg } => {
                    if msg_is_resync(&msg) {
                        let snap = state.snapshot_for(&from);
                        if let Some(m) = state.members.get(&from) {
                            let _ = m.sink.send(snap);
                        }
                        continue;
                    }
                    // media-plane messages route through the sfu host
                    if matches!(
                        msg,
                        SignalingMessage::TransportCreate { .. }
                            | SignalingMessage::TransportConnect { .. }
                            | SignalingMessage::Produce { .. }
                            | SignalingMessage::Consume { .. }
                    ) {
                        // R-F14: consume gate — requester must be a member and
                        // the target producer must belong to a GRANTED peer
                        // (floor confidentiality; guessed ids get nothing)
                        if let SignalingMessage::Consume { producer_id } = &msg {
                            // JSON string value — as_str(), not to_string()
                            // (which embeds the quotes; gate would be a
                            // permanent deny — measured on live)
                            let pid = producer_id.as_str().unwrap_or_default();
                            let allowed = state.producer_owner_granted(pid);
                            if !allowed {
                                if let Some(m) = state.members.get(&from) {
                                    let _ = m.sink.send(SignalingMessage::Error {
                                        code: "consume_denied".into(),
                                        detail: "producer not available for consumption".into(),
                                    });
                                }
                                continue;
                            }
                        }
                        // modules/05 media-plane events (transport_create/
                        // produce_ok/transport_fail) with full room/peer/gen
                        let media_event = match &msg {
                            SignalingMessage::TransportCreate { .. } => {
                                Some(obs::Event::TransportCreate)
                            }
                            SignalingMessage::Produce { .. } => Some(obs::Event::ProduceOk),
                            _ => None,
                        };
                        let replies = crate::media::handle_media_message(
                            media.as_ref(),
                            &state.id,
                            &from,
                            &msg,
                            &mut media_ctx,
                            app.config.transport_guardrail,
                        )
                        .await;
                        if let Some(ev) = media_event {
                            obs::event(
                                state.id.0.as_ref(),
                                from.0.as_ref(),
                                state.floor.generation(),
                                ev,
                                serde_json::json!({}),
                            );
                        }
                        // direct replies go to the producer FIRST (ordering
                        // pin: owner learns its id before the room does)
                        if let Some(m) = state.members.get(&from) {
                            for d in &replies {
                                let _ = m.sink.send(d.clone());
                            }
                        }
                        // R-F14 registry + pull-model availability announce
                        // (additive wire): room consumes against this id
                        if let Some(SignalingMessage::ProduceOk { producer_id }) = replies.first() {
                            // JSON string value — as_str() (to_string embeds
                            // the quotes; registry then never matches the
                            // bare id in consume requests — permanent deny)
                            let pid_bare = producer_id.as_str().unwrap_or_default().to_string();
                            state.note_producer(&from, pid_bare);
                            state.broadcast(&SignalingMessage::ProducerAvailable {
                                peer: from.clone(),
                                producer_id: producer_id.clone(),
                            });
                        }
                        continue;
                    }
                    let floor_mutated = state.handle_floor_message(&from, &msg);
                    if let Some(m) = state.members.get(&from) {
                        for d in floor_mutated.direct {
                            let _ = m.sink.send(d);
                        }
                    }
                    // D13 gating: media reconciles with the NEW state after
                    // every floor transition (R-F15 — was missing entirely)
                    if floor_mutated.mutated {
                        media.apply_floor(&state.id, state.floor_ref()).await;
                    }
                }
                RoomCommand::IdleTick => {
                    // D16/CM-1: empty room past ROOM_IDLE_TTL_S → reap.
                    state.track_empty(); // arm on first empty sight, disarm on join
                    if !state.is_empty() {
                        continue;
                    }
                    let empty_since = match state.empty_since() {
                        Some(t) => t,
                        None => continue, // empty but clock not armed yet
                    };
                    let ttl = std::time::Duration::from_secs(cfg_room_idle_ttl_s);
                    if empty_since.elapsed() >= ttl {
                        obs::event(
                            state.id.0.as_ref(),
                            "-",
                            state.floor.generation(),
                            obs::Event::Leave, // closed vocab; reason field disambiguates
                            serde_json::json!({"reason": "room_idle_ttl_reaped"}),
                        );
                        media.destroy_room(&state.id).await;
                        // registry removal guarded by channel identity — never
                        // delete a successor room spawned after our death
                        let mut rooms = app.rooms.lock().await;
                        if rooms
                            .get(&state.id)
                            .is_some_and(|existing| existing.same_channel(&tx_probe))
                        {
                            rooms.remove(&state.id);
                        }
                        drop(rooms);
                        drop(tx_probe);
                        return; // task ends (tx stays owned by the closure —
                        // its drop closes the channel for the registry)
                    }
                }
                RoomCommand::AdminInspect(tx) => {
                    let _ = tx.send(state.admin_report());
                }
                RoomCommand::WatchdogTick => {
                    // probe granted peers through the backend; silent-past-grace
                    // peers get MediaDown (core releases + promotes, F-sequence)
                    let granted: Vec<PeerId> = state.floor.grants().to_vec();
                    if granted.is_empty() {
                        continue;
                    }
                    let mut results = Vec::with_capacity(granted.len());
                    for p in granted {
                        let active = matches!(
                            media.media_activity(&state.id, &p).await,
                            talkservo_sfu::ActivityState::Active
                        );
                        // admin `speaking` bit: cache the probe result (S1)
                        state.note_activity(&p, active);
                        results.push((p, active));
                    }
                    let grace = std::time::Duration::from_millis(cfg_media_grace_ms);
                    let downs = watch.probe(results, grace, std::time::Instant::now());
                    for peer in downs {
                        for m in state.apply_media_down(&peer) {
                            state.broadcast(&m);
                        }
                        state.broadcast(&SignalingMessage::MediaFailed { peer: peer.clone() });
                        media.apply_floor(&state.id, state.floor_ref()).await;
                    }
                }
                RoomCommand::MaxHoldTick => {
                    // dispatch-mode cap fires Timeout like an operator timer;
                    // core applies the same Timeout rule (R7). The tick interval
                    // is only armed when FLOOR_MAX_HOLD_MS is set (config gate).
                    for m in state.apply_timeout() {
                        state.broadcast(&m);
                    }
                    media.apply_floor(&state.id, state.floor_ref()).await;
                }
                RoomCommand::RefreshTick => {
                    // TokenRefresh is per-connection in the full design; the PoC
                    // re-signs a fresh token per member and pushes it directly.
                    for m in state.members.values() {
                        if let Ok(jwt) = crate::auth::issue(
                            &refresh_secret,
                            m.info.id.0.as_ref(),
                            state.id.0.as_ref(),
                            m.info.role,
                            refresh_ttl_s,
                        ) {
                            let _ = m.sink.send(SignalingMessage::TokenRefresh { jwt });
                        }
                    }
                }
                RoomCommand::Evict => {
                    // S2: registry delete kicks all - same per-peer teardown
                    // the disconnect path runs (media free, leave relay,
                    // delta broadcast; sockets close as their sinks drop).
                    let peers: Vec<PeerId> = state.members.keys().cloned().collect();
                    for p in peers {
                        // signal FIRST (connection loop answers with a 1000
                        // Close frame), then run the room-side teardown the
                        // disconnect path runs - order avoids the connection's
                        // own Left-on-close racing a half-evicted member.
                        if let Some(m) = state.members.get(&p) {
                            let _ = m.wire_tx.try_send(RoomCommand::Left { peer: p.clone() });
                        }
                        media.peer_left(&state.id, &p).await;
                        if let Some(delta) = state.leave_removed(&p) {
                            state.relay_leave(&p);
                            obs::event(
                                state.id.0.as_ref(),
                                p.0.as_ref(),
                                state.floor.generation(),
                                obs::Event::Leave,
                                serde_json::json!({}),
                            );
                            state.broadcast(&delta);
                        }
                    }
                    break; // task ends; reaper drops the dead registry slot
                }
                RoomCommand::KickPeer { peer, reply } => {
                    let found = state.members.contains_key(&peer);
                    if found {
                        if let Some(m) = state.members.get(&peer) {
                            let _ = m.wire_tx.try_send(RoomCommand::Left { peer: peer.clone() });
                        }
                        media.peer_left(&state.id, &peer).await;
                        if let Some(delta) = state.leave_removed(&peer) {
                            state.relay_leave(&peer);
                            obs::event(
                                state.id.0.as_ref(),
                                peer.0.as_ref(),
                                state.floor.generation(),
                                obs::Event::Leave,
                                serde_json::json!({}),
                            );
                            state.broadcast(&delta);
                        }
                    }
                    let _ = reply.send(found);
                }
                RoomCommand::ForceRelease { peer, reply } => {
                    let found = state.members.contains_key(&peer);
                    if found {
                        // route through the peer's own authorized release path:
                        // grants shrink via core, PIT-21 rebroadcast fires
                        state.handle_floor_message(&peer, &SignalingMessage::FloorRelease);
                    }
                    let _ = reply.send(found);
                }
                RoomCommand::Left { peer } => {
                    media.peer_left(&state.id, &peer).await; // R-F16: free media objects NOW
                    if let Some(delta) = state.leave(&peer) {
                        state.relay_leave(&peer);
                        obs::event(
                            state.id.0.as_ref(),
                            peer.0.as_ref(),
                            state.floor.generation(),
                            obs::Event::Leave,
                            serde_json::json!({}),
                        );
                        state.broadcast(&delta);
                    }
                }
            }
        }
    });
    tx
}

fn msg_is_resync(m: &SignalingMessage) -> bool {
    matches!(m, SignalingMessage::Resync)
}
