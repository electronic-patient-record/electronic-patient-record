# Changelog

Each entry says what was not done or not verified. Per-crate changelogs hold the detail.

## [0.1.0] - 2026-10-20

Milestone M0, scaffold. **Not for clinical use.**

### Added

- **Matcher** (`electronic-patient-record-matcher-with-rust`): `Patient` identity type; deterministic, symmetric,
  explainable matching with per-field breakdown; NHS number modulus-11 validation; refusals for twins, missing
  given names, differing verified NHS numbers; 13 tests.
- **Service** (`electronic-patient-record-service-with-rust`): loco.rs app; PASETO bearer guard, on by default,
  refusing to start without keys or with login off in production; `POST /api/patients/compare`; append-only
  `audit_events` table with triggers; 6 unit tests and 2 database-gated tests.
- **UI** (`electronic-patient-record-ui-with-svelte`): SvelteKit shell, locale-prefixed URLs, strict CSP and
  security headers, proxy that only forwards `/api`, a compare page; 26 unit tests, 7 browser tests with axe.
- CI, release workflow, root `compose.yaml`, Containerfiles, scripts.

### Added after the first cut: audit writer (AU-1, partly)

- `audit::append` / `audit::verify`: hash-chained, keyed with `EPR_INTEGRITY_MAC_KEY` when set. `compare` now writes an
  audit row before answering and returns 500, with no result, if it cannot. 6 unit and 7 Postgres tests.
- Found and fixed while testing by hand: my first key variable name was wrong (`EPR_INTEGRITY_KEY`; the crate derives
  `EPR_INTEGRITY_MAC_KEY`), so the chain silently stayed unkeyed. A unit test now pins the names.

### Added after the first cut: Keycloak (SE-8)

- The guard accepts Keycloak access tokens; `v4.` tokens still go to PASETO, everything else to Keycloak. The audit agent is the
  token `sub`. Invalid Keycloak configuration refuses to start. 8 DB-free tests plus a by-hand run against Keycloak 26.3.0.
- `keycloak` compose profile (Keycloak on the shared PostgreSQL, realm `epr` imported), `scripts/keycloak-dev-user.sh`,
  `scripts/keycloak-dev-token.sh`, and standalone `scripts/podman-postgres.sh` / `scripts/podman-redis.sh` (both tried).
- **Not done:** roles are not consulted; the UI cannot sign in (SE-9); the API cannot reach a plain-HTTP Keycloak from inside compose
  (it needs HTTPS); the CI container job does not exercise Keycloak; tested only on one node, `start-dev`.

### Added after the first cut: UI sign-in (SE-9)

- The UI signs readers in through Keycloak (`epr-web`, authorization code flow with PKCE) in its backend-for-frontend. The browser holds one
  opaque `httpOnly`, `SameSite=Lax` session cookie (`__Host-` prefixed over HTTPS); the tokens stay in the server's session store, which
  sends the access token to the API, refreshes it before it expires, ends idle (15 min) and over-long (8 h) sessions, and signs out
  locally first and then at Keycloak. Cross-site POSTs to the proxy and to sign-out are refused. Off unless `EPR_OIDC_ISSUER` is set;
  a bad value stops the UI at startup. 24 new unit tests (50 in all).
- Verified by hand on 2026-10-21 against Keycloak 26.3.0 and the real API (`playwright.keycloak.config.ts`, 5 passed, 1 skipped): sign-in, a
  protected call that returned 200 with the audit row's agent set to the Keycloak `sub`, sign-out, a tampered cookie, a replayed
  pre-sign-out cookie, a forged callback, and cross-site POSTs.
