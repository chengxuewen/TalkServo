// Admin console unit smoke (plan-5 S2): gate auth state, monitor rendering
// from mocked /admin/api, token issuance link shapes. fetch is stubbed —
// component wiring only, server contract lives in admin_api.rs tests.

import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor, within, cleanup, act } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { App, ConfigProvider, theme } from "antd";
import React from "react";
import { AdminPage } from "../src/pages/admin/index.js";
import { AdminApi } from "../src/lib/adminApi.js";
import { setAdminJwt, adminJwt } from "../src/lib/adminApi.js";

const routes: Record<string, unknown> = {
  "/admin/api/overview": { rooms: 1, peers: 2, uptime_s: 5, backend: "stub" },
  "/admin/api/rooms/live": [
    {
      room: "ops-01",
      mode: "exclusive",
      holders: ["alice"],
      queue_len: 1,
      peers: [
        { id: "alice", connected_since_ms: 0, role: "field", granted: true, muted: false },
        { id: "bob", connected_since_ms: 0, role: "field", granted: false, muted: true, mod_rooms: [] },
      ],
      muted: ["bob"],
      speaking: ["alice"],
    },
    { room: "ghost", stale: true },
  ],
  "/admin/api/rooms": [
    { room: "ops-01", owner: "squad", locked: false },
    { room: "ops-02", owner: "patrol", locked: true },
  ],
  "/admin/api/audit": {
    events: [
      { seq: 2, ts_ms: 1792470000000, event: "kick", room: "ops-01", actor: "admin", target: "bob", reason: "abuse" },
      { seq: 1, ts_ms: 1792469000000, event: "login_fail", actor: "mallory" },
    ],
    dropped: 1,
  },
};

