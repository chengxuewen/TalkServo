import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@talkservo/client": path.resolve(__dirname, "../packages/client/src/index.ts"),
    },
  },
  server: { port: 5173 },
  test: {
    environment: "jsdom",
    setupFiles: ["./tests/setup.ts"],
    exclude: ["**/node_modules/**", "**/dist/**", "**/e2e/**"],
    // AntD + jsdom: one Table/Drawer mount costs ~900ms of real CPU, so the
    // intervention cases need several seconds. Default 5000ms is CI-flaky.
    testTimeout: 15000,
  },
});
