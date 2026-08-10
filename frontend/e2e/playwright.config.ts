import { defineConfig } from "@playwright/test";

const runId = (
  process.env.GITHUB_RUN_ID
    ? `${process.env.GITHUB_RUN_ID}-${process.env.GITHUB_RUN_ATTEMPT ?? "1"}`
    : `local-${process.pid}-${Date.now()}`
).replace(/[^a-zA-Z0-9-]/g, "-");
const portOffset = (process.pid % 1000) * 2;
const backendPort = Number(process.env.ANGUI_E2E_BACKEND_PORT ?? 8081 + portOffset);
const frontendPort = Number(
  process.env.ANGUI_E2E_FRONTEND_PORT ?? 5174 + portOffset,
);
const databaseFile = `.e2e/angui-e2e-${runId}.db`;
const databaseUrl = `sqlite://${databaseFile}?mode=rwc`;
const reuseExistingServer =
  !process.env.CI && process.env.PLAYWRIGHT_REUSE_EXISTING_SERVER === "1";

export default defineConfig({
  testDir: ".",
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI ? [["github"], ["html", { open: "never" }]] : "list",
  outputDir: "../test-results",
  use: {
    baseURL: `http://127.0.0.1:${frontendPort}`,
    channel: process.env.PLAYWRIGHT_BROWSER_CHANNEL,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  webServer: [
    {
      command: "node e2e/start-backend.mjs",
      cwd: "..",
      url: `http://127.0.0.1:${backendPort}/api/health`,
      // A cold Cargo cache compiles the migration and application binaries
      // before the health endpoint becomes available.
      timeout: 300_000,
      reuseExistingServer,
      env: {
        DATABASE_URL: databaseUrl,
        ANGUI_E2E_DATABASE_FILE: databaseFile,
        ANGUI_HOST: "127.0.0.1",
        ANGUI_PORT: String(backendPort),
        ANGUI_FRONTEND_ORIGIN: `http://127.0.0.1:${frontendPort}`,
        ANGUI_RUNTIME_ENV: "test",
        ANGUI_ALLOW_DEMO_BOOTSTRAP: "1",
        ANGUI_DEMO_PASSWORD: "e2e-demo-password",
        ANGUI_DEMO_GRANT_REVIEWER_ADMINS: "1",
        ANGUI_ATTACHMENT_STORAGE_DIRECTORY: `.e2e/attachments-${runId}`,
        RUST_LOG: "info,sqlx=warn",
      },
    },
    {
      command: "node node_modules/vite/bin/vite.js --host 127.0.0.1 --port 5174 --strictPort",
      cwd: "..",
      url: `http://127.0.0.1:${frontendPort}`,
      timeout: 60_000,
      reuseExistingServer,
      env: {
        VITE_API_PROXY_TARGET: `http://127.0.0.1:${backendPort}`,
      },
    },
  ],
});
