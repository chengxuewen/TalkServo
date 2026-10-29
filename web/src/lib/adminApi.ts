// Thin fetch wrapper for the out-of-band admin REST (plan-5 S2, D18).
// The admin JWT lives in sessionStorage ONLY (tab-scoped, never persisted).
// 401 -> onUnauthorized so the page bounces to its gate card. No wire/SDK
// overlap: this is deliberately NOT in packages/client (D14 scope: signaling
// domain); the Electron shell wraps the same SPA, so web-side is correct.

const KEY = "ts-admin-jwt";
const ROLE_KEY = "ts-admin-role";

export function adminRole(): string | null {
  return sessionStorage.getItem(ROLE_KEY);
}

export function adminJwt(): string | null {
  return sessionStorage.getItem(KEY);
}

export function setAdminJwt(token: string | null): void {
  if (token) sessionStorage.setItem(KEY, token);
  else sessionStorage.removeItem(KEY);
  if (!token) sessionStorage.removeItem(ROLE_KEY);
}

export interface PeerRow {
  id: string;
  connected_since_ms: number;
  role?: "dispatch" | "field";
  mod_rooms?: string[];
  granted?: boolean;
  muted?: boolean;
}

/** rooms.yaml row (plan-7 S2 registry; optional fields fall back to globals). */
export interface RoomConfigRow {
  room: string;
  owner: string;
  mode?: "exclusive" | "open" | "hybrid";
  max_peers?: number;
  max_queue?: number;
  locked: boolean;
}

export interface RoomRow {
  room: string;
  stale?: boolean;
  mode?: string;
  holders?: string[];
  queue_len?: number;
  peers?: PeerRow[];
  muted?: string[];
  speaking?: string[];
}

export interface Overview {
  rooms: number;
  peers: number;
  uptime_s: number;
  backend: string;
}

export interface AccountRow {
  user: string;
  role: string;
}

export interface AuditRow {
  seq: number;
  ts_ms: number;
  event: string;
  room?: string;
  actor?: string;
  target?: string;
  reason?: string;
  changes?: { key: string; old: unknown; new: unknown }[];
}

export interface AuditPage {
  events: AuditRow[];
  dropped: number;
}

export interface Issued {
  jwt: string;
  field_url: string;
  dispatch_url: string;
  participant_url: string;
}

export class AdminApi {
  constructor(private readonly onUnauthorized: () => void) {}

  private async call<T>(method: string, path: string, body?: unknown): Promise<T> {
    const res = await fetch(path, {
      method,
      headers: {
        ...(body ? { "content-type": "application/json" } : {}),
        authorization: `Bearer ${adminJwt() ?? ""}`,
      },
      body: body ? JSON.stringify(body) : undefined,
    });
    if (res.status === 401 || res.status === 503) {
      this.onUnauthorized();
      throw new Error(`admin api ${res.status}`);
    }
    if (!res.ok) throw new Error(`admin api ${res.status}`);
    return (await res.json()) as T;
  }

  /** D22 password login: server verifies the PBKDF2 hash from server.yaml
   * and mints a short-lived admin JWT. Errors surface as `login <status>`
   * (401 wrong password, 503 password login not configured). */
  async login(user: string, password: string): Promise<string> {
    const res = await fetch("/admin/api/login", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ user, password }),
    });
    if (!res.ok) throw new Error(`login ${res.status}`);
    const body = (await res.json()) as { jwt?: string; role?: string };
    if (!body.jwt) throw new Error("login: response missing jwt");
    setAdminJwt(body.jwt);
    sessionStorage.setItem(ROLE_KEY, body.role ?? "admin");
    return body.jwt;
  }

  private async callVoid(method: string, path: string, body?: unknown): Promise<void> {
    const res = await fetch(path, {
      method,
      headers: {
        ...(body ? { "content-type": "application/json" } : {}),
        authorization: `Bearer ${adminJwt() ?? ""}`,
      },
      body: body ? JSON.stringify(body) : undefined,
    });
    if (res.status === 401 || res.status === 503) {
      this.onUnauthorized();
      throw new Error(`admin api ${res.status}`);
    }
    if (!res.ok) throw new Error(`admin api ${res.status}`);
  }

  /** Account registry rows — never carries hashes (server strips them). */
  accounts(): Promise<AccountRow[]> {
    return this.call("GET", "/admin/api/accounts");
  }
  /** Create or reset a password (upsert — sister Accounts semantics). */
  /** S5: create carries the tier (absent = server default admin). */
  createAccount(user: string, password: string, role?: string): Promise<void> {
    return this.callVoid("POST", "/admin/api/accounts", { user, password, role });
  }
  removeAccount(user: string): Promise<void> {
    return this.callVoid("DELETE", `/admin/api/accounts/${encodeURIComponent(user)}`);
  }

  overview(): Promise<Overview> {
    return this.call("GET", "/admin/api/overview");
  }
  roomsLive(): Promise<RoomRow[]> {
    return this.call("GET", "/admin/api/rooms/live");
  }

  roomsRegistry(): Promise<RoomConfigRow[]> {
    return this.call("GET", "/admin/api/rooms");
  }

  createRoom(row: Partial<RoomConfigRow> & { room: string }): Promise<void> {
    return this.callVoid("POST", "/admin/api/rooms", row);
  }

  deleteRoom(room: string): Promise<void> {
    return this.callVoid("DELETE", `/admin/api/rooms/${encodeURIComponent(room)}`);
  }
  /** S6 intervention kit (R2: reason goes to the audit trail, never the wire). */
  kickPeer(room: string, peer: string, reason?: string): Promise<void> {
    return this.callVoid(
      "DELETE",
      `/admin/api/rooms/${encodeURIComponent(room)}/peers/${encodeURIComponent(peer)}`,
      reason ? { reason } : undefined,
    );
  }
  forceRelease(room: string, peer: string, reason?: string): Promise<void> {
    return this.callVoid(
      "POST",
      `/admin/api/rooms/${encodeURIComponent(room)}/peers/${encodeURIComponent(peer)}/force-release`,
      reason ? { reason } : undefined,
    );
  }
  // `after` is the server's seq cursor (audit.rs AuditQuery): the push plane gives a
  // hint, this asks for just what is newer — the ONLY way a row enters the table
  audit(q: { event?: string; room?: string; actor?: string; limit?: number; after?: number } = {}): Promise<AuditPage> {
    const ps = new URLSearchParams();
    if (q.after != null) ps.set("after", String(q.after));
    if (q.event) ps.set("event", q.event);
    if (q.room) ps.set("room", q.room);
    if (q.actor) ps.set("actor", q.actor);
    if (q.limit) ps.set("limit", String(q.limit));
    const qs = ps.toString();
    return this.call("GET", `/admin/api/audit${qs ? `?${qs}` : ""}`);
  }
  issue(req: {
    /** empty/omitted = server-generated subject (D29-#4: u-xxxxxx, fixed at issue) */
    sub?: string;
    room: string;
    role: "dispatch" | "field";
    ttl_s?: number;
    mod_rooms?: string[];
  }): Promise<Issued> {
    return this.call("POST", "/admin/api/tokens", req);
  }
}
