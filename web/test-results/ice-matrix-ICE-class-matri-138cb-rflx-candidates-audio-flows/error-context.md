# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ice-matrix.spec.ts >> ICE class matrix (live + coturn, D17 LAN) >> #9 srflx: STUN via coturn yields srflx candidates, audio flows
- Location: e2e/ice-matrix.spec.ts:54:3

# Error details

```
Error: flow 0, types ["host","srflx","srflx","host","host","host","host","host","prflx"]

expect(received).toBeGreaterThan(expected)

Expected: > 2
Received:   0
```

# Test source

```ts
  1  | // Acceptance #9 — ICE candidate class matrix (D17: LAN range, self-hosted
  2  | // coturn). The host row is exercised by every other live test (direct UDP
  3  | // candidates); this spec covers srflx (STUN via coturn) and relay (TURN
  4  | // allocation, policy=relay). Requires TS_COTURN=1 (global-setup starts the
  5  | // container) + TS_E2E_FEATURES=live.
  6  | import { test, expect, type Page } from "@playwright/test";
  7  | import { LIVE, joinField, urlFor, audioLevel } from "./helpers.js";
  8  | 
  9  | const COTURN = !!process.env.TS_COTURN;
  10 | 
  11 | /** local-candidate types gathered by the holder's send transport. */
  12 | async function gatherTypes(page: Page): Promise<string[]> {
  13 |   return page.evaluate(async () => {
  14 |     const w = window as unknown as {
  15 |       __tsSendT?: { getStats(): Promise<RTCStatsReport> };
  16 |     };
  17 |     if (!w.__tsSendT) return [];
  18 |     const rep = await w.__tsSendT.getStats();
  19 |     const types: string[] = [];
  20 |     rep.forEach((r) => {
  21 |       if (r.type === "local-candidate") types.push(String(r.candidateType));
  22 |     });
  23 |     return types;
  24 |   });
  25 | }
  26 | 
  27 | /** one arm: listener first (caps settle), holder joins (optional url suffix),
  28 |  *  hold, sample flow, release. Returns gathered types + flow probe. */
  29 | async function arm(browser: import("@playwright/test").Browser, label: string, holderUrl = "") {
  30 |   const ctxL = await browser.newContext();
  31 |   const listener = await ctxL.newPage();
  32 |   await listener.goto(urlFor(`${label}-listener`) + holderUrl);
  33 |   await expect(listener.getByText("connected")).toBeVisible({ timeout: 15_000 });
  34 |   await listener.waitForTimeout(3000);
  35 |   const ctxH = await browser.newContext();
  36 |   const holder = await ctxH.newPage();
  37 |   await holder.goto(urlFor(`${label}-holder`) + holderUrl);
  38 |   await expect(holder.getByText("connected")).toBeVisible({ timeout: 15_000 });
  39 |   await holder.waitForTimeout(3000);
  40 | 
  41 |   await holder.keyboard.down("Space");
  42 |   await expect(holder.getByText(/HOLDING|SPEAKING/)).toBeVisible({ timeout: 15_000 });
  43 |   const flow = await audioLevel(listener, `${label}-holder`); // RTP growth probe
  44 |   const types = await gatherTypes(holder);
  45 |   await holder.keyboard.up("Space");
  46 |   await ctxH.close();
  47 |   await ctxL.close();
  48 |   return { flow, types };
  49 | }
  50 | 
  51 | test.describe("ICE class matrix (live + coturn, D17 LAN)", () => {
  52 |   test.skip(!LIVE || !COTURN, "requires TS_E2E_FEATURES=live + TS_COTURN=1");
  53 | 
  54 |   test("#9 srflx: STUN via coturn yields srflx candidates, audio flows", async ({ browser }) => {
  55 |     test.setTimeout(120_000);
  56 |     // default policy "all": host+srflx(+relay) gathered; assert srflx EXISTS
  57 |     // and the P-sequence completes (WebRTC then correctly prefers host on LAN)
  58 |     const { flow, types } = await arm(browser, "ix-srflx");
  59 |     expect(types, `gathered: ${JSON.stringify(types)}`).toContain("srflx");
> 60 |     expect(flow, `flow ${flow}, types ${JSON.stringify(types)}`).toBeGreaterThan(2);
     |                                                                  ^ Error: flow 0, types ["host","srflx","srflx","host","host","host","host","host","prflx"]
  61 |   });
  62 | 
  63 |   test("#9 relay: policy=relay routes the P-sequence through TURN", async ({ browser }) => {
  64 |     test.setTimeout(180_000);
  65 |     // ?ice=relay sets RTCPeerConnection iceTransportPolicy=relay on BOTH
  66 |     // transports (session.ts) — only relay candidates may exist; if TURN
  67 |     // allocation fails nothing flows and the flow probe times out
  68 |     const a = await arm(browser, "ix-relay", "&ice=relay");
  69 |     const b = await arm(browser, "ix-relay2", "&ice=relay");
  70 |     for (const r of [a, b]) {
  71 |       expect(r.types.length, "relay policy must still gather candidates").toBeGreaterThan(0);
  72 |       expect(r.types, `must be relay-only, got ${JSON.stringify(r.types)}`).not.toContain("host");
  73 |       expect(r.flow, `audio must flow over TURN relay (types ${JSON.stringify(r.types)})`).toBeGreaterThan(2);
  74 |     }
  75 |   });
  76 | });
  77 | 
```