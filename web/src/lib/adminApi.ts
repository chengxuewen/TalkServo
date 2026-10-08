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

export interface Issued {
  jwt: string;
  field_url: string;
  dispatch_url: string;
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

  overview(): Promise<Overview> {
    return this.call("GET", "/admin/api/overview");
  }
  rooms(): Promise<RoomRow[]> {
    return this.call("GET", "/admin/api/rooms");
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
