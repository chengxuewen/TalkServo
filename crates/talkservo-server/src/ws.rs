//! WS entry: axum route, Join sequence (modules/02 §J), frame limits.

use crate::auth;
use crate::config::Config;
use crate::obs;
use crate::room::{RoomState, Sink};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
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
    /// Concrete SFU backend (exactly one; feature-selected, modules/03).
    #[cfg(feature = "stub-media")]
    pub media: std::sync::Arc<talkservo_sfu::StubSfu>,
    #[cfg(not(feature = "stub-media"))]
    pub media: std::sync::Arc<talkservo_sfu::MediasoupSfu>,
}

pub enum RoomCommand {
    /// A validated connection wants in: (peer, role, inbound msg rx, outbound tx sink)
    Join {
        peer: PeerId,
        role: talkservo_core::wire::Role,
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
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app): State<Arc<App>>,
) -> impl IntoResponse {
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
                .send(Message::Text(serde_json::to_string(&err).expect("ser").into()))
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
                .send(Message::Text(serde_json::to_string(&err).expect("ser").into()))
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
            .send(Message::Text(serde_json::to_string(&err).expect("ser").into()))
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
                .send(Message::Text(serde_json::to_string(&err).expect("ser").into()))
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

    // Get/create the room's writer channel.
    let room_tx = {
        let mut rooms = app.rooms.lock().await;
        rooms
            .entry(room_id.clone())
            .or_insert_with(|| spawn_room(room_id.clone(), app.clone()))
            .clone()
    };

    // Ask the room to register us (AlreadyJoined check inside).
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    room_tx
        .send(RoomCommand::Join {
            peer: peer.clone(),
            role: claims.role,
            sink: outbound_tx.clone(),
            reply: reply_tx,
        })
        .await
        .expect("room alive");

    let cmd_rx = match reply_rx.await {
        Ok(Ok(rx)) => rx,
        Ok(Err(code)) => {
            let err = SignalingMessage::Error {
                code,
                detail: "identity already connected".into(),
            };
            let _ = tx
                .send(Message::Text(serde_json::to_string(&err).expect("ser").into()))
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
                    | RoomCommand::IdleTick => {}
                }
            }
        }
    }
    let _ = &mut tx;
}

/// Validate JWT; returns claims (room comes from the aud claim).
fn extract_room_and_validate(
    config: &Config,
    jwt: &str,
) -> Result<auth::Claims, auth::AuthError> {
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
            let mut watchdog = tokio::time::interval(std::time::Duration::from_millis(
                cfg.no_rtp_watchdog_ms,
            ));
            let mut maxhold = tokio::time::interval(std::time::Duration::from_millis(
                cfg.floor_max_hold_ms.unwrap_or(u64::MAX),
            ));
            let mut refresh = tokio::time::interval(std::time::Duration::from_secs(
                cfg.jwt_ttl_s.saturating_sub(300).max(1),
            ));
            // tokio intervals fire the FIRST tick immediately — skip it so the
            // room doesn't get timer commands before any member exists
            // idle sweep: fine-grained enough to respect TTL within 30s
            let mut idle = tokio::time::interval(std::time::Duration::from_secs(
                cfg.room_idle_ttl_s.min(30),
            ));
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
        let mut state = RoomState::new(
            room_id.clone(),
            app.config.max_peers,
            app.config.max_queue,
        );
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
                    sink,
                    reply,
                } => {
                    let since_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    let conn_sink = sink.clone();
                    let res = state.join(peer.clone(), role, sink, since_ms);
                    match res {
                        Ok(()) => {
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
                            let _ = conn_sink.send(SignalingMessage::Welcome {
                                v: 1,
                                peer_id: peer.clone(),
                                turn_creds: serde_json::json!({"uris": []}),
                            });
                            let _ = conn_sink.send(SignalingMessage::RouterCaps {
                                media_codecs: serde_json::json!([
                                    {"mimeType": "audio/opus", "channels": 1}
                                ]),
                            });
                            let delta = SignalingMessage::PeerJoined {
                                info: state.members[&peer].info.clone(),
                                generation,
                            };
                            state.broadcast_except(&delta, Some(&peer));
                            let peers: Vec<talkservo_core::wire::PeerInfo> = state
                                .members
                                .values()
                                .map(|m| m.info.clone())
                                .collect();
                            let _ = conn_sink.send(SignalingMessage::PeerList {
                                peers,
                                generation,
                            });
                            let snap = state.snapshot_for(&peer);
                            let _ = conn_sink.send(snap);
                            let (_wire_tx, wire_rx) = mpsc::channel(64);
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
                        SignalingMessage::TransportCreate
                            | SignalingMessage::TransportConnect { .. }
                            | SignalingMessage::Produce { .. }
                            | SignalingMessage::Consume { .. }
                    ) {
                        let replies = crate::media::handle_media_message(
                            media.as_ref(),
                            &state.id,
                            &from,
                            &msg,
                            &mut media_ctx,
                            app.config.transport_guardrail,
                        )
                        .await;
                        if let Some(m) = state.members.get(&from) {
                            for d in replies {
                                let _ = m.sink.send(d);
                            }
                        }
                        continue;
                    }
                    let direct = state.handle_floor_message(&from, &msg);
                    if let Some(m) = state.members.get(&from) {
                        for d in direct {
                            let _ = m.sink.send(d);
                        }
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
                        results.push((p, active));
                    }
                    let grace = std::time::Duration::from_millis(cfg_media_grace_ms);
                    let downs = watch.probe(results, grace, std::time::Instant::now());
                    for peer in downs {
                        for m in state.apply_media_down(&peer) {
                            state.broadcast(&m);
                        }
                        state.broadcast(&SignalingMessage::MediaFailed { peer: peer.clone() });
                    }
                }
                RoomCommand::MaxHoldTick => {
                    // dispatch-mode cap fires Timeout like an operator timer;
                    // core applies the same Timeout rule (R7). The tick interval
                    // is only armed when FLOOR_MAX_HOLD_MS is set (config gate).
                    for m in state.apply_timeout() {
                        state.broadcast(&m);
                    }
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
                            let _ = m
                                .sink
                                .send(SignalingMessage::TokenRefresh { jwt });
                        }
                    }
                }
                RoomCommand::Left { peer } => {
                    if let Some(delta) = state.leave(&peer) {
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
