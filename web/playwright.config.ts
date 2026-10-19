import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  // console-checklist §6-K parity (LiveKit/Jitsi): no flake-masking retries,
  // keep failure traces, never leave a stray .only in a commit
  retries: 0,
  forbidOnly: !!process.env.CI,
  use: {
    baseURL: process.env.TS_URL ?? "http://127.0.0.1:8080",
    browserName: "chromium",
    trace: "retain-on-failure",
    launchOptions: {
      args: [
        "--use-fake-device-for-media-stream",
        "--use-fake-ui-for-media-stream",
        "--autoplay-policy=no-user-gesture-required",
      ],
    },
    permissions: ["microphone"],
  },
  // the suite starts its own server in globalSetup (this line + workers were
  // lost in a config-merge edit during the test-honesty batch and rode along
  // unexecuted; P3 forensics caught it: every ctest-lane e2e since ran BARE -
  // no server, no TS_URL/TS_E2E_SECRET. Lesson encoded: the gate must be the
  // LAST action before a commit, never merely 'a run that happened during' the batch)
  globalSetup: "./e2e/global-setup.ts",
  workers: 1, // shared room state; serialize
  reporter: [["list"]],
});
