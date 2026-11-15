import { defineConfig, devices } from "@playwright/test";

// The backend is stubbed per test via `page.route`, so no running Rust service
// is needed. The production build is served by `vite preview`. Another project's
// server may already hold 4173 on a shared machine: run with PW_PORT=4183.
const PORT = Number(process.env.PW_PORT ?? 4173);

export default defineConfig({
  testDir: "tests/e2e",
  timeout: 30_000,
  fullyParallel: true,
  // A fixed zone, so a time the page prints in the reader's zone is the same on every machine.
  use: {
    baseURL: `http://localhost:${PORT}`,
    trace: "on-first-retry",
    timezoneId: "UTC",
  },
  // Every test runs twice: with the device in light mode and in dark mode, so axe checks every page in both
  // passing themes (TH-1, spec/index.md "Themes").
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], colorScheme: "light" },
    },
    {
      name: "chromium-dark",
      use: { ...devices["Desktop Chrome"], colorScheme: "dark" },
    },
  ],
  webServer: {
    command: `pnpm run build && pnpm exec vite preview --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
