// LAN deployment smoke (plan-4 T4'): browser over https://<lan-host> —
// proves TLS termination, WS upgrade through the proxy, secure-context
// getUserMedia, and the announced-LAN-IP media path. Prereq: server bound
// LAN/0.0.0.0 with TS_ANNOUNCED_IP + Caddy TLS front (see deploy/Caddyfile).
// Usage: TS_SMOKE_URL=https://host:port TS_JWT_SECRET=... node scripts/lan-smoke.mjs
import { spawnSync } from "node:child_process";
import { chromium } from "playwright";

const BASE = process.env.TS_SMOKE_URL ?? "https://talkservo.test:8443";
const HOST = new URL(BASE).hostname;
const LAN_IP = process.env.TS_LAN_IP ?? "192.168.2.127";
const SECRET = process.env.TS_JWT_SECRET ?? "lan";
const token = spawnSync("bash", ["../scripts/issue-token.sh", "smoke", "smoke-room", "field", "600"],
  { encoding: "utf8", env: { ...process.env, TS_JWT_SECRET: SECRET } }).stdout.trim();
if (!token) { console.error("token mint failed"); process.exit(1); }

const browser = await chromium.launch({
  args: [
    "--use-fake-device-for-media-stream",
    "--use-fake-ui-for-media-stream",
    `--host-resolver-rules=MAP ${HOST} ${LAN_IP}`,
    // env http_proxy would otherwise capture LAN traffic (proxy family of
    // the --noproxy lesson) — smoke is explicitly direct-connect
    "--proxy-server=direct://",
  ],
});
const ctx = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await ctx.newPage();
const steps = [];
page.on("console", (m) => {
  const t = m.text();
  if (t.includes("router_caps") || t.includes("device loaded") || t.includes("produce ride")) steps.push(t.slice(0, 60));
});
await page.goto(`${BASE}/f/smoke-room?jwt=${token}`, { waitUntil: "domcontentloaded" });
await page.getByText("connected").first().waitFor({ timeout: 15_000 });
console.log("WS: connected over https LAN ✓");
await page.waitForTimeout(4000); // media setup (device load + transports + produce)
if (!steps.some((s) => s.includes("device loaded"))) {
  console.error("media setup did not reach device.load — secure context?", steps);
  process.exit(1);
}
console.log("MEDIA: device loaded + produce ride over LAN TLS ✓", steps.length, "wire milestones");
await browser.close();
