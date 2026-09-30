// E2E harness: build + run the real server (stub-media + embedded-web) and
// mint tokens per peer. Env-gated: needs cargo + TS_JWT_SECRET capability.
import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const SECRET = `e2e-${Date.now()}`;
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
  const build = spawnSync(
    PIXI,
    ["run", "cargo", "build", "-p", "talkservo-server", "--no-default-features", "--features", "embedded-web,stub-media"],
    { cwd: REPO, stdio: "inherit" },
  );
  if (build.status !== 0) throw new Error(`cargo build failed: ${build.status}`);
  server = spawn(
    path.join(REPO, "target", "debug", "talkservo-server"),
    [],
    {
      env: {
        ...process.env,
        TS_JWT_SECRET: SECRET,
        TS_BIND: `127.0.0.1:${PORT}`,
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
    server?.kill();
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
