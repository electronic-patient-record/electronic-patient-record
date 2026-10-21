import { defineConfig, devices } from "@playwright/test";

// End-to-end sign-in against a real Keycloak and a real API (task SE-9). Not part
// of the default run: it needs containers. See operations/keycloak/README.md.
//   EPR_E2E_API_URL=http://127.0.0.1:5190  EPR_E2E_USERNAME=dev.clinician  EPR_E2E_PASSWORD=…
//   pnpm exec playwright test -c playwright.keycloak.config.ts
// The port must be one the `epr-web` client allows (see operations/keycloak/epr-realm.json).
const PORT = Number(process.env.PW_KC_PORT ?? 4310);
const ORIGIN = `http://127.0.0.1:${PORT}`;
const ISSUER = process.env.EPR_E2E_ISSUER ?? "http://127.0.0.1:8081/realms/epr";

export default defineConfig({
  testDir: "tests/e2e-keycloak",
  timeout: 60_000,
  workers: 1,
  use: { baseURL: ORIGIN, trace: "on-first-retry" },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: `pnpm run build && pnpm exec vite preview --host 127.0.0.1 --port ${PORT} --strictPort`,
    url: ORIGIN,
    reuseExistingServer: false,
    timeout: 120_000,
    env: {
      EPR_API_URL: process.env.EPR_E2E_API_URL ?? "http://127.0.0.1:5150",
      EPR_OIDC_ISSUER: ISSUER,
      EPR_OIDC_CLIENT_ID: "epr-web",
      EPR_PUBLIC_URL: ORIGIN,
      EPR_SESSION_IDLE_SECONDS: "900",
    },
  },
});
