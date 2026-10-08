// Shared e2e helpers (plan-4): token minting, field-page join, audibility
// probe. Split out of media.spec.ts because Playwright forbids spec→spec
// imports; both live specs consume this module.
import { test, expect, type Page } from "@playwright/test";
import { spawnSync } from "node:child_process";
import path from "node:path";

const REPO = path.resolve(new URL("../..", import.meta.url).pathname);
export const ROOM = "media-room";
export const LIVE = process.env.TS_E2E_FEATURES === "live";

export function urlFor(sub: string): string {
  const out = spawnSync(
    "bash",
    [path.join(REPO, "scripts", "issue-token.sh"), sub, ROOM, "field", "3600"],
    { encoding: "utf8", env: { ...process.env, TS_JWT_SECRET: process.env.TS_E2E_SECRET } },
  );
  return `${process.env.TS_URL!}/f/${ROOM}?jwt=${out.stdout.trim()}`;
}

export async function joinField(page: Page, sub: string) {
  await page.goto(urlFor(sub));
  await expect(page.getByText("connected")).toBeVisible({ timeout: 15_000 });
}

/** Audibility probe: remote track unmuted + element clock advancing.
 *  (Analyser RMS measures 0 in headless chromium even while the element
 *  plays — WebAudio/remote-stream quirk; the element path is the truth.) */
export async function audioLevel(page: Page, peer: string): Promise<number> {
  // wait for the consumer track to land (announcement -> ConsumeOk ->
  // element) before probing — HOLDING appears fast, the consumer chain lags
  await page
    .waitForFunction(
      (p) => !!document.querySelector(`#ts-audio-pool audio[data-peer="${p}"]`),
      peer,
      { timeout: 25_000 },
    )
    .catch(() => {});
  return page.evaluate(async (peerId) => {
    const w = window as unknown as { __tsRecvT?: { getStats(): Promise<RTCStatsReport> } };
    if (!w.__tsRecvT) return -1;
    const snap = async () => {
      let bytes = 0;
      (await w.__tsRecvT.getStats()).forEach((r) => {
        if (r.type === "inbound-rtp" && r.kind === "audio") bytes = Number(r.bytesReceived ?? 0);
      });
      return bytes;
    };
    // decode/playout is a headless-output concern; the wire truth is RTP
    // bytes flowing on the recv transport. NOTE: the E11 watchdog (2s grace)
    // downs the fake-device speaker during its beep gaps — sample EARLY in
    // the hold, not at leisure.
    const b0 = await snap();
    for (let n = 0; n < 25; n++) {
      await new Promise((r) => setTimeout(r, 800));
      const b1 = await snap();
      if (b1 > b0) return 100;
    }
    (window as unknown as { __tsClockDbg?: string }).__tsClockDbg = `bytes stuck b0=${b0}`;
    return 0;
  }, peer);
}
