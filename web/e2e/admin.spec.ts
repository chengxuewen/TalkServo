// Plan-5 S2: admin console end-to-end on the stub lane — gate auth, live
// monitor row, token issuance, and the click-through of an issued link
// (proof the whole loop works through the REAL page, no test-only hooks).
import { test, expect } from "@playwright/test";
import { spawnSync } from "node:child_process";
import path from "node:path";

const REPO = path.resolve(new URL("../..", import.meta.url).pathname);
const ROOM = "admin-room";

function userToken(sub: string): string {
  const out = spawnSync(
    "bash",
    [path.join(REPO, "scripts", "issue-token.sh"), sub, ROOM, "field", "3600"],
    { encoding: "utf8", env: { ...process.env, TS_JWT_SECRET: process.env.TS_E2E_SECRET ?? "" } },
  );
  return out.stdout.trim();
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

  test("gate -> monitor -> issue -> click-through", async ({ browser }) => {
    test.setTimeout(90_000);
    // a field peer first, so Monitor has something to show
    const ctxF = await browser.newContext();
    const field = await ctxF.newPage();
    await field.goto(`${process.env.TS_URL}/f/${ROOM}?jwt=${userToken("probe-1")}`);
    await expect(field.getByText("connected")).toBeVisible({ timeout: 15_000 });

    const admin = await (await browser.newContext()).newPage();
    await admin.goto(`${process.env.TS_URL}/a`);
    await admin.getByPlaceholder("Admin JWT").fill(adminToken());
    await admin.getByRole("button", { name: "Enter" }).click();

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
    expect(field_url).toContain(`/f/${ROOM}?jwt=${jwt}`);

    const click = await (await browser.newContext()).newPage();
    await click.goto(`${process.env.TS_URL}/f/${ROOM}?jwt=${jwt}`);
    await expect(click.getByText("connected")).toBeVisible({ timeout: 15_000 });

    // negative: a USER token at the admin gate bounces (audience isolation)
    const neg = await (await browser.newContext()).newPage();
    await neg.goto(`${process.env.TS_URL}/a`);
    await neg.getByPlaceholder("Admin JWT").fill(userToken("nope"));
    await neg.getByRole("button", { name: "Enter" }).click();
    await expect(neg.getByText(/rejected/i)).toBeVisible({ timeout: 10_000 });

    await ctxF.close();
  });
});
