// S-3 React owner rule: ONE module-scope client per room/role pair, exposed to
// views through useSyncExternalStore. StrictMode double-mounts are safe:
// subscribe is idempotent and the client is never re-created per mount.

import { useSyncExternalStore } from "react";
import {
  TalkServoClient,
  MediaManager,
  MediasoupClientStack,
  type ClientEvent,
  type FloorMirror,
  type Role,
  type LikeWebSocket,
} from "@talkservo/client";

export interface RoomSession {
  client: TalkServoClient;
  media: MediaManager;
  mirror: FloorMirror;
  events: ClientEvent[];
  connected: boolean;
  selfId: string | null;
  /** S-F4: in-flight connect promise — StrictMode double-invoke awaits the
   *  SAME attempt instead of minting a second transport (which would trip
   *  already_joined and leave a dead handle on the session). */
  connectPromise: Promise<void> | null;
}

interface SessionKey {
  room: string;
  role: Role;
}

const EMPTY_MIRROR: FloorMirror = {
  mode: "hybrid",
  grants: [],
  muted: [],
  queue: [],
  peers: [],
  generation: 0,
  selfId: null,
};
const EMPTY_EVENTS: ClientEvent[] = [];
const sessions = new Map<string, RoomSession>();

export function sessionKey({ room, role }: SessionKey): string {
  return `${role}:${room}`;
}

export function getSession(key: string): RoomSession | undefined {
  return sessions.get(key);
}

/** Idempotent create-or-get (StrictMode safe). */
export function ensureSession(
  { room, role }: SessionKey,
  wsFactory?: (url: string) => LikeWebSocket,
): RoomSession {
  const key = sessionKey({ room, role });
  const existing = sessions.get(key);
  if (existing) return existing;

  const client = new TalkServoClient({ role, ...(wsFactory ? { wsFactory } : {}) });
  const media = new MediaManager({ stack: new MediasoupClientStack() });
  const session: RoomSession = {
    client,
    media,
    mirror: {
      mode: "hybrid",
      grants: [],
      muted: [],
      queue: [],
      peers: [],
      generation: 0,
      selfId: null,
    },
    events: [],
    connected: false,
    selfId: null,
    connectPromise: null,
  };
  media.attach(client);
  client.on((e) => {
    switch (e.kind) {
      case "state":
        session.mirror = e.mirror;
        break;
      case "connected":
        session.connected = true;
        session.selfId = e.selfId;
        break;
      case "disconnected":
        session.connected = false;
        break;
      default:
        break;
    }
    if (e.kind === "denied" || e.kind === "taken" || e.kind === "error") {
      session.events = [e, ...session.events].slice(0, 100);
    }
    notify(key);
  });
  sessions.set(key, session);
  return session;
}

/** Connect the session (JWT minted by the host page or dev helper).
 *  Idempotent: concurrent/StrictMode-doubled calls share one attempt;
 *  an already-connected session is a no-op. */
export async function connectSession(
  key: string,
  url: string,
  jwt: string,
): Promise<void> {
  const s = sessions.get(key);
  if (!s) throw new Error(`no session ${key}`);
  if (s.connectPromise) return s.connectPromise;
  if (s.connected) return;
  s.connectPromise = s.client
    .connect(url, jwt)
    .then(async () => {
      await wireMedia(s);
      s.connectPromise = null;
    })
    .catch((err) => {
      s.connectPromise = null; // allow retry (fresh JWT, transient net)
      throw err;
    });
  await s.connectPromise;
}

export function dropSession(key: string): void {
  const s = sessions.get(key);
  if (s) {
    s.client.disconnect();
    sessions.delete(key);
  }
}

// ── React binding ───────────────────────────────────────────────────────────

const listeners = new Set<() => void>();
function notify(key: string) {
  invalidateStatus(key);
  for (const fn of listeners) fn();
}
function subscribe(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

/** Media orchestration: device load + transports + produce/consume wiring.
 *  Browser-only (mediasoup-stack); unit tests never hit this path. */
async function wireMedia(s: RoomSession): Promise<void> {
  if (typeof window === "undefined") return;
  const stack = s.media.stackRef();
  if (!stack) return;
  try {
    // RouterCaps arrived during the join burst — device loads from it
    await stack.load(s.media.routerCaps ?? {});
    // transports: TransportCreate → TransportInfo pairs ride the raw wire
    const info = await s.client.requestTransport();
    await stack.createSendTransport({ ice: info.ice, dtls: info.dtls });
    await stack.createRecvTransport({ ice: info.ice, dtls: info.dtls });
    // mic publish (paused server-side until granted — D13)
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    const track = stream.getAudioTracks()[0];
    if (track) await s.media.publishMic(track);
    s.media.setRecvFactory(async () => {
      if (!("createRecvTransport" in stack)) throw new Error("no recv");
      return stack.createRecvTransport({ ice: info.ice, dtls: info.dtls });
    });
  } catch (err) {
    console.warn("[talkservo] media setup failed (signaling still live):", err);
  }
}

/** Live mirror snapshot for a session (re-renders on wire events). */
export function useMirror(key: string): FloorMirror {
  return useSyncExternalStore(
    subscribe,
    () => sessions.get(key)?.mirror ?? EMPTY_MIRROR,
    () => EMPTY_MIRROR,
  );
}

export interface SessionStatus {
  connected: boolean;
  selfId: string | null;
  events: ClientEvent[];
}

const EMPTY_STATUS: SessionStatus = { connected: false, selfId: null, events: EMPTY_EVENTS };

/** Cached status objects: getSnapshot must return a STABLE reference between
 *  notifications, or React re-renders forever. */
const statusCache = new Map<string, SessionStatus>();

function statusOf(key: string): SessionStatus {
  let st = statusCache.get(key);
  if (!st) {
    const s = sessions.get(key);
    st = {
      connected: s?.connected ?? false,
      selfId: s?.selfId ?? null,
      events: s?.events ?? EMPTY_EVENTS,
    };
    statusCache.set(key, st);
  }
  return st;
}

function invalidateStatus(key: string) {
  statusCache.delete(key);
}

/** Session status bits (connected/selfId/events). */
export function useSessionStatus(key: string): SessionStatus {
  return useSyncExternalStore(
    subscribe,
    () => statusOf(key),
    () => EMPTY_STATUS,
  );
}



/** Dev helper: mint a token via the server's issue-token endpoint shape.
 *  PoC: the page reads JWT from the query string (?jwt=...) — the dispatcher
 *  mints out-of-band via `pixi run issue-token` until the admin surface. */
export function jwtFromQuery(): string | null {
  return new URLSearchParams(window.location.search).get("jwt");
}