- **Not done:** the session store is in process memory, so a restart signs everyone out and two UI replicas cannot share sessions; token
  refresh and idle expiry are tested with a stub provider only (Keycloak's access token lives 300 s and the run did not wait); the
  ID token is decoded, not signature-checked (allowed for a token straight from the token endpoint, OIDC Core 3.1.3.7; the API checks
  the access token on every call); no front-channel or back-channel logout from Keycloak into the UI; the Keycloak end-to-end run is
  not in CI; no Entra ID; the sign-in and sign-out text is English only.

### Added after the first cut: policy engine and route gate (SE-1, in part)

- `policy.rs`: ten roles, nine resource kinds, six actions (no delete), care relationship, security labels, purpose of use, consent and
  break-the-glass, as a pure engine. `authz.rs` gates every route on the token's `role` attribute; a route with no entry in
  `routes.rs` is 403 for everyone; each refusal is audited (route template, method and roles, never the body). `tests/route_census.rs`
  fails when the served routes and the table disagree (checked by emptying the table). Keycloak realm roles map to policy roles through
  `operations/keycloak/role-map.json`; `scripts/keycloak-dev-user.sh` takes a role.
- Tested: 15 engine tests including a 907,200-request matrix checked against invariants; 9 DB-free guard tests; 3 Postgres tests of the
  gate and its audit rows. By hand against Keycloak 26.3.0 and the real API: clinician 200, auditor 403, no role 403, unlisted route 403, no
  token 401; the UI shows "Your role does not allow this." for the no-role user (6 end-to-end tests passed).
- **Behaviour change:** a valid Keycloak or PASETO token with no recognised role used to be enough for `compare`; it now gets 403.
- **Not done:** the record-level `evaluate` is called by no handler; no consent store, break-the-glass grant store or masking (SE-2 to SE-4);
  no care-relationship lookup. **The permission table, label lists and purpose rules are an engineer's first draft that no clinical safety
  officer or privacy officer has reviewed.** With login off there is no authorization (development only). 401s are not audited.

### Changed after the first cut

- The matcher is published as `electronic-patient-record-matcher` 0.1.0 on crates.io; the service depends on it from
  the registry (no path dependency), so its container build no longer copies the matcher.
- Docker replaced by Podman everywhere: `Containerfile` and `.containerignore` instead of `Dockerfile` and
  `.dockerignore`; `podman build` / `podman compose` in CI and release. Image names still use the `docker.io/…`
  registry, which is only a registry name.

### Not done or not verified

- No clinical record, no FHIR endpoints, no patient storage, no patient CRUD, no merge.
- Audit: no verify endpoint, denied requests are not audited, no other route is audited (none reads stored data), and
  an existing unkeyed chain cannot adopt a key later. Without `EPR_INTEGRITY_MAC_KEY` the chain is unkeyed SHA-256
  (warned at startup), so someone who can write SQL can recompute it.
- Authorization is a role gate per route only; consent, break-the-glass and masking are not built (SE-2 to SE-4). The UI signs in through
  Keycloak, but with sign-in off (no `EPR_OIDC_ISSUER`) its proxy sends no token and only works against an API with login off.
- The matcher has 13 hand-written tests, not the 200-pair labelled corpus the acceptance criterion in ID-1 asks
  for, and no fuzz target. Its thresholds are unvalidated guesses.
- Only English strings exist; the other nine locales fall back to English. No Lily widgets are used yet.
- Run by hand on 2026-10-20: both images built; the compose stack reached healthy; with no keys the API container
  exits 1 ("refusing to start"); with a key set an unauthenticated call gets 401; the UI image serves, redirects to
  `/en-001`, and its proxy 404s outside `/api`.
- **Not run:** the GitHub Actions workflows (`ci.yml`, `release.yml`) have never executed (the Podman steps in them, including `podman-compose` and the multi-arch `podman manifest` push, are untested; `podman compose` locally delegates to a Homebrew `docker-compose` client talking to Podman), so any error in them is
  unknown; there is no zap job and no chaos workflow; the release image and SBOM steps are untested.
- No hazard log, no clinical safety officer, no governance pack (AS-1 to AS-3).
