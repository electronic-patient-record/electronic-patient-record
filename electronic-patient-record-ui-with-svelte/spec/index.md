# UI — Specification

Source of truth for the UI's internals. The entity spec ([`../../spec/index.md`](../../spec/index.md)) wins on the
cross-subproject contract.

## 1. Built (M0)

- **URLs**: `/<locale>/…` with the ten `LOCALES`; a bare URL redirects to the remembered, browser-preferred or
  default (`en-001`) locale; `/api`, `/auth`, `/_app`, `/assets` are unlocalised.
- **Security**: strict CSP from `vite.config.ts` (nonces/hashes, no inline style except SvelteKit's own announcer),
  `nosniff`, `DENY` framing, referrer and permissions policies, HSTS over HTTPS.
- **BFF**: `/api/proxy/<path>` forwards GET/POST to `EPR_API_URL` only when the path starts with `api/` and has no
  empty, `.`, `..`, `%` or backslash segment. The browser holds no token.
- **Sign-in (SE-9)**: with `EPR_OIDC_ISSUER` set, `/auth/login` starts the authorization code flow with PKCE (S256, state, nonce)
  against the Keycloak realm; `/auth/callback` checks the state and the ID token's issuer, audience and nonce, and creates a
  session with a fresh random id; `/auth/logout` (POST, same-origin only) ends it here and at Keycloak. The browser holds only the
  `httpOnly`, `SameSite=Lax` cookie `epr_session` (`__Host-epr_session` over HTTPS); tokens stay in the server's in-memory store
  (`src/lib/session.ts`). The proxy adds `Authorization: Bearer`, refreshes within 30 s of expiry, and answers 401 when there is no
  live session. Idle limit 900 s and absolute limit 28800 s (`EPR_SESSION_IDLE_SECONDS`, `EPR_SESSION_ABSOLUTE_SECONDS`). `returnTo`
  is a same-site path only. Plain `http://` is accepted only for loopback. Environment: `EPR_OIDC_ISSUER`, `EPR_OIDC_CLIENT_ID`
  (default `epr-web`), `EPR_PUBLIC_URL` (required with an issuer; its origin must be one the client allows).
- **Pages**: home; `/patients` compares two patients and renders the decision, the score, the per-field table and
  the reasons a link cannot be automatic. A missing similarity renders "not available".
- **Accessibility**: skip link, landmarks, labelled inputs, `role="alert"` on failure, axe (WCAG 2.2 AA) on both pages.

## 2. Not built

A session store shared between UI replicas (the store is in process memory); Keycloak-initiated logout; the Keycloak end-to-end
run in CI; patient search,
registration, banner, summary; the clinical pages; the Lily widgets and `DataTable`; translations; the inbox; the
portal; dark-theme toggle (colours follow `prefers-color-scheme`).
