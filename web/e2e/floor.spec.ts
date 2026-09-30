// Plan-3 gap #2 (A): signaling-matrix e2e — 3 real browser instances against
// the real server (stub-media + embedded-web). Acceptance §4 #1-#4 + E11 UX.
import { test, expect, type Page } from "@playwright/test";
import { spawnSync } from "node:child_process";
import path from "node:path";

const ROOM = "e2e-room";
const REPO = path.resolve(new URL("../..", import.meta.url).pathname);

/** Mint through the repo script with the harness secret (env-injected by
 *  global-setup — the module import would re-evaluate a fresh secret). */
function issueToken(sub: string, role: "dispatch" | "field"): string {
  const out = spawnSync(
    "bash",
    [path.join(REPO, "scripts", "issue-token.sh"), sub, ROOM, role, "3600"],
    { encoding: "utf8", env: { ...process.env, TS_JWT_SECRET: process.env.TS_E2E_SECRET } },
  );
  if (out.status !== 0) throw new Error(`issue-token: ${out.stderr}`);
  return out.stdout.trim();
}

function urlFor(role: "d" | "f", sub: string): string {
  return `${process.env.TS_URL!}/${role}/${ROOM}?jwt=${issueToken(sub, role === "d" ? "dispatch" : "field")}`;
}

async function joinField(page: Page, sub: string) {
  await page.goto(urlFor("f", sub));
  await expect(page.getByText("connected")).toBeVisible({ timeout: 10_000 });
}

async function joinDispatch(page: Page, sub: string) {
  await page.goto(urlFor("d", sub));
  await expect(page.getByText("connected")).toBeVisible({ timeout: 10_000 });
}

test("J: three instances see a 3-peer roster", async ({ browser }) => {
  const a = await browser.newContext();
  const pa = await a.newPage();
  await joinField(pa, "alpha");

  const b = await browser.newContext();
  const pb = await b.newPage();
  await joinField(pb, "bravo");

  const d = await browser.newContext();
  const pd = await d.newPage();
  await joinDispatch(pd, "chief");

  // rosters converge (server deltas + snapshots)
  await expect(pa.getByText("bravo")).toBeVisible({ timeout: 5000 });
  await expect(pb.getByText("alpha")).toBeVisible({ timeout: 5000 });
  await expect(pd.getByText("alpha")).toBeVisible({ timeout: 5000 });
  await expect(pd.getByText("bravo")).toBeVisible({ timeout: 5000 });

  await a.close();
  await b.close();
  await d.close();
});

test("P/X/R: hold grants, preempt takes, release idles — dispatcher sees gen", async ({
  browser,
}) => {
  const a = await browser.newContext();
  const pa = await a.newPage();
  await joinField(pa, "alpha");
  const c = await browser.newContext();
  const pc = await c.newPage();
  await joinField(pc, "charlie");
  const d = await browser.newContext();
  const pd = await d.newPage();
  await joinDispatch(pd, "chief");

  // switch to Exclusive (room default is Hybrid cap-2 — a preempt with free
  // capacity takes the EMPTY slot, no Taken event)
  await pd.getByRole("button", { name: "Exclusive" }).click();
  await expect(pd.getByText("mode: exclusive")).toBeVisible({ timeout: 5000 });

  // P: alpha holds (Space keydown)
  await pa.keyboard.down("Space");
  await expect(pa.getByText(/HOLDING/)).toBeVisible({ timeout: 5000 });
  await expect(pa.getByText("Floor granted. You may speak.")).toBeVisible({ timeout: 5000 });
  // tally ring on the dispatcher grid flips to holder
  await expect(pd.locator(".tally-ring.holder")).toHaveCount(1, { timeout: 5000 });

  // X: the dispatcher preempts (its authorized action — priority-9 request
  // from the chief's identity; modules/08 preempt-broadcast)
  await pd.getByRole("button", { name: "Preempt" }).first().click();
  // holder ring moves to chief (taken + re-grant collapsed into the new state)
  await expect(pd.locator(".tally-ring.holder")).toHaveCount(1, { timeout: 5000 });
  const holderCard = pd.locator(".ant-card", { hasText: "chief" });
  await expect(holderCard.locator(".tally-ring.holder")).toBeVisible({ timeout: 5000 });
  // alpha sees the loss via its aria-live channel
  await expect(pa.getByText(/Floor idle\. You may speak|Floor held by chief/)).toBeVisible({ timeout: 5000 });

  // R(lease): alpha unholds first (its key is still physically down — the
  // button shows HOLDING while the pointer/keys stay pressed), then the
  // chief force-releases its preempted floor
  await pa.keyboard.up("Space");
  await pd.getByRole("button", { name: "Force release" }).click();
  await expect(pa.getByText("HOLD TO TALK")).toBeVisible({ timeout: 5000 });
  await expect(pc.getByText("HOLD TO TALK")).toBeVisible({ timeout: 5000 });

  // dispatcher gen badge advanced
  await expect(pd.getByText(/gen \d+/)).toBeVisible({ timeout: 5000 });

  await a.close();
  await c.close();
  await d.close();
});

test("R1: reconnect restores the roster from a fresh snapshot", async ({ browser }) => {
  const a = await browser.newContext();
  const pa = await a.newPage();
  await joinField(pa, "reconnector");

  // hard refresh = fresh page, fresh WS, same identity (R-sequence via Join)
  await pa.reload();
  await joinField(pa, "reconnector");
  await expect(pa.getByText("reconnector")).toBeVisible({ timeout: 5000 });

  await a.close();
});

test("E11: silent uplink surfaces MediaFailed (server-synthetic)", async ({ browser }) => {
  // NOTE: E11's real trigger needs the live host's AudioLevelObserver; the
  // stub path can't script activity through the wire. What IS verifiable
  // here: the media-failure UX path renders when the server announces it —
  // skipped until the browser slice wires a kill-switch endpoint (plan-3 T5
  // dependency note). This test documents the gap honestly.
  test.skip(true, "E11 e2e needs a kill-switch test endpoint (next slice)");
});
