// Admin console unit smoke (plan-5 S2): gate auth state, monitor rendering
// from mocked /admin/api, token issuance link shapes. fetch is stubbed —
// component wiring only, server contract lives in admin_api.rs tests.

import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { ConfigProvider, theme } from "antd";
import React from "react";
import { AdminPage } from "../src/pages/admin.js";
import { setAdminJwt, adminJwt } from "../src/lib/adminApi.js";

const routes: Record<string, unknown> = {
  "/admin/api/overview": { rooms: 1, peers: 2, uptime_s: 5, backend: "stub" },
  "/admin/api/rooms": [
    {
      room: "ops-01",
      mode: "ptt",
      holders: ["alice"],
      queue_len: 1,
      peers: [{ id: "alice", connected_since_ms: 0 }, { id: "bob", connected_since_ms: 0 }],
      muted: ["bob"],
      speaking: ["alice"],
    },
    { room: "ghost", stale: true },
  ],
};

function mockFetch() {
  globalThis.fetch = vi.fn(async (url: unknown) => ({
    ok: true,
    status: 200,
    json: async () => routes[String(url)] ?? { error: "not mocked" },
  })) as never;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  setAdminJwt(null);
});

function ui() {
  return render(
    <ConfigProvider theme={{ algorithm: theme.darkAlgorithm }}>
      <MemoryRouter initialEntries={["/admin"]}>
        <AdminPage />
      </MemoryRouter>
    </ConfigProvider>,
  );
}

describe("admin console", () => {
  it("renders gate when no jwt", () => {
    ui();
    expect(screen.getByText("Admin access")).toBeTruthy();
  });

  it("password login mints and stores the admin jwt (D22)", async () => {
    globalThis.fetch = vi.fn(async (url: unknown, init?: { method?: string; body?: string }) => {
      if (String(url) === "/admin/api/login") {
        const { user, password } = JSON.parse(String(init?.body)) as { user: string; password: string };
        const ok = user === "ops-1" && password === "s3cret";
        return {
          ok,
          status: ok ? 200 : 401,
          json: async () => (ok ? { jwt: "minted-jwt" } : { error: "invalid credentials" }),
        };
      }
      return { ok: true, status: 200, json: async () => routes[String(url)] ?? {} };
    }) as never;
    ui();
    fireEvent.change(screen.getByPlaceholderText("Username"), { target: { value: "ops-1" } });
    fireEvent.change(screen.getByPlaceholderText("Password"), { target: { value: "s3cret" } });
    fireEvent.click(screen.getByText("Log in"));
    expect(await screen.findByText("ops-01")).toBeTruthy(); // logged in -> monitor
    expect(adminJwt()).toBe("minted-jwt");
  });

  it("monitor shows holders/speaking/muted/stale + overview cards", async () => {
    setAdminJwt("tok");
    mockFetch();
    ui();
    expect(await screen.findByText("ops-01")).toBeTruthy();
    expect(screen.getByText("alice")).toBeTruthy();
    expect(screen.getByText("speaking")).toBeTruthy();
    expect(screen.getByText("bob")).toBeTruthy();
    expect(screen.getByText("stale")).toBeTruthy();
    expect(screen.getByText(/Rooms 1/)).toBeTruthy();
    expect(screen.getByText(/Backend stub/)).toBeTruthy();
  });

  it("401 from an api call bounces to the gate and clears the stored jwt", async () => {
    setAdminJwt("expired-token");
    globalThis.fetch = vi.fn(async () => ({
      ok: false,
      status: 401,
      json: async () => ({ error: "expired" }),
    })) as never;
    ui();
    // first poll fires on mount -> 401 -> onUnauthorized -> gate re-renders
    expect(await screen.findByText("Username")).toBeTruthy();
    expect(adminJwt()).toBeNull();
  });

  it("tokens form posts and shows copy-ready links", async () => {
    setAdminJwt("tok");
    mockFetch();
    ui();
    fireEvent.click(await screen.findByText("Tokens"));
    fireEvent.change(screen.getByPlaceholderText("field-7"), { target: { value: "dev-9" } });
    fireEvent.change(screen.getByPlaceholderText("ops-01"), { target: { value: "ops-01" } });
    (globalThis.fetch as ReturnType<typeof vi.fn>).mockImplementation(async (url: unknown) =>
      String(url) === "/admin/api/tokens"
        ? {
            ok: true,
            status: 200,
            json: async () => ({
              jwt: "eyJmock",
              field_url: "https://lan:8443/field/ops-01?jwt=eyJmock",
              dispatch_url: "https://lan:8443/dispatch/ops-01?jwt=eyJmock",
              participant_url: "https://lan:8443/room/ops-01?jwt=eyJmock",
            }),
          }
        : { ok: true, status: 200, json: async () => routes[String(url)] },
    );
    fireEvent.click(screen.getByRole("button", { name: /^Issue$/ }));
    await waitFor(() =>
      expect(screen.getByText(/field: https:\/\/lan:8443\/field\/ops-01\?jwt=/)).toBeTruthy(),
    );
    await waitFor(() =>
      expect(screen.getByText(/participant: https:\/\/lan:8443\/room\/ops-01\?jwt=/)).toBeTruthy(),
    );
  });
});