function mockFetch() {
  globalThis.fetch = vi.fn(async (url: unknown, init?: { method?: string }) => {
    return {
      ok: true,
      status: 200,
      json: async () => routes[String(url).split("?")[0]] ?? { error: "not mocked" },
    };
  }) as never;
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
    expect(screen.getByText(/Live Rooms 1 · 2 registered/)).toBeTruthy();
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
    expect(await screen.findByPlaceholderText("Username")).toBeTruthy();
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
    fireEvent.click(screen.getByText("Issue"));
    await waitFor(() =>
      expect(screen.getByText(/field: https:\/\/lan:8443\/field\/ops-01\?jwt=/)).toBeTruthy(),
    );
    await waitFor(() =>
      expect(screen.getByText(/participant: https:\/\/lan:8443\/room\/ops-01\?jwt=/)).toBeTruthy(),
    );
  });

  it("Tokens mod-rooms multi-select puts the claims on the issued token (S4 UI)", async () => {
    setAdminJwt("tok");
    mockFetch();
    const { container } = ui();
    fireEvent.click(await screen.findByText("Tokens"));
    const item = () =>
      [...container.querySelectorAll(".ant-form-item")].find((fi) =>
        fi.textContent?.includes("Moderator of"),
      );
    await waitFor(() => expect(item()).toBeTruthy());
    fireEvent.change(within(container).getByPlaceholderText("field-7"), {
      target: { value: "squad-lead" },
    });
    fireEvent.change(within(container).getByPlaceholderText("ops-01"), {
      target: { value: "ops-01" },
    });

    // the registry options come from /admin/api/rooms (ops-01 + ops-02); AntD
    // opens a Select on mouseDown and paints the list into a body-level dropdown
    fireEvent.mouseDown(
      (item() as HTMLElement).querySelector(".ant-select-selector") as HTMLElement,
    );
    await waitFor(() =>
      expect(document.querySelectorAll(".ant-select-item-option").length).toBeGreaterThan(0),
    );
    const pick = (name: string) => {
      const opt = [...document.querySelectorAll(".ant-select-item-option")].find(
        (o) => o.textContent === name,
      ) as HTMLElement;
      expect(opt).toBeTruthy();
      fireEvent.click(opt);
    };
    pick("ops-01");
    pick("ops-02");

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
    await act(async () => {
      fireEvent.click(within(container).getByText("Issue"));
    });

    // the claim has to reach the WIRE, not just the form state (testing.md rule 1)
    await waitFor(
      () => {
        const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock
          .calls as unknown as [string, { method?: string; body?: string }][];
        const post = calls.find(
          ([u, i]) => String(u) === "/admin/api/tokens" && i?.method === "POST",
        );
        expect(post).toBeTruthy();
        const req = JSON.parse(post?.[1]?.body ?? "{}") as {
          sub?: string;
          mod_rooms?: string[];
        };
        expect(req.sub).toBe("squad-lead");
        expect(req.mod_rooms).toEqual(["ops-01", "ops-02"]);
      },
      { timeout: 4000 },
    );
  });

  it("Sider filters Accounts by tier (S5: operator sees four tabs)", async () => {
    setAdminJwt("tok");
    sessionStorage.setItem("ts-admin-role", "operator");
    mockFetch();
    ui();
    await screen.findByText("ops-01");
    expect(screen.queryByText("Accounts")).toBeNull();
    expect(screen.getByText("Logs")).toBeTruthy();
    sessionStorage.removeItem("ts-admin-role");
  });

  it("Logs tab renders events and the dropped warning (S1)", async () => {
    setAdminJwt("tok");
    mockFetch();
    ui();
    fireEvent.click(await screen.findByText("Logs"));
    expect(await screen.findByText("abuse")).toBeTruthy(); // reason column
    expect(screen.getAllByText("kick").length).toBeGreaterThan(0);
    expect(screen.getByText(/failed to persist/)).toBeTruthy(); // dropped = 1
  });

  it("monitor expansion drives kick with reason to the API (S6 UI)", async () => {
    setAdminJwt("tok");
    mockFetch();
    const { container } = ui();
    await screen.findByText("ops-01");
    fireEvent.click(container.querySelector(".ant-table-row-expand-icon") as HTMLElement);
    // two Kick buttons in the expanded rows (alice, bob) - target bob (index 1)
    // two Kick buttons in the expanded rows (alice, bob) - target bob (index 1).
    // querySelectorAll instead of ByRole: the accessible-name walk costs ~2.5s in jsdom.
    const kickBtns = () =>
      [...container.querySelectorAll("button")].filter((b) => b.textContent === "Kick");
    await waitFor(() => expect(kickBtns().length).toBe(2), { timeout: 3000 });
    fireEvent.click(kickBtns()[1]);
    await waitFor(() => expect(document.querySelector(".ant-modal")).toBeTruthy());
    const dialog = document.querySelector(".ant-modal") as HTMLElement;
    fireEvent.change(within(dialog).getByPlaceholderText(/Reason/), { target: { value: "abuse of open mic" } });
    await act(async () => {
      fireEvent.click(within(dialog).getByText("Kick"));
    });
    await waitFor(
      () => {
        const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock
          .calls as unknown as [string, { method?: string; body?: string }][];
        const kick = calls.find(
          ([u, i]) =>
            String(u).includes("/admin/api/rooms/ops-01/peers/bob") && i?.method === "DELETE",
        );
        expect(kick).toBeTruthy();
        const parsed = JSON.parse((kick?.[1]?.body ?? "{}") as string) as { reason?: string };
        expect(parsed.reason).toBe("abuse of open mic");
      },
      { timeout: 4000 },
    );
  });

  it("peer link opens the detail drawer with S6.2 fields", async () => {
    setAdminJwt("tok");
    mockFetch();
    const { container } = ui();
    await screen.findByText("ops-01");
    fireEvent.click(container.querySelector(".ant-table-row-expand-icon") as HTMLElement);
    // the muted Tag also says "bob" - the peer link is the only BUTTON named bob
    const peerLink = () =>
      [...container.querySelectorAll("button")].find((b) => b.textContent === "bob");
    await waitFor(() => expect(peerLink()).toBeTruthy(), { timeout: 3000 });
    fireEvent.click(peerLink() as HTMLElement);
    // AntD Descriptions labels render with a trailing colon - regex match
    expect(await screen.findByText(/moderator of/)).toBeTruthy();
    expect(screen.getByText(/holding floor/)).toBeTruthy();
  });

  it("rooms tab lists the registry and gates writes per tier (S2/S5 UI)", async () => {
    setAdminJwt("tok");
    mockFetch();
    ui();
    fireEvent.click(await screen.findByText("Rooms"));
    expect(await screen.findByText("squad")).toBeTruthy();
    expect(screen.getByText(/New room/i)).toBeTruthy();
    cleanup(); // isolate the remount (two live trees double every query)
    sessionStorage.setItem("ts-admin-role", "operator");
    ui();
    fireEvent.click(await screen.findByText("Rooms"));
    expect(await screen.findByText(/registry writes require admin/)).toBeTruthy();
    sessionStorage.removeItem("ts-admin-role");
  });

  it("rooms create posts the TYPED values, not the empty draft (S7.5 e2e caught this)", async () => {
    setAdminJwt("tok");
    mockFetch();
    ui();
    fireEvent.click(await screen.findByText("Rooms"));
    await screen.findByText("squad"); // registry loaded, tab live
    fireEvent.click(screen.getByText(/New room/i));
    const modal = document.querySelector(".ant-modal") as HTMLElement;
    expect(modal).toBeTruthy();
    fireEvent.change(within(modal).getByPlaceholderText("ops-01"), {
      target: { value: "night-ops" },
    });
    fireEvent.click(within(modal).getByRole("switch")); // Locked (refuse joins)
    await act(async () => {
      fireEvent.click(within(modal).getByText("Save"));
    });
    await waitFor(
      () => {
        const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock
          .calls as unknown as [string, { method?: string; body?: string }][];
        const post = calls.find(
          ([u, i]) => String(u) === "/admin/api/rooms" && i?.method === "POST",
        );
        expect(post).toBeTruthy();
        const row = JSON.parse(post?.[1]?.body ?? "{}") as {
          room?: string;
          locked?: boolean;
        };
        expect(row.room).toBe("night-ops");
        expect(row.locked).toBe(true);
      },
      { timeout: 4000 },
    );
  });
});

