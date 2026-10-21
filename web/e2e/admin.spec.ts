// Plan-5 S2: admin console end-to-end on the stub lane — gate auth, live
// monitor row, token issuance, and the click-through of an issued link
// (proof the whole loop works through the REAL page, no test-only hooks).
import { test, expect, type Browser, type APIRequestContext, type Page } from "@playwright/test";
import { spawnSync } from "node:child_process";
import path from "node:path";

const REPO = path.resolve(new URL("../..", import.meta.url).pathname);
const ROOM = "admin-room";

function userToken(sub: string, room = ROOM): string {
  const out = spawnSync(
    "bash",
    [path.join(REPO, "scripts", "issue-token.sh"), sub, room, "field", "3600"],
    { encoding: "utf8", env: { ...process.env, TS_JWT_SECRET: process.env.TS_E2E_SECRET } },
  );
  return out.stdout.trim();
}

const BASE = () => process.env.TS_URL!;
const OPS = "e2e-ops";
const OPS_PASS = "e2e-pass-123";

/** Idempotent account upsert (the bootstrap token is the CLI-side path) + the REAL
 * UI login, so every row starts from a console the same way an operator does. */
async function adminLoggedIn(browser: Browser, request: APIRequestContext): Promise<Page> {
  const boot = await request.post(`${BASE()}/admin/api/accounts`, {
    headers: { authorization: `Bearer ${adminToken()}` },
    data: { user: OPS, password: OPS_PASS },
  });
  expect(boot.status(), "account upsert via registry").toBeLessThan(300);
  const page = await (await browser.newContext()).newPage();
  await page.goto(`${BASE()}/admin`);
  await page.getByPlaceholder("Username").fill(OPS);
  await page.getByPlaceholder("Password").fill(OPS_PASS);
  await page.getByRole("button", { name: "Log in" }).click();
  await expect(page.getByRole("button", { name: "Monitor" })).toBeVisible({
    timeout: 15_000,
  });
  return page;
}

/** Join a room as a field peer and wait for the wire to say so. `frames`
 *  optionally collects every inbound WS payload, which is how PIT-26's
 *  "did the frame never arrive, or arrive and get discarded?" gets answered. */
async function joinField(
  browser: Browser,
  sub: string,
  room: string,
  frames?: string[],
): Promise<Page> {
  const page = await (await browser.newContext()).newPage();
  if (frames) {
    page.on("websocket", (ws) =>
      ws.on("framereceived", (f) => frames.push(String(f.payload))),
    );
  }
  await page.goto(`${BASE()}/field/${room}?jwt=${userToken(sub, room)}`);
  await expect(page.getByText("connected")).toBeVisible({ timeout: 15_000 });
  return page;
}

/** PIT-26 capture: the mirror the page actually holds, for failure messages. */
async function mirrorDump(page: Page, room: string): Promise<string> {
  return page.evaluate((key) => {
    const w = window as unknown as {
      __tsSessions?: Map<
        string,
        { mirror: { peers: { id: string }[]; generation: number; muted: string[] } }
      >;
    };
    const s = w.__tsSessions?.get(key);
    return s
      ? JSON.stringify({
          peers: s.mirror.peers.map((p) => p.id),
          generation: s.mirror.generation,
          muted: s.mirror.muted,
        })
      : "no session";
  }, `field:${room}`);
}

function adminToken(): string {
  const out = spawnSync(
    "bash",
    [path.join(REPO, "scripts", "issue-admin-token.sh"), "e2e-root", "3600"],
    {
      encoding: "utf8",
      env: { ...process.env, TS_ADMIN_SECRET: process.env.TS_E2E_ADMIN_SECRET ?? "" },
    },
  );
  return out.stdout.trim();
}

