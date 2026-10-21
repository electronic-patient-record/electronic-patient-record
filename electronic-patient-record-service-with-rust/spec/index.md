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

| Policy engine (SE-1) | `policy.rs`: roles from the token's `role` attribute (`clinician nurse pharmacist allied-health receptionist clinical-admin patient proxy auditor integrator`; unknown values are dropped) × nine resource kinds × six actions (no delete). `gate` is the route-level check; `evaluate` is the record-level one (care relationship, security labels `R V PSY HIV SUD ETH`, purpose of use, consent, break-the-glass), returning `Allow(obligations)` or `Deny`. Break-the-glass lifts only a missing care relationship, only for a read by a treating role, and never overrides a consent refusal or unmasks a label | `policy::tests` (15, including a matrix of 10 roles × 9 resources × 6 actions × 8 label sets × 5 relationships × 3 consents × 7 purposes × 2 grants = 907,200 requests checked against invariants) |
| Route gate | `authz::authorize_mw` runs after authentication on every request: public paths and login-off pass; a route with no entry in `routes::TABLE` is 403 for everyone; a caller whose roles do not permit the entry's resource and action is 403. Each refusal is audited (outcome 4; route template, method and roles only, never the body) | `authz::tests`, `tests/keycloak_guard.rs` (9), `tests/authz_audit.rs` (3, Postgres); run by hand against Keycloak 26.3.0: clinician 200, auditor 403, no role 403, unlisted route 403, no token 401 |
| Route census | Every served route is public (the three health probes) or in `routes::TABLE`, and every table entry is served | `tests/route_census.rs` (3); fails when the table is emptied |

With login off (development only) there is no subject and the gate stands aside, as authentication does.

## 2. Configuration

`EPR_REQUIRE_AUTH`, `EPR_KEYCLOAK_URL` / `_REALM` / `_AUDIENCES` / `_ROLE_MAP_FILE` (see `operations/keycloak/README.md`), `EPR_PASETO_KEYS` (inline JSON key set), `EPR_PASETO_KEYS_URL`, `EPR_INTEGRITY_MAC_KEY` / `EPR_INTEGRITY_MAC_KEY_FILE` (audit chain key), `EPR_TOKEN_ISSUER`
(default `authentication-service`), `EPR_TOKEN_AUDIENCE` (default `electronic-patient-record`), `EPR_REDIS_URL`
(production cache), `DATABASE_URL`, `PORT`.

## 3. Not built

Patient storage and CRUD; the resource store; FHIR endpoints and the CapabilityStatement; consent records (SE-2: the engine takes
a consent value but nothing stores or looks one up); break-the-glass grants (SE-3: the engine checks a grant but nothing creates,
stores or expires one, and the privacy-officer notification is only an obligation); masking (SE-4: the engine says what to hide,
nothing hides it); the care-relationship lookup; the record-level `evaluate` is called by no handler yet because no handler reads a stored record;
the access log for patients (SE-6); an HTTP endpoint to verify the chain (AU-2); audit of 401 responses (only 403s from the gate are audited); refusing to start in `production` without an integrity key (it only warns); adopting a key on a chain that already has unkeyed rows (they would fail verification; set the key before the first write); OpenAPI; rate limiting; CORS allow-list; the search index.
The permission table, label lists and purpose rules are an engineer's first draft, unreviewed by a clinical safety officer or privacy
officer. A valid token whose roles the table permits is enough for the one route there is.
