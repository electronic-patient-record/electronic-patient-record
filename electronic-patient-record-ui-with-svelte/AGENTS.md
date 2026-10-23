# AGENTS.md — UI

SvelteKit (kit 3), Svelte 5 runes, TypeScript strict, adapter-node. Read [`spec/index.md`](spec/index.md) first.

- **Status**: shell. Built: locale-prefixed URLs, security headers and a strict CSP, the `/api/proxy/*` BFF proxy,
  Keycloak sign-in with server-side sessions (`/auth/*`, off unless `EPR_OIDC_ISSUER` is set), a home page, patient
  search, registration and compare pages, a patient record page with an access-log page, `/my-record`, and admin pages
  for the HL7 feed and break-the-glass review (see `src/routes`). Not built: a shared session store, Lily widgets,
  the `DataTable` wrapper, translations (English only; the ten `LOCALES` fall back to it).
- Environment variables are declared in `src/env.ts` (`defineEnvVars`) and read from `$app/env/private`.
- The browser never calls the API directly: it calls `/api/proxy/api/...`, and `src/lib/proxy.ts` refuses anything
  outside `/api`. Keep that rule and its tests.
- Tokens never reach the browser: the cookie holds only a random session id. Do not put a token in page data, `locals`,
  a cookie value or a log line. `redirect()` to the identity provider is allowed only to the issuer's origin.
- Sign-in end-to-end (needs Keycloak and the API; see `operations/keycloak/README.md`):
  `EPR_E2E_API_URL=… EPR_E2E_PASSWORD=… pnpm exec playwright test -c playwright.keycloak.config.ts`.
- Computed styles use the `style:` directive; the CSP forbids inline `style=""`.
- Every page must pass the axe audit in `tests/e2e/smoke.spec.ts` (WCAG 2.2 AA). Add new pages to that list.
- Never display a missing value as `0` or "none"; show "not available".
- Gates: `pnpm run check` (0 errors, 0 warnings), `pnpm exec vitest run`, `PW_PORT=4183 CI=true pnpm exec playwright test`.