// entry-flow wave: Rooms-row Enter mints the operator token with an OMITTED sub —
// the server generates u-xxxxxx (D29-#4). Ratchet row #4 (testing.md rule 1): the
// issue call is a write path; the assert is on method+path+body (sub ABSENT).
import { EntryActions } from "../src/pages/admin/entryActions.js";

describe("rooms row entry actions", () => {
  it("Enter issues {room, role:dispatch} with NO sub and Copy links issues field", async () => {
    setAdminJwt("tok");
    const calls: [string, RequestInit | undefined][] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: unknown, init?: RequestInit) => {
        calls.push([String(url), init as RequestInit | undefined]);
        return {
          ok: true,
          status: 200,
          json: async () => ({
            sub: "u-generated",
            jwt: "j",
            field_url: "http://x/field/r?jwt=j",
            dispatch_url: "http://x/dispatch/r?jwt=j",
            participant_url: "http://x/room/r?jwt=j",
          }),
        } as Response;
      }) as unknown as typeof fetch,
    );
    render(
      <ConfigProvider theme={{ algorithm: theme.darkAlgorithm }}>
        <App>
          <EntryActions api={new AdminApi(() => {})} room="ops-01" />
        </App>
      </ConfigProvider>,
    );
    await act(async () => {
      fireEvent.click(screen.getByText("Enter"));
    });
    await waitFor(() => {
      const post = calls.find(([u, i]) => u === "/admin/api/tokens" && i?.method === "POST");
      expect(post).toBeTruthy();
      const req = JSON.parse((post?.[1]?.body as string) ?? "{}") as { sub?: string; room?: string; role?: string };
      expect(req.room).toBe("ops-01");
      expect(req.role).toBe("dispatch");
      expect("sub" in req).toBe(false); // OMITTED, not empty string — the D29 wire shape
    });
    await act(async () => {
      fireEvent.click(screen.getByText("Copy links"));
    });
    await waitFor(() => {
      const posts = calls.filter(([u, i]) => u === "/admin/api/tokens" && i?.method === "POST");
      expect(posts.length).toBe(2);
    });
  });
});

// entry-flow wave: JoinFailure tiers render from the status fields the client already
// classifies (signal.ts: auth-expired/4410) — no apply-to-join affordance (D29-#2=B).
import { JoinFailure } from "../src/components/joinFailure.js";

describe("join failure tiers", () => {
  const base = {
    connected: false,
    selfId: null,
    events: [],
    lastDenial: null,
    removedReason: null,
    expired: false,
  };
  it("expired tier: link-expired copy, no retry promise", () => {
    render(
      <ConfigProvider theme={{ algorithm: theme.darkAlgorithm }}>
        <JoinFailure status={{ ...base, expired: true }} />
      </ConfigProvider>,
    );
    expect(screen.getByTestId("join-expired")).toBeTruthy();
    expect(screen.getByText(/fresh join link/)).toBeTruthy();
  });
  it("removed tier: shows the operator's reason verbatim (R7)", () => {
    render(
      <ConfigProvider theme={{ algorithm: theme.darkAlgorithm }}>
        <JoinFailure status={{ ...base, removedReason: "noise on channel" }} />
      </ConfigProvider>,
    );
    expect(screen.getByTestId("join-removed")).toBeTruthy();
    expect(screen.getByText("noise on channel")).toBeTruthy();
  });
  it("no terminal state: renders nothing (reconnect flow owns the UI)", () => {
    const { container } = render(
      <ConfigProvider theme={{ algorithm: theme.darkAlgorithm }}>
        <JoinFailure status={base} />
      </ConfigProvider>,
    );
    expect(container.querySelector(".ant-alert")).toBeNull();
  });
});
