//! Media orchestration (plan-2 T3): wire media messages → sfu host calls.
//!
//! Generic over the concrete `Sfu` backend selected at App construction
//! (exactly one active, modules/03). The stub records calls for tests; the
//! mediasoup host reports honest task-pointer errors until Task 5.

use std::collections::{HashMap, HashSet};
use talkservo_core::ids::{PeerId, RoomId};
use talkservo_core::wire::SignalingMessage;
use talkservo_sfu::{ProducerId, Sfu, SfuError};

/// Per-room media bookkeeping the router needs (transport guardrail, DTLS state).
#[derive(Default)]
pub struct MediaCtx {
    pub transport_counts: HashMap<RoomId, usize>,
    pub dtls_connected: HashSet<(String, String)>,
}


/// Route one media wire message through the sfu host.
/// Returns direct replies for the requester.
#[allow(clippy::too_many_arguments)]
pub async fn handle_media_message<M: Sfu + Sync + 'static>(
    media: &M,
    room: &RoomId,
    from: &PeerId,
    msg: &SignalingMessage,
    ctx: &mut MediaCtx,
    guardrail: usize,
) -> Vec<SignalingMessage> {
    match msg {
        SignalingMessage::TransportCreate => {
            let count = ctx.transport_counts.entry(room.clone()).or_insert(0);
            if *count >= guardrail {
                return vec![SignalingMessage::Error {
                    code: "media_capacity".into(),
                    detail: format!("transport guardrail {guardrail} reached"),
                }];
            }
            match media.create_transport(room, from).await {
                Ok(t) => {
                    *count += 1;
                    vec![SignalingMessage::TransportInfo {
                        ice: t.ice,
                        dtls: t.dtls,
                        addrs: t.addrs,
                    }]
                }
                Err(e) => media_error(e, from),
            }
        }
        SignalingMessage::TransportConnect { dtls } => {
            ctx.dtls_connected
                .insert((room.to_string(), from.to_string()));
            match media.connect_transport(room, from, dtls.clone()).await {
                Ok(()) => vec![],
                Err(e) => {
                    ctx.dtls_connected
                        .remove(&(room.to_string(), from.to_string()));
                    media_error(e, from)
                }
            }
        }
        SignalingMessage::Produce { rtp_parameters } => {
            if !ctx.dtls_connected.contains(&(room.to_string(), from.to_string())) {
                return vec![SignalingMessage::Error {
                    code: "bad_order".into(),
                    detail: "Produce before TransportConnect".into(),
                }];
            }
            match media.produce(room, from, rtp_parameters.clone()).await {
                Ok(pid) => vec![SignalingMessage::ProduceOk {
                    producer_id: serde_json::json!(pid.0),
                }],
                Err(e) => media_error(e, from),
            }
        }
        SignalingMessage::Consume { producer_id } => {
            // wire producer_id is a JSON string value — as_str(), not
            // to_string() (which would embed the quotes)
            let pid_str = producer_id.as_str().unwrap_or_default().to_string();
            match media
                .consume(room, from, &ProducerId(pid_str))
                .await
            {
                Ok((consumer_id, params)) => vec![SignalingMessage::ConsumeOk {
                    producer_id: producer_id.clone(),
                    consumer_id: serde_json::json!(consumer_id),
                    rtp_parameters: params,
                }],
                Err(e) => media_error(e, from),
            }
        }
        _ => vec![],
    }
}

fn media_error(e: SfuError, from: &PeerId) -> Vec<SignalingMessage> {
    match e {
        SfuError::TransportTimeout { .. } => {
            // E4: single attempt at PoC → kick media half (retry lands with
            // the live host in Task 5)
            vec![
                SignalingMessage::MediaFailed { peer: from.clone() },
                SignalingMessage::Error {
                    code: "transport_timeout".into(),
                    detail: "transport creation timed out".into(),
                },
            ]
        }
        SfuError::Capacity(m) => vec![SignalingMessage::Error {
            code: "media_capacity".into(),
            detail: m,
        }],
        other => vec![SignalingMessage::Error {
            code: "media_error".into(),
            detail: other.to_string(),
        }],
    }
}
