import { defineConfig } from "@playwright/test";

const runId = (
  process.env.GITHUB_RUN_ID
    ? `${process.env.GITHUB_RUN_ID}-${process.env.GITHUB_RUN_ATTEMPT ?? "1"}`
    : process.env.ANGUI_E2E_RUN_ID ?? "local"
).replace(/[^a-zA-Z0-9-]/g, "-");
const portSeed = [...runId].reduce(
  (hash, character) => (hash * 31 + character.charCodeAt(0)) % 10_000,
  0,
);
const backendPort = Number(process.env.ANGUI_E2E_BACKEND_PORT ?? 20_000 + portSeed);
const frontendPort = Number(
  process.env.ANGUI_E2E_FRONTEND_PORT ?? 40_000 + portSeed,
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
  webServer: {
    command: "node e2e/start-e2e.mjs",
    cwd: "..",
    // Readiness goes through Vite's proxy, so both the frontend listener and
    // the backend health endpoint must be available before tests begin.
    url: `http://127.0.0.1:${frontendPort}/api/health`,
    // A cold Cargo cache compiles the migration and application binaries
    // before the health endpoint becomes available.
    timeout: 300_000,
    reuseExistingServer,
    env: {
      DATABASE_URL: databaseUrl,
      ANGUI_E2E_DATABASE_FILE: databaseFile,
      ANGUI_E2E_BACKEND_PORT: String(backendPort),
      ANGUI_E2E_FRONTEND_PORT: String(frontendPort),
      ANGUI_HOST: "127.0.0.1",
      ANGUI_PORT: String(backendPort),
      ANGUI_FRONTEND_ORIGIN: `http://127.0.0.1:${frontendPort}`,
      ANGUI_RUNTIME_ENV: "test",
      ANGUI_ALLOW_DEMO_BOOTSTRAP: "1",
      ANGUI_DEMO_PASSWORD: "e2e-demo-password",
      ANGUI_DEMO_GRANT_REVIEWER_ADMINS: "1",
      ANGUI_ATTACHMENT_STORAGE_DIRECTORY: `.e2e/attachments-${runId}`,
      VITE_API_PROXY_TARGET: `http://127.0.0.1:${backendPort}`,
      RUST_LOG: "info,sqlx=warn",
    },
  },
});
