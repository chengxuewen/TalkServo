import { test, expect } from "@playwright/test";
import { spawnSync } from "node:child_process";
import path from "node:path";
const REPO = path.resolve(new URL("../..", import.meta.url).pathname);

test("diag J", async ({ browser }) => {
  const page = await browser.newPage();
  page.on("pageerror", (e) => console.log("PAGEERROR:", e.message));
  page.on("console", (m) => { if (m.type() === "error") console.log("CERR:", m.text()); });
  const tok = spawnSync("bash", [path.join(REPO, "scripts", "issue-token.sh"), "d1", "e2e-room", "field", "3600"],
    { encoding: "utf8", env: { ...process.env, TS_JWT_SECRET: process.env.TS_E2E_SECRET } }).stdout.trim();
  await page.goto(`${process.env.TS_URL}/f/e2e-room?jwt=${tok}`);
  await page.waitForTimeout(3000);
  console.log("TEXT:", JSON.stringify((await page.evaluate(() => document.body.innerText)).slice(0, 200)));
});
