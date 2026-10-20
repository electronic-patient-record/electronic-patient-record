# AGENTS.md — UI

SvelteKit (kit 3), Svelte 5 runes, TypeScript strict, adapter-node. Read [`spec/index.md`](spec/index.md) first.

- **Status**: shell. Built: locale-prefixed URLs, security headers and a strict CSP, the `/api/proxy/*` BFF proxy,
  a home page and a patient-compare page. Not built: sign-in/session, patient pages, Lily widgets, the `DataTable`
  wrapper, translations (English only; the ten `LOCALES` fall back to it).
- Environment variables are declared in `src/env.ts` (`defineEnvVars`) and read from `$app/env/private`.
- The browser never calls the API directly: it calls `/api/proxy/api/...`, and `src/lib/proxy.ts` refuses anything
  outside `/api`. Keep that rule and its tests.
- Computed styles use the `style:` directive; the CSP forbids inline `style=""`.
- Every page must pass the axe audit in `tests/e2e/smoke.spec.ts` (WCAG 2.2 AA). Add new pages to that list.
- Never display a missing value as `0` or "none"; show "not available".
- Gates: `pnpm run check` (0 errors, 0 warnings), `pnpm exec vitest run`, `PW_PORT=4183 CI=true pnpm exec playwright test`.
