// Plan-9 W2 — the console's use of `/admin/ws`.
//
// What these cases own: push must actually paint the screen, a burst of hints must
// cost ONE incremental fetch, and a dead socket must be VISIBLE. The last one is
// the whole point of R2=A: a green console that stopped listening is the failure
// mode this wave was created to remove, so "no indicator" is a bug here, not a
// cosmetic gap.
//
// The socket is injected through `window.__tsAdminWsFactory` (see
// src/pages/admin/index.tsx); nothing reaches the network.
import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { ConfigProvider, theme } from "antd";
import React from "react";
import { AdminPage } from "../src/pages/admin/index.js";
import { setAdminJwt } from "../src/lib/adminApi.js";
import type { Sock } from "../src/lib/adminEventsClient.js";
import type { AdminEvent, AdminIn } from "../src/lib/adminEvents.js";

class FakeSock implements Sock {
  sent: AdminIn[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((ev: { data: unknown }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  constructor(public url: string) {}
  send(data: string): void {
    this.sent.push(JSON.parse(data) as AdminIn);
  }
  close(): void {
    this.onclose?.();
  }
  /** server → console frame */
  push(e: AdminEvent): void {
    this.onmessage?.({ data: JSON.stringify(e) });
  }
  framesOfType(t: string): AdminIn[] {
    return this.sent.filter((f) => f.type === t);
  }
}

let last: FakeSock | null = null;

function installSocket(): FakeSock {
  const make = (url: string) => {
    last = new FakeSock(url);
    // the real socket opens on a macrotask; mirror it so onopen is assigned first
    setTimeout(() => last?.onopen?.(), 0);
    return last;
  };
  (window as unknown as { __tsAdminWsFactory?: (u: string) => Sock }).__tsAdminWsFactory =
    make;
  return last as FakeSock;
}

const POLL_BODY = { rooms: 1, peers: 1, uptime_s: 5, backend: "stub" };

function mockFetch(auditRows: () => unknown[]) {
  const auditCalls: string[] = [];
  globalThis.fetch = vi.fn(async (url: unknown) => {
    const u = String(url);
    if (u.startsWith("/admin/api/audit")) auditCalls.push(u);
    const body = u.startsWith("/admin/api/rooms/live")
      ? [{ room: "polled-room", mode: "exclusive", holders: [], peers: [] }]
      : u.startsWith("/admin/api/overview")
        ? POLL_BODY
        : u.startsWith("/admin/api/audit")
          ? { events: auditRows(), dropped: 0 }
          : {};
    return {
      ok: true,
      status: 200,
      json: async () => body,
    };
  }) as never;
  return auditCalls;
}

function ui() {
  setAdminJwt("tok");
  return render(
    <ConfigProvider theme={{ algorithm: theme.darkAlgorithm }}>
      <MemoryRouter initialEntries={["/admin"]}>
        <AdminPage />
      </MemoryRouter>
    </ConfigProvider>,
  );
}

afterEach(() => {
  cleanup();
  setAdminJwt(null);
  vi.unstubAllGlobals();
  delete (window as unknown as { __tsAdminWsFactory?: unknown }).__tsAdminWsFactory;
  last = null;
});

describe("admin live link (plan-9 W2)", () => {
  it("authenticates by frame, never by URL, and reports live on hello", async () => {
    installSocket();
    mockFetch(() => []);
    ui();
    await waitFor(() => expect(last).toBeTruthy());
    const sock = last as FakeSock;
    expect(sock.url).toMatch(/\/admin\/ws$/);
    expect(sock.url).not.toContain("tok"); // the token never rides the URL

    await waitFor(() => expect(sock.framesOfType("auth")).toHaveLength(1));
    expect(sock.framesOfType("auth")[0]).toEqual({
      type: "auth",
      token: "tok",
    });
    expect(sock.framesOfType("subscribe")).toHaveLength(1);

    // connecting is NOT live: the degrade notice must stay up until hello lands
    expect(await screen.findByTestId("push-state")).toBeTruthy();
    sock.push({ type: "hello", v: 1, dropped: 0, debounce_ms: 150 });
    await waitFor(() => expect(screen.queryByTestId("push-state")).toBeNull());
  });

  it("rooms_live paints a room the poll never returned", async () => {
    installSocket();
    mockFetch(() => []);
    ui();
    const sock = await waitFor(() => {
      expect(last).toBeTruthy();
      return last as FakeSock;
    });
    await screen.findByText("polled-room"); // the reconcile/poll path first
    sock.push({
      type: "hello",
      v: 1,
      dropped: 0,
      debounce_ms: 150,
    });
    sock.push({
      type: "rooms_live",
      v: 1,
      rooms: [{ room: "pushed-room", mode: "open", holders: [], peers: [] }],
    });
    // the push, not a poll, is what put it there: nothing else ever returned it
    expect(await screen.findByText("pushed-room")).toBeTruthy();
  });

  it("a burst of hints costs exactly one incremental fetch, and rows still come from the API", async () => {
    let seq = 100;
    installSocket();
    const auditCalls = mockFetch(() => [
      { seq: seq++, ts_ms: 1, event: "kick", room: "r1", actor: "admin" },
    ]);
    ui();
    fireEvent.click(await screen.findByText("Logs"));
    await waitFor(() => expect(auditCalls.length).toBeGreaterThan(0));
    const before = auditCalls.length;
    const fullBefore = auditCalls.filter((u) => !u.includes("after=")).length;

    const sock = last as FakeSock;
    seq = 200;
    // five separate macrotasks, all inside one debounce window
    for (let i = 0; i < 5; i += 1) {
      sock.push({ type: "audit_hint", v: 1 });
      await new Promise((r) => setTimeout(r, 10));
    }
    sock.push({ type: "hello", v: 1, dropped: 0, debounce_ms: 150 });
    await waitFor(() => expect(auditCalls.length).toBeGreaterThan(before));
    await new Promise((r) => setTimeout(r, 400));
    const incremental = auditCalls.filter((u) => u.includes("after="));
    expect(
      incremental.length,
      `5 hints must coalesce into one incremental fetch, saw: ${JSON.stringify(incremental)}`,
    ).toBe(1);
    // the sharper half: no hint may re-trigger the FULL page fetch — that is the
    // regression that made this test pass for the wrong reason once already
    expect(
      auditCalls.filter((u) => !u.includes("after=")).length,
      "hints must not fall back to full refetches",
    ).toBe(fullBefore);
    // R3=B held: the row arrived over the API cursor, not inside the hint frame
    expect(await screen.findByText("200")).toBeTruthy();
  });

  it("a dropped socket becomes visible instead of quietly freezing the screen", async () => {
    installSocket();
    mockFetch(() => []);
    ui();
    const sock = last as FakeSock;
    sock.push({ type: "hello", v: 1, dropped: 0, debounce_ms: 150 });
    await waitFor(() => expect(screen.queryByTestId("push-state")).toBeNull());
    sock.close();
    expect(await screen.findByTestId("push-state")).toBeTruthy();
    expect(screen.getByTestId("push-state").textContent).toContain("down");
  });
});
