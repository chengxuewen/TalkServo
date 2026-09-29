// Wire store: client-side floor-state mirror. Generation discipline (D16):
// events with gen < seen are DROPPED; event ordering is trusted to the server's
// single-writer (in-order per WS); role-scoped snapshots reshape the mirror.

import type { DispatchSnapshot, FieldSnapshot, FloorMode, PeerInfo, Pending, ServerMessage } from "./types.js";

export interface FloorMirror {
  mode: FloorMode;
  grants: string[];
  muted: string[];
  queue: Pending[]; // empty for field role
  peers: PeerInfo[];
  generation: number;
  /** My peer id (from Welcome). */
  selfId: string | null;
}

export function initialMirror(): FloorMirror {
  return {
    mode: "hybrid",
    grants: [],
    muted: [],
    queue: [],
    peers: [],
    generation: 0,
    selfId: null,
  };
}

/** Reduction: fold one server message into the mirror. Returns the NEW mirror
 *  (immutable) or null when the message is stale/discarded. */
export function reduce(m: FloorMirror, msg: ServerMessage, role: "dispatch" | "field"): FloorMirror | null {
  switch (msg.type) {
    case "welcome":
      return { ...m, selfId: msg.peer_id };

    case "peer_list":
      if (msg.generation < m.generation) return null; // stale
      return { ...m, peers: msg.peers, generation: msg.generation };

    case "peer_joined":
      if (msg.generation < m.generation) return null;
      return {
        ...m,
        peers: [...m.peers.filter((p) => p.id !== msg.info.id), msg.info],
        generation: msg.generation,
      };

    case "peer_left":
      if (msg.generation < m.generation) return null;
      return {
        ...m,
        peers: m.peers.filter((p) => p.id !== msg.peer),
        generation: msg.generation,
      };

    case "floor_granted":
      if (msg.generation < m.generation) return null;
      return { ...m, grants: msg.grants, generation: msg.generation };

    case "floor_taken":
      if (msg.generation < m.generation) return null;
      // old holder displaced; grants arrive in the follow-up FloorGranted
      return { ...m, generation: msg.generation };

    case "floor_denied":
      if (msg.generation < m.generation) return null;
      return { ...m, generation: msg.generation };

    case "floor_queued":
      if (msg.generation < m.generation) return null;
      return { ...m, generation: msg.generation };

    case "floor_idle":
      if (msg.generation < m.generation) return null;
      return { ...m, grants: [], generation: msg.generation };

    case "media_failed":
      // informational; the authoritative state change arrives as a floor event
      return m;

    case "server_snapshot": {
      if (msg.generation < m.generation) return null;
      const p = msg.payload;
      // D12: a field-role client must never adopt a dispatch-scoped snapshot
      // (queue visibility). Wire shouldn't send it; defense anyway.
      if (role === "field" && p.payload === "dispatch") return null;
      if (p.payload === "field") {
        const f = p as { payload: "field" } & FieldSnapshot;
        return {
          mode: f.mode,
          grants: f.grants,
          muted: f.muted,
          queue: [], // field role never sees the queue (D12)
          peers: f.peers,
          generation: f.generation,
          selfId: m.selfId,
        };
      }
      const d = p as { payload: "dispatch" } & DispatchSnapshot;
      return {
        mode: d.mode,
        grants: d.grants,
        muted: d.muted,
        queue: d.queue,
        peers: d.peers,
        generation: d.generation,
        selfId: m.selfId,
      };
    }

    case "token_refresh":
    case "router_caps":
    case "transport_info":
    case "produce_ok":
    case "consume_ok":
    case "media_restart":
    case "error":
      return m; // not mirror state

    default: {
      // role never changes mid-session from the wire; keep TS exhaustive
      void role;
      return m;
    }
  }
}
