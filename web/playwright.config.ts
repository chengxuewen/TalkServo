import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  use: {
    baseURL: process.env.TS_URL ?? "http://127.0.0.1:8080",
    browserName: "chromium",
    launchOptions: {
      args: [
        "--use-fake-device-for-media-stream",
        "--use-fake-ui-for-media-stream",
        "--autoplay-policy=no-user-gesture-required",
      ],
    },
    permissions: ["microphone"],
  },
  // the suite starts its own server in globalSetup
  globalSetup: "./e2e/global-setup.ts",
  workers: 1, // shared room state; serialize
  reporter: [["list"]],
});