test.describe("admin console (stub lane)", () => {
  test.skip(!process.env.TS_ADMIN, "set TS_ADMIN=1 (enables server TS_ADMIN_SECRET)");

  test("gate -> monitor -> issue -> click-through", async ({ browser, request }) => {
    test.setTimeout(90_000);
    // bootstrap: the script-minted token creates the first human account
    // (the real operator path), then the UI only ever sees user/pwd.
    // bootstrap: the script-minted token creates the first human account
    // (the real operator path), then the UI only ever sees user/pwd.
    const boot = await request.post(`${process.env.TS_URL}/admin/api/accounts`, {
      headers: { authorization: `Bearer ${adminToken()}` },
      data: { user: "e2e-ops", password: "e2e-pass-123" },
    });
    expect(boot.status(), "account created via registry").toBe(201);
    // a field peer first, so Monitor has something to show
    const ctxF = await browser.newContext();
    const field = await ctxF.newPage();
    await field.goto(`${process.env.TS_URL}/field/${ROOM}?jwt=${userToken("probe-1")}`);
    await expect(field.getByText("connected")).toBeVisible({ timeout: 15_000 });

    const admin = await (await browser.newContext()).newPage();
    await admin.goto(`${process.env.TS_URL}/admin`);
    await admin.getByPlaceholder("Username").fill("e2e-ops");
    await admin.getByPlaceholder("Password").fill("e2e-pass-123");
    await admin.getByRole("button", { name: "Log in" }).click();

    // Monitor: room row carries the peer
    await expect(admin.getByText(ROOM)).toBeVisible({ timeout: 10_000 });
    await expect(admin.getByText("Peers 1")).toBeVisible({ timeout: 10_000 });

    // Tokens: issue and capture the response (link truth, not the truncated
    // display), then click through in a fresh context.
    const respP = admin.waitForResponse((r) => r.url().endsWith("/admin/api/tokens"));
    await admin.getByRole("button", { name: "Tokens" }).click();
    await admin.getByPlaceholder("field-7").fill("click-dev");
    await admin.getByPlaceholder("ops-01").fill(ROOM);
    await admin.getByRole("button", { name: "Issue" }).click();
    const issued = (await respP).json() as Promise<{ jwt: string; field_url: string }>;
    const { jwt, field_url } = await issued;
    expect(field_url).toContain(`/field/${ROOM}?jwt=${jwt}`);

    const click = await (await browser.newContext()).newPage();
    await click.goto(`${process.env.TS_URL}/field/${ROOM}?jwt=${jwt}`);
    await expect(click.getByText("connected")).toBeVisible({ timeout: 15_000 });

    // negatives: wrong password bounces in the UI; a USER token at the admin
    // API bounces too (audience isolation, API-level since no paste exists)
    const neg = await (await browser.newContext()).newPage();
    await neg.goto(`${process.env.TS_URL}/admin`);
    await neg.getByPlaceholder("Username").fill("e2e-ops");
    await neg.getByPlaceholder("Password").fill("totally-wrong");
    await neg.getByRole("button", { name: "Log in" }).click();
    await expect(neg.getByText(/invalid username or password/i)).toBeVisible({ timeout: 10_000 });
    const bounce = await request.get(`${process.env.TS_URL}/admin/api/overview`, {
      headers: { authorization: `Bearer ${userToken("nope")}` },
    });
    expect(bounce.status(), "user token never authenticates admin API").toBe(401);

    await ctxF.close();
  });

  /** Expand the room row in Monitor and kick one peer through the confirm Modal. */
  async function kickFromMonitor(admin: Page, room: string, peer: string, reason: string) {
    await expect(admin.getByText(room)).toBeVisible({ timeout: 20_000 });
    await admin
      .locator("tr.ant-table-row", { hasText: room })
      .first()
      .locator(".ant-table-row-expand-icon")
      .click();
    // the nested peer table renders inside its own tr.ant-table-row; the room row's
    // own cells never carry the peer id, so this is the peer row, not the parent
    const row = admin.locator("tr.ant-table-row").filter({ hasText: peer }).last();
    await row.getByRole("button", { name: "Kick" }).click();
    const modal = admin.locator(".ant-modal");
    await modal.getByPlaceholder(/Reason/).fill(reason);
    const del = admin.waitForResponse(
      (r) => r.url().includes(`/peers/${peer}`) && r.request().method() === "DELETE",
    );
    await modal.getByRole("button", { name: "Kick" }).click();
    expect((await del).status(), `kick ${peer} answers 204`).toBe(204);
    await expect(modal).toBeHidden({ timeout: 10_000 });
  }

  test("cross-client: a console kick clears the peer from ANOTHER peer's page (S7.4)", async ({
    browser,
    request,
  }) => {
    test.setTimeout(120_000);
    const room = "kick-room";
    const frames: string[] = [];
    const witness = await joinField(browser, "witness", room, frames);
    const victim = await joinField(browser, "victim", room);
    await expect(witness.getByText("victim")).toBeVisible({ timeout: 10_000 });

    const admin = await adminLoggedIn(browser, request);
    await kickFromMonitor(admin, room, "victim", "cross-client kick reason");

    // the point of this row: a page nobody touched learns the departure (PIT-21 rule
    // - assert the OTHER peers' mirrors, never only the acting client's own screen)
    try {
      await expect(witness.getByText("victim")).toBeHidden({ timeout: 10_000 });
    } catch {
      // PIT-26: this row has now failed twice under `ctest -L e2e-stub` and never
      // under a direct `npx playwright test`. Fail with the evidence that
      // distinguishes the two hypotheses instead of with a screenshot.
      const about = frames.filter((f) => f.includes("victim"));
      throw new Error(
        "victim still on the witness page. " +
          `mirror=${await mirrorDump(witness, room)}; ` +
          `frames-mentioning-victim=${about.length ? JSON.stringify(about.slice(-4)) : "NONE"}; ` +
          `total-inbound-frames=${frames.length}`,
      );
    }
    // and the kicked peer is TOLD, in words it cannot mistake for bad weather
    // (R7-b: the removal is terminal, so no "waiting for connection" promise)
    // three surfaces say it (tag, button, aria-live), so scope by role/element type
    // rather than praying the text is unique (PIT-24: no getByRole in interaction
    // paths; this is an assertion, and it is scoped to the button on purpose)
    await expect(
      victim.locator("button", { hasText: "REMOVED BY DISPATCHER" }),
    ).toBeVisible({ timeout: 15_000 });
    await expect(victim.getByTestId("floor-state")).toContainText(
      "kicked by the dispatcher",
    );
    await expect(victim.getByText("Removed by dispatcher", { exact: true })).toBeVisible();
  });

  test("audit: the console action lands as an evidence line in the Logs tab (S7.4)", async ({
    browser,
    request,
  }) => {
    test.setTimeout(120_000);
    const room = "audit-room";
    await joinField(browser, "noisy", room);

    const admin = await adminLoggedIn(browser, request);
    await kickFromMonitor(admin, room, "noisy", "audit evidence reason");

    // Logs is deliberately manual-refresh (an evidence surface, not telemetry), so
    // the refresh click is part of what this row proves
    await admin.getByRole("button", { name: "Logs" }).click();
    await admin.getByPlaceholder("room").fill(room);
    await admin.getByRole("button", { name: "Refresh" }).click();
    // one line per action; scope to the kick row, because the room filter keeps the
    // join/leave lines for the SAME peer on screen and strict mode would match them all
    const kickRow = admin.locator("tr.ant-table-row", { hasText: "kick" }).first();
    await expect(kickRow.getByText("audit evidence reason")).toBeVisible({
      timeout: 10_000,
    });
    await expect(kickRow.getByText("noisy", { exact: true })).toBeVisible(); // target
    await expect(kickRow.getByText(OPS)).toBeVisible(); // actor = the console account
    // R4 the other way round: a healthy writer must NOT warn about lost lines
    await expect(admin.getByText(/failed to persist/)).toBeHidden();
  });

  test("locked room: the registry lock refuses a valid token, visibly (S7.4)", async ({
    browser,
    request,
  }) => {
    test.setTimeout(120_000);
    const room = "locked-room";
    const admin = await adminLoggedIn(browser, request);
    await admin.getByRole("button", { name: "Rooms" }).click();
    await admin.getByRole("button", { name: /New room/i }).click();
    const modal = admin.locator(".ant-modal");
    await modal.getByPlaceholder("ops-01").fill(room);
    await modal.getByRole("switch").click(); // "Locked (refuse joins)"
    await modal.getByRole("button", { name: "Save" }).click();
    await expect(admin.getByText("🔒")).toBeVisible({ timeout: 10_000 });

    // a structurally valid token for that room now cannot get in: hot registry read,
    // no restart, no new wire variant (Error.code carries it)
    const joiner = await (await browser.newContext()).newPage();
    const frames: string[] = [];
    joiner.on("websocket", (ws) =>
      ws.on("framereceived", (f) => frames.push(String(f.payload))),
    );
    await joiner.goto(`${BASE()}/field/${room}?jwt=${userToken("late-joiner", room)}`);
    await expect(joiner.getByText("connected")).toBeHidden();
    await expect
      .poll(() => frames.some((f) => f.includes("room_locked")), {
        timeout: 15_000,
        message: "the server never answered room_locked on the wire",
      })
      .toBe(true);
  });
});
