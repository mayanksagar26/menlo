import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright drives the plan-review flow (§2) against the Vite dev server with a
 * stubbed Tauri IPC — see `e2e/fixtures.ts`. It exercises the screen the whole product
 * rests on without needing a compiled binary or a real folder on disk.
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: "http://localhost:1420",
    trace: "on-first-retry",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "npm run dev",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
