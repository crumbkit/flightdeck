import { defineConfig, devices } from "@playwright/test";

// Acceptance tests drive the built app in real browsers (APP-7). Each test
// runs in light and in dark appearance.
const port = 4173;

export default defineConfig({
  testDir: "tests/acceptance",
  forbidOnly: true,
  // A failed test is never retried (BASE-12).
  retries: 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: `http://localhost:${port}`,
    trace: "retain-on-failure",
  },
  projects: [
    { name: "light", use: { ...devices["Desktop Chrome"], colorScheme: "light" } },
    { name: "dark", use: { ...devices["Desktop Chrome"], colorScheme: "dark" } },
  ],
  // The lane builds before this step, so the server only serves dist/.
  webServer: {
    command: `vite preview --port ${port} --strictPort`,
    port,
    reuseExistingServer: false,
  },
});
