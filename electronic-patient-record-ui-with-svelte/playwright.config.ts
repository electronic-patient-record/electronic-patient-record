import { defineConfig, devices } from "@playwright/test";

// The backend is stubbed per test via `page.route`, so no running Rust service
// is needed. The production build is served by `vite preview`. Another project's
// server may already hold 4173 on a shared machine: run with PW_PORT=4183.
const PORT = Number(process.env.PW_PORT ?? 4173);

export default defineConfig({
  testDir: "tests/e2e",
  timeout: 30_000,
  fullyParallel: true,
  use: { baseURL: `http://localhost:${PORT}`, trace: "on-first-retry" },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: `pnpm run build && pnpm exec vite preview --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
