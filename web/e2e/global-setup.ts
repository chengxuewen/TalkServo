// E2E harness: build + run the real server (stub-media + embedded-web) and
// mint tokens per peer. Env-gated: needs cargo + TS_JWT_SECRET capability.
import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const SECRET = `e2e-${Date.now()}`;
const ACCOUNTS_FILE = mkdtempSync(path.join(tmpdir(), "ts-e2e-accounts-")) + "/accounts.yaml";
const PORT = 8090;

let server;

const REPO = path.resolve(new URL("..", import.meta.url).pathname, "..");

export default async function () {
  // web build (fresh dist for rust-embed)
  spawnSync("npm", ["run", "build"], { cwd: path.join(REPO, "web"), stdio: "inherit" });
  // server binary (stub path — fastest, no worker). The pixi environment
  // carries the toolchain env (CC/LIBCLANG); plain `cargo` on PATH may be a
  // different build whose rust-embed folder resolution diverges.
  const PIXI = path.join(process.env.HOME ?? "", ".pixi", "bin", "pixi");
  // live lane: sfu-mediasoup + embedded-web (pages served by the same binary)
  const FEATURES =
    process.env.TS_E2E_FEATURES === "live"
      ? "embedded-web,sfu-mediasoup"
      : (process.env.TS_E2E_FEATURES ?? "embedded-web,stub-media");
  const build = spawnSync(
    PIXI,
    ["run", "cargo", "build", "-p", "talkservo-server", "--no-default-features", "--features", FEATURES],
    { cwd: REPO, stdio: "inherit" },
  );
  if (build.status !== 0) throw new Error(`cargo build failed: ${build.status}`);
  // #9 ICE matrix lane: host-network coturn via compose (idempotent up);
  // the server issues HMAC turn creds from the SAME secret; RTC ports move
  // to the mask-aligned range to avoid the coturn relay block (40000-40100)
  const coturn = !!process.env.TS_COTURN;
  if (coturn) {
    const up = spawnSync("docker", ["compose", "-f", path.join(REPO, "docker", "compose.yml"), "up", "-d", "coturn"],
      // the whole compose file interpolates even for one service — the
      // server service's required vars get dummy values (unused here)
      { env: { ...process.env, TURN_SECRET: "e2e-turn-secret", TS_JWT_SECRET: "compose-interpolation-only" }, stdio: "inherit" });
    if (up.status !== 0) throw new Error("coturn compose up failed");
  }
  // admin console lane (plan-5 S2): TS_ADMIN=1 enables the server's
  // TS_ADMIN_SECRET; specs mint the bootstrap token with the SAME constant
  const adminSecret = "e2e-admin-secret-fixed";
  if (process.env.TS_ADMIN) {
    process.env.TS_E2E_ADMIN_SECRET = adminSecret;
  }
  server = spawn(
    path.join(REPO, "target", "debug", "talkservo-server"),
    [],
    {
      env: {
        ...process.env,
        TS_JWT_SECRET: SECRET,
        TS_BIND: `127.0.0.1:${PORT}`,
        // writable accounts registry for the D22 user/pwd gate (spec creates
        // the first account via the script-minted bootstrap token)
        TS_ACCOUNTS_FILE: ACCOUNTS_FILE,
        ...(process.env.TS_ADMIN ? { TS_ADMIN_SECRET: adminSecret } : {}),
        // the live fake-device speaker beeps with ~400ms gaps — the E11
        // watchdog (2s grace) would down it mid-hold and make audibility/
        // latency probes phase-dependent. The e2e E11 row is a stub-synthetic
        // assertion (floor.spec) and the Rust live_e11 tests run their own
        // server — relaxing here affects neither. Production default stays 2s.
        ...(FEATURES.includes("sfu-mediasoup")
          ? { NO_RTP_WATCHDOG_MS: process.env.NO_RTP_WATCHDOG_MS ?? "600000" }
          : {}),
        ...(coturn
          ? {
              TURN_SECRET: "e2e-turn-secret",
              TURN_URI: "turn:127.0.0.1:3478?transport=udp",
              RTC_PORT_MIN: "40960",
              RTC_PORT_MAX: "41215",
            }
          : {}),
      },
      stdio: ["ignore", "inherit", "inherit"],
    },
  );
  // wait for /healthz (10s hard gate — a failed bind must fail the suite HERE)
  let healthy = false;
  for (let i = 0; i < 50; i++) {
    try {
      const res = await fetch(`http://127.0.0.1:${PORT}/healthz`);
      if (res.ok) { healthy = true; break; }
    } catch {}
    await new Promise((r) => setTimeout(r, 200));
  }
  if (!healthy) throw new Error(`server failed healthz on :${PORT}`);
  process.env.TS_URL = `http://127.0.0.1:${PORT}`;
  process.env.TS_E2E_SECRET = SECRET;
  return async () => {
    if (!process.env.TS_E2E_KEEP) server?.kill();
  };
}

export function issueToken(sub: string, room: string, role: "dispatch" | "field"): string {
  const out = spawnSync(
    "bash",
    [path.join(REPO, "scripts", "issue-token.sh"), sub, room, role, "3600"],
    {
      encoding: "utf8",
      env: { ...process.env, TS_JWT_SECRET: SECRET },
    },
  );
  if (out.status !== 0) throw new Error(`issue-token failed: ${out.stderr}`);
  return out.stdout.trim();
}
