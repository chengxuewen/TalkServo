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
  reporter: [["list"]],
});
