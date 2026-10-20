// Thin fetch wrapper for the out-of-band admin REST (plan-5 S2, D18).
// The admin JWT lives in sessionStorage ONLY (tab-scoped, never persisted).
// 401 -> onUnauthorized so the page bounces to its gate card. No wire/SDK
// overlap: this is deliberately NOT in packages/client (D14 scope: signaling
// domain); the Electron shell wraps the same SPA, so web-side is correct.

const KEY = "ts-a…t";

export function adminJwt(): string | null {
  return sessionStorage.getItem(KEY);
}

export function setAdminJwt(token: string | null): void {
  if (token) sessionStorage.setItem(KEY, token);
  else sessionStorage.removeItem(KEY);
}

export interface PeerRow {
  id: string;
  connected_since_ms: number;
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
    const body = (await res.json()) as { jwt?: string };
    if (!body.jwt) throw new Error("login: response missing jwt");
    setAdminJwt(body.jwt);
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
  createAccount(user: string, password: string): Promise<void> {
    return this.callVoid("POST", "/admin/api/accounts", { user, password });
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
  issue(req: {
    sub: string;
    room: string;
    role: "dispatch" | "field";
    ttl_s?: number;
  }): Promise<Issued> {
    return this.call("POST", "/admin/api/tokens", req);
  }
}
