// P0 multi-web concurrent talk — modules/08 §1 / D1 grant-set semantics.
// Dispatcher flips the room Open; three web clients take the floor at the
// same time and ALL stay ON AIR (grants size 3). Subject is the floor state
// machine over real WS signaling + embedded-web; the media-plane concurrency
// on the real SFU rides the live lane (same pages, fake audio devices).
import { test, expect, type Page } from "@playwright/test";
import { spawnSync } from "node:child_process";
import path from "node:path";

const ROOM = "talk-room";
const REPO = path.resolve(new URL("../..", import.meta.url).pathname);

function issueToken(sub: string, role: "dispatch" | "field"): string {
  const out = spawnSync(
    "bash",
    [path.join(REPO, "scripts", "issue-token.sh"), sub, ROOM, role, "3600"],
    { encoding: "utf8", env: { ...process.env, TS_JWT_SECRET: process.env.TS_E2E_SECRET } },
  );
  if (out.status !== 0) throw new Error(`issue-token: ${out.stderr}`);
  return out.stdout.trim();
}

/** path: "dispatch" | "field" | "room" (participant shell rides the field token) */
function urlFor(pathName: "dispatch" | "field" | "room", sub: string): string {
  const role = pathName === "dispatch" ? "dispatch" : "field";
  return `${process.env.TS_URL!}/${pathName}/${ROOM}?jwt=${issueToken(sub, role)}`;
}

async function joinParticipant(page: Page, sub: string) {
  await page.goto(urlFor("room", sub));
  await expect(page.getByText("connected")).toBeVisible({ timeout: 10_000 });
}

test("Open mode: three web clients speak at once", async ({ browser }) => {
  // 1. dispatcher flips the room from Hybrid default to Open
  const d = await (await browser.newContext()).newPage();
  await d.goto(urlFor("dispatch", "chief"));
  await expect(d.getByText("connected")).toBeVisible({ timeout: 10_000 });
  await d.getByRole("button", { name: "Open" }).click();
  await expect(d.getByText("mode: open")).toBeVisible();

  // 2. two participant shells + one field hold-to-talk, taken concurrently
  // (the mode change from step 1 must have PROPAGATED to these non-actor
  // mirrors — PIT-21 rule: wire-state assertions observe other peers' pages)
  const p1 = await (await browser.newContext()).newPage();
  await joinParticipant(p1, "alpha");
  await expect(p1.getByText("mode: open")).toBeVisible({ timeout: 10_000 });   // propagated
  await p1.getByRole("button", { name: "JOIN THE CONVERSATION" }).click();
  await expect(p1.getByText("ON AIR — click to mute")).toBeVisible({ timeout: 5_000 });

  const p2 = await (await browser.newContext()).newPage();
  await joinParticipant(p2, "bravo");
  await p2.getByRole("button", { name: "JOIN THE CONVERSATION" }).click();
  await expect(p2.getByText("ON AIR — click to mute")).toBeVisible({ timeout: 5_000 });

  const f3 = await (await browser.newContext()).newPage();
  await f3.goto(urlFor("field", "charlie"));
  await expect(f3.getByText("connected")).toBeVisible({ timeout: 10_000 });
  await f3.keyboard.down("Space"); // hold-to-talk via the default keybind
  await expect(f3.getByText("Floor granted. You may speak.")).toBeVisible({ timeout: 5_000 });

  // 3. CONCURRENCY: all three still granted simultaneously (Hybrid would have
  // capped at 2; Open grants every request)
  await expect(p1.getByText("ON AIR — click to mute")).toBeVisible();
  await expect(p2.getByText("ON AIR — click to mute")).toBeVisible();
  await expect(f3.getByText(/HOLDING/)).toBeVisible();
  await expect(p1.getByText("bravo")).toBeVisible(); // mutual rosters
  await expect(p1.getByText("Speakers (3)")).toBeVisible(); // all three granted at once

  // 4. one yields — the others are unaffected (no cascade idle under Open);
  // the yield is asserted on a NON-ACTOR mirror too (PIT-21 rule: grant-set
  // assertions must observe other peers' mirrors, not the acting client only)
  await p2.getByRole("button", { name: /ON AIR/ }).click();
  await expect(p2.getByText("JOIN THE CONVERSATION")).toBeVisible();
  await expect(p1.getByText("ON AIR — click to mute")).toBeVisible();
  await expect(p1.getByText("Speakers (2)")).toBeVisible({ timeout: 5_000 }); // p1's mirror saw p2 leave
  // field page agrees: its tally-ring holder count drops to 3 -> 2 (the field
  // page has no Speakers card; the ring locator is its native cross-check)
  await expect(f3.locator(".tally-ring.holder")).toHaveCount(2, { timeout: 5_000 });
  await f3.keyboard.up("Space");

  await d.context().close();
  await p1.context().close();
  await p2.context().close();
  await f3.context().close();
});
