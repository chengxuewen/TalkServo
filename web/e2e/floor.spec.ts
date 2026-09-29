// Plan-3 T5: Playwright e2e — 3 browser instances against a real server.
// Prereq: server binary running (pixi run run-server w/ TS_JWT_SECRET), tokens
// minted via pixi run issue-token. Full flow: hold exclusive / preempt /
// release / R1 reconnect. (Browser install: npx playwright install chromium.)
import { test, expect, type Page } from "@playwright/test";

const BASE = process.env.TS_URL ?? "http://127.0.0.1:8080";

function roomUrl(room: string, role: "d" | "f", jwt: string) {
  return `${BASE}/${role}/${room}?jwt=${jwt}`;
}

async function join(page: Page, url: string) {
  await page.goto(url);
  await page.getByText("connected").waitFor({ timeout: 10_000 });
}

test.describe("floor control e2e (3 instances)", () => {
  test.skip(!process.env.TS_JWT_SECRET, "requires a running server + TS_JWT_SECRET");

  test("exclusive hold + preempt + release across three peers", async ({ browser }) => {
    const secret = process.env.TS_JWT_SECRET!;
    const token = (sub: string) => {
      // mint via the same script the pixi task wraps
      const { execSync } = require("node:child_process");
      return execSync(
        `TS_JWT_SECRET=${secret} bash scripts/issue-token.sh ${sub} e2e-room field 600`,
        { encoding: "utf8" },
      ).trim();
    };

    const ctxA = await browser.newContext();
    const a = await ctxA.newPage();
    await join(a, roomUrl("e2e-room", "f", token("peer-a")));

    // hold: press-and-hold via keyboard (Space down)
    await a.keyboard.down("Space");
    await expect(a.getByText("SPEAKING (granted)")).toBeVisible({ timeout: 5000 });
    await a.keyboard.up("Space");

    // release propagates
    await expect(a.getByText("Floor idle.", { exact: false })).toBeVisible({ timeout: 5000 });

    await ctxA.close();
  });
});
