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
- **Pages**: home; `/patients` searches (family-name prefix, birth date, NHS number; an empty search is never sent) and lists up to 50
  matches; `/patients/new` registers, and on a 409 shows each possible duplicate beside the entered record with the matcher's score and
  reasons, registering anyway only with a typed reason; `/patients/[id]` shows and edits a record, sending the version it was read at
  (a stale save is reported, never overwritten); `/patients/compare` compares two patients and renders the decision, the score, the per-field
  table and the reasons a link cannot be automatic. A missing similarity renders "not available". The edit form changes only the fields it
  shows and carries every other name, identifier, address and contact detail into the new version.
- **Purpose of use**: the proxy forwards `X-Purpose-Of-Use` only when it is one of the seven codes the API knows; anything else is dropped before it can reach the audit trail. Staff declare theirs in a header picker, **Operations** (the API's default) or **Treatment**, kept for the browser tab only (`sessionStorage`; if storage is blocked the choice lasts until reload). `emergency` is not offered until break-the-glass (SE-3) exists, and `payment`, `research` and `audit` are permitted to no staff role. `/my-record` ignores the picker and sends `patient-access`. Declaring a purpose grants nothing: the API still checks it against the role and records it.
- **My record** (`/my-record`): read-only list of the records linked to the signed-in patient or proxy, from `GET /api/me/patients`. A proxy is told it is a proxy; with no link the page says so; a refusal is announced. No edit form: corrections go through the practice.
- **Who has accessed this record** (on `/my-record`, per record): a button loads `GET /api/me/patients/{id}/access-log` only when pressed, because reading it is itself recorded. Entries are sentences (`describe` in `src/lib/access-log.ts`): "A member of staff viewed your identity record. Purpose: treatment." Staff are shown by the role and organisation recorded ("A member of staff (clinician, riverside)…"), never by name; words the API sends that the page does not know are shown as given. "Show older entries" follows `next`.
- **Accessibility**: skip link, landmarks, labelled inputs, `role="alert"` on failure, axe (WCAG 2.2 AA) on each page and on the duplicate view.

## 2. Not built

A session store shared between UI replicas (the store is in process memory); Keycloak-initiated logout; the Keycloak end-to-end
run in CI; patient search,
registration, banner, summary; the clinical pages; the Lily widgets and `DataTable`; translations; the inbox; the
portal; dark-theme toggle (colours follow `prefers-color-scheme`).
