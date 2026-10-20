# Service — Specification

Source of truth for the service's internals. The entity spec ([`../../spec/index.md`](../../spec/index.md)) wins on
the cross-subproject contract.

## 1. Built (M0)

| Area | Behaviour | Evidence |
|---|---|---|
| Login enforcement | On unless `settings.require_auth: false` or `EPR_REQUIRE_AUTH=false`; an explicit env value wins over config | `auth::tests::auth_is_on_by_default` |
| Startup gate | Refuses to start in `production` with login off; or with login on, no keys and no `EPR_PASETO_KEYS_URL` | `auth::tests::startup_gate`; run by hand |
| Keycloak | With `EPR_KEYCLOAK_URL` set, a bearer token that does not start `v4.` is verified against the realm (signature, issuer `{url}/realms/{realm}`, audience, expiry, verified email); the caller is the token `sub`. Half-configured Keycloak, a remote `http://` URL, no audience, or a bad role map: refuses to start | `tests/keycloak_guard.rs` (8, DB-free, local OIDC stand-in); run by hand against Keycloak 26.3.0 under Podman |
| Public paths | `/_health`, `/_ping`, `/_readiness` only | `auth::tests` |
| Fail closed | Before the guard is initialised every non-public request gets 503 | by construction |
| Compare | `POST /api/patients/compare` takes `{a, b}` (matcher `Patient`) and returns a matcher `MatchResult` | `controllers::patients::tests` |
| Audit table | `audit_events`, append-only; action in C/R/U/D/E; outcome in 0/4/8/12; triggers refuse UPDATE, DELETE, TRUNCATE | `tests/audit_immutable.rs` (Postgres) |
| Audit writer | `audit::append` adds a row in its own transaction under an advisory lock; each `digest` covers the previous row's `digest` (keyed MAC via `integrity-mac` when `EPR_INTEGRITY_MAC_KEY` is set, else unkeyed SHA-256, with a startup warning); `audit::verify` recomputes the whole chain and names the first broken row | `audit::tests` (6); `tests/audit_chain.rs` (7, Postgres): intact chain keyed and unkeyed, edit and delete detected, a recomputed digest without the key detected, 20 concurrent appends do not fork |
| Compare is audited | `POST /api/patients/compare` appends an `E` row (agent = token `sub`, or `anonymous` with login off; detail = decision and score only, never demographics) **before** returning; if the append fails the request fails (500) and no result is returned | `tests/audit_chain.rs`; run by hand with the audit table missing |

## 2. Configuration

`EPR_REQUIRE_AUTH`, `EPR_PASETO_KEYS` (inline JSON key set), `EPR_PASETO_KEYS_URL`, `EPR_INTEGRITY_MAC_KEY` / `EPR_INTEGRITY_MAC_KEY_FILE` (audit chain key), `EPR_TOKEN_ISSUER`
(default `authentication-service`), `EPR_TOKEN_AUDIENCE` (default `electronic-patient-record`), `EPR_REDIS_URL`
(production cache), `DATABASE_URL`, `PORT`.

## 3. Not built

Patient storage and CRUD; the resource store; FHIR endpoints and the CapabilityStatement; the policy engine;
consent; break-the-glass; an HTTP endpoint to verify the chain (AU-2); audit of denied requests (401/403); refusing to start in `production` without an integrity key (it only warns); adopting a key on a chain that already has unkeyed rows (they would fail verification; set the key before the first write); OpenAPI; rate limiting; CORS allow-list; the search index.
A valid token (PASETO or Keycloak) is currently enough to call any route; Keycloak roles are not consulted.
