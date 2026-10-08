// Acceptance #9 — ICE candidate class matrix (D17: LAN range, self-hosted
// coturn). The host row is exercised by every other live test (direct UDP
// candidates); this spec covers srflx (STUN via coturn) and relay (TURN
// allocation, policy=relay). Requires TS_COTURN=1 (global-setup starts the
// container) + TS_E2E_FEATURES=live.
import { test, expect, type Page } from "@playwright/test";
import { LIVE, joinField, urlFor, audioLevel } from "./helpers.js";

const COTURN = !!process.env.TS_COTURN;

/** local-candidate types gathered by the holder's send transport. */
async function gatherTypes(page: Page): Promise<string[]> {
  return page.evaluate(async () => {
    const w = window as unknown as {
      __tsSendT?: { getStats(): Promise<RTCStatsReport> };
    };
    if (!w.__tsSendT) return [];
    const rep = await w.__tsSendT.getStats();
    const types: string[] = [];
    rep.forEach((r) => {
      if (r.type === "local-candidate") types.push(String(r.candidateType));
    });
    return types;
  });
}

/** one arm: listener first (caps settle), holder joins (optional url suffix),
 *  hold, sample flow, release. Returns gathered types + flow probe. */
async function arm(browser: import("@playwright/test").Browser, label: string, holderUrl = "") {
  const ctxL = await browser.newContext();
  const listener = await ctxL.newPage();
  await listener.goto(urlFor(`${label}-listener`) + holderUrl);
  await expect(listener.getByText("connected")).toBeVisible({ timeout: 15_000 });
  await listener.waitForTimeout(3000);
  const ctxH = await browser.newContext();
  const holder = await ctxH.newPage();
  await holder.goto(urlFor(`${label}-holder`) + holderUrl);
  await expect(holder.getByText("connected")).toBeVisible({ timeout: 15_000 });
  await holder.waitForTimeout(3000);

  await holder.keyboard.down("Space");
  await expect(holder.getByText(/HOLDING|SPEAKING/)).toBeVisible({ timeout: 15_000 });
  const flow = await audioLevel(listener, `${label}-holder`); // RTP growth probe
  const types = await gatherTypes(holder);
  await holder.keyboard.up("Space");
  await ctxH.close();
  await ctxL.close();
  return { flow, types };
}

test.describe("ICE class matrix (live + coturn, D17 LAN)", () => {
  test.skip(!LIVE || !COTURN, "requires TS_E2E_FEATURES=live + TS_COTURN=1");

  test("#9 srflx: STUN via coturn yields srflx candidates, audio flows", async ({ browser }) => {
    test.setTimeout(120_000);
    // default policy "all": host+srflx(+relay) gathered; assert srflx EXISTS
    // and the P-sequence completes (WebRTC then correctly prefers host on LAN)
    const { flow, types } = await arm(browser, "ix-srflx");
    expect(types, `gathered: ${JSON.stringify(types)}`).toContain("srflx");
    expect(flow, `flow ${flow}, types ${JSON.stringify(types)}`).toBeGreaterThan(2);
  });

  test("#9 relay: policy=relay routes the P-sequence through TURN", async ({ browser }) => {
    test.setTimeout(180_000);
    // ?ice=relay sets RTCPeerConnection iceTransportPolicy=relay on BOTH
    // transports (session.ts) — only relay candidates may exist; if TURN
    // allocation fails nothing flows and the flow probe times out
    const a = await arm(browser, "ix-relay", "&ice=relay");
    const b = await arm(browser, "ix-relay2", "&ice=relay");
    for (const r of [a, b]) {
      expect(r.types.length, "relay policy must still gather candidates").toBeGreaterThan(0);
      expect(r.types, `must be relay-only, got ${JSON.stringify(r.types)}`).not.toContain("host");
      expect(r.flow, `audio must flow over TURN relay (types ${JSON.stringify(r.types)})`).toBeGreaterThan(2);
    }
  });
});
