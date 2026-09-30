// Wire contract mirror — docs/modules/02 + crates/talkservo-core/src/wire.rs.
// Field names snake_case exactly as the wire (serde rename_all). Any drift
// between this file and the Rust enum is a CI failure (drift gate, plan-3 T1).

export type FloorMode = "exclusive" | "open" | "hybrid";
export type DenyReason =
  | "busy"
  | "exceeds_ceiling"
  | "not_member"
  | "rate_limited"
  | "preempt_priority"
  | "no_media";
export type Role = "dispatch" | "field";

export interface PeerInfo {
  id: string;
  role: Role;
  connected_since_ms: number;
}

export interface Pending {
  peer: string;
  priority: number;
}

export interface FieldSnapshot {
  mode: FloorMode;
  grants: string[];
  muted: string[];
  generation: number;
  peers: PeerInfo[];
}

export interface DispatchSnapshot extends FieldSnapshot {
  queue: Pending[];
}

/** serde(tag="payload") flattens the snapshot fields into the tagged object. */
export type ServerSnapshotPayload =
  | ({ payload: "field" } & FieldSnapshot)
  | ({ payload: "dispatch" } & DispatchSnapshot);

export interface TransportInfo {
  ice: unknown;
  dtls: unknown;
  addrs: unknown;
}

// ── wire messages (serde tag = "type", snake_case) ─────────────────────────

export type ClientMessage =
  | { type: "join"; v: number; jwt: string }
  | { type: "floor_request"; priority: number; preempt: boolean }
  | { type: "floor_release" }
  | { type: "mode_change"; mode: FloorMode }
  | { type: "mute_set"; peer: string; on: boolean }
  | { type: "transport_create" }
  | { type: "transport_connect"; dtls: unknown }
  | { type: "produce"; rtp_parameters: unknown }
  | { type: "consume"; producer_id: unknown }
  | { type: "resync" };

export type ServerMessage =
  | { type: "welcome"; v: number; peer_id: string; turn_creds: unknown }
  | { type: "router_caps"; media_codecs: unknown }
  | { type: "peer_list"; peers: PeerInfo[]; generation: number }
  | { type: "peer_joined"; info: PeerInfo; generation: number }
  | { type: "peer_left"; peer: string; generation: number }
  | { type: "floor_granted"; grants: string[]; generation: number }
  | { type: "floor_taken"; by: string; generation: number }
  | { type: "floor_denied"; reason: DenyReason; generation: number }
  | { type: "floor_queued"; position: number; generation: number }
  | { type: "floor_idle"; generation: number; reason: string | null }
  | { type: "transport_info"; ice: unknown; dtls: unknown; addrs: unknown }
  | { type: "produce_ok"; producer_id: unknown }
  | {
      type: "consume_ok";
      producer_id: unknown;
      consumer_id: unknown;
      rtp_parameters: unknown;
    }
  | { type: "media_restart"; room: string; reason: string }
  | { type: "media_failed"; peer: string }
  | { type: "token_refresh"; jwt: string }
  | { type: "server_snapshot"; payload: ServerSnapshotPayload; generation: number }
  | { type: "error"; code: string; detail: string };

export type AnyMessage = ClientMessage | ServerMessage;

// ── protocol constants ──────────────────────────────────────────────────────

/** Wire protocol version (modules/02 §1: `v:1`). */
export const PROTOCOL_VERSION = 1;

export function isServerMessage(m: unknown): m is ServerMessage {
  return (
    typeof m === "object" &&
    m !== null &&
    "type" in m &&
    typeof (m as { type: unknown }).type === "string"
  );
}
