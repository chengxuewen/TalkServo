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
use tokio::sync::mpsc;

/// Shared app state.
pub struct App {
    pub config: Config,
    pub rooms: tokio::sync::Mutex<HashMap<RoomId, mpsc::Sender<RoomCommand>>>,
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
                    RoomCommand::Join { .. } => {}
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
    tokio::spawn(async move {
        let mut state = RoomState::new(
            room_id.clone(),
            app.config.max_peers,
            app.config.max_queue,
        );
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
                    let direct = state.handle_floor_message(&from, &msg);
                    if let Some(m) = state.members.get(&from) {
                        for d in direct {
                            let _ = m.sink.send(d);
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
