//! Wire-contract roundtrip coverage: ≥1 case per SignalingMessage variant,
//! plus the wire-rule pins (snake_case tags, closed DenyReason, role-scoped
//! snapshots, D16 generation/version plumbing).

use talkservo_core::ids::{PeerId, RoomId};
use talkservo_core::wire::{
    DenyReason, DispatchSnapshot, FieldSnapshot, FloorMode, PeerInfo, Role,
    ServerSnapshotPayload, SignalingMessage,
};

fn pid(s: &str) -> PeerId {
    PeerId::from(s)
}

fn roundtrip(m: SignalingMessage) -> SignalingMessage {
    let json = serde_json::to_value(&m).expect("serialize");
    serde_json::from_value(json).expect("deserialize")
}

fn assert_tag(m: &SignalingMessage, tag: &str) {
    let v = serde_json::to_value(m).expect("serialize");
    assert_eq!(v["type"], tag, "tag mismatch for {m:?}");
}

#[test]
fn client_to_server_variants_roundtrip() {
    let cases = vec![
        SignalingMessage::Join {
            v: 1,
            jwt: "tok".into(),
        },
        SignalingMessage::FloorRequest {
            priority: 3,
            preempt: true,
        },
        SignalingMessage::FloorRelease,
        SignalingMessage::ModeChange {
            mode: FloorMode::Hybrid,
        },
        SignalingMessage::MuteSet {
            peer: pid("p1"),
            on: true,
        },
        SignalingMessage::TransportCreate,
        SignalingMessage::TransportConnect {
            dtls: serde_json::json!({"fingerprint": "ab:cd"}),
        },
        SignalingMessage::Produce {
            rtp_parameters: serde_json::json!({"codecs": []}),
        },
        SignalingMessage::Consume {
            producer_id: serde_json::json!("prod-1"),
        },
        SignalingMessage::Resync,
    ];
    for m in cases {
        assert_eq!(roundtrip(m.clone()), m);
    }
}

#[test]
fn server_to_client_variants_roundtrip() {
    let info = PeerInfo {
        id: pid("p1"),
        role: Role::Field,
        connected_since_ms: 1_000,
    };
    let cases = vec![
        SignalingMessage::Welcome {
            v: 1,
            peer_id: pid("p1"),
            turn_creds: serde_json::json!({"uris": ["turn:host"]}),
        },
        SignalingMessage::RouterCaps {
            media_codecs: serde_json::json!([{"mimeType": "audio/opus"}]),
        },
        SignalingMessage::PeerList {
            peers: vec![info.clone()],
            generation: 7,
        },
        SignalingMessage::PeerJoined {
            info: info.clone(),
            generation: 8,
        },
        SignalingMessage::PeerLeft {
            peer: pid("p1"),
            generation: 9,
        },
        SignalingMessage::FloorGranted {
            grants: vec![pid("p1")],
            generation: 10,
        },
        SignalingMessage::FloorTaken {
            by: pid("p2"),
            generation: 11,
        },
        SignalingMessage::FloorDenied {
            reason: DenyReason::Busy,
            generation: 12,
        },
        SignalingMessage::FloorQueued {
            position: 7,
            generation: 99,
        },
        SignalingMessage::FloorIdle {
            generation: 13,
            reason: Some("timeout".into()),
        },
        SignalingMessage::TransportInfo {
            ice: serde_json::json!({"ufrag": "x"}),
            dtls: serde_json::json!({}),
            addrs: serde_json::json!([]),
        },
        SignalingMessage::ProduceOk {
            producer_id: serde_json::json!("prod-1"),
        },
        SignalingMessage::ConsumeOk {
            producer_id: serde_json::json!("prod-1"),
            consumer_id: serde_json::json!("cons-1"),
            rtp_parameters: serde_json::json!({}),
        },
        SignalingMessage::MediaRestart {
            room: RoomId::from("r1"),
            reason: "sfu-recovered".into(),
        },
        SignalingMessage::MediaFailed { peer: pid("p3") },
        SignalingMessage::TokenRefresh { jwt: "new".into() },
    ];
    for m in cases {
        assert_eq!(roundtrip(m.clone()), m);
    }
}

#[test]
fn snapshot_roundtrips_in_both_role_scopes() {
    let field = FieldSnapshot {
        mode: FloorMode::Exclusive,
        grants: vec![pid("a")],
        muted: vec![pid("b")],
        generation: 5,
        peers: vec![PeerInfo {
            id: pid("a"),
            role: Role::Dispatch,
            connected_since_ms: 0,
        }],
    };
    let dispatch = DispatchSnapshot {
        queue: vec![talkservo_core::wire::Pending {
            peer: pid("c"),
            priority: 2,
        }],
        field: field.clone(),
    };

    let m_field = SignalingMessage::ServerSnapshot {
        payload: ServerSnapshotPayload::Field(field),
        generation: 5,
    };
    let m_dispatch = SignalingMessage::ServerSnapshot {
        payload: ServerSnapshotPayload::Dispatch(dispatch),
        generation: 6,
    };
    assert_eq!(roundtrip(m_field.clone()), m_field);
    assert_eq!(roundtrip(m_dispatch.clone()), m_dispatch);

    // dispatch payload embeds "dispatch" tag + queue visible; field payload has no queue key
    let v = serde_json::to_value(&m_dispatch).unwrap();
    assert_eq!(v["payload"]["payload"], "dispatch");
    assert!(v["payload"]["queue"].is_array());
    let v = serde_json::to_value(&m_field).unwrap();
    assert_eq!(v["payload"]["payload"], "field");
    assert!(v["payload"].get("queue").is_none());
}

#[test]
fn error_variant_roundtrip() {
    let m = SignalingMessage::Error {
        code: "bad_version".into(),
        detail: "unsupported protocol version".into(),
    };
    assert_tag(&m, "error");
    assert_eq!(roundtrip(m.clone()), m);
}

#[test]
fn all_tags_are_snake_case() {
    let probes = vec![
        (SignalingMessage::FloorRequest {
            priority: 1,
            preempt: false,
        }, "floor_request"),
        (SignalingMessage::TransportCreate, "transport_create"),
        (SignalingMessage::MediaRestart {
            room: RoomId::from("r"),
            reason: String::new(),
        }, "media_restart"),
    ];
    for (m, tag) in probes {
        assert_tag(&m, tag);
    }
}

#[test]
fn closed_deny_reason_rejects_unknown_string() {
    let raw = serde_json::json!({"type":"floor_denied","reason":"wat","generation":1});
    let err = serde_json::from_value::<SignalingMessage>(raw).expect_err("must reject");
    assert!(err.is_data(), "unknown DenyReason → data error, got {err}");
}

#[test]
fn generation_is_required_on_floor_messages() {
    // D16: receivers drop stale views — the generation field is load-bearing,
    // its absence must fail deserialization.
    let raw = serde_json::json!({"type":"floor_granted","grants":["a"]});
    assert!(serde_json::from_value::<SignalingMessage>(raw).is_err());
}

#[test]
fn ids_serialize_as_plain_strings() {
    // keeps TS unions simple: PeerId is a string on the wire, not a wrapper object
    let v = serde_json::to_value(pid("peer-9")).unwrap();
    assert_eq!(v, serde_json::json!("peer-9"));
}
