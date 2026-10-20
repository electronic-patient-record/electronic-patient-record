# Service — Specification

Source of truth for the service's internals. The entity spec ([`../../spec/index.md`](../../spec/index.md)) wins on
the cross-subproject contract.

## 1. Built (M0)

| Area | Behaviour | Evidence |
|---|---|---|
| Login enforcement | On unless `settings.require_auth: false` or `EPR_REQUIRE_AUTH=false`; an explicit env value wins over config | `auth::tests::auth_is_on_by_default` |
| Startup gate | Refuses to start in `production` with login off; or with login on, no keys and no `EPR_PASETO_KEYS_URL` | `auth::tests::startup_gate`; run by hand |
| Public paths | `/_health`, `/_ping`, `/_readiness` only | `auth::tests` |
| Fail closed | Before the guard is initialised every non-public request gets 503 | by construction |
| Compare | `POST /api/patients/compare` takes `{a, b}` (matcher `Patient`) and returns a matcher `MatchResult` | `controllers::patients::tests` |
| Audit table | `audit_events`, append-only; action in C/R/U/D/E; outcome in 0/4/8/12; triggers refuse UPDATE, DELETE, TRUNCATE | `tests/audit_immutable.rs` (Postgres) |

## 2. Configuration

`EPR_REQUIRE_AUTH`, `EPR_PASETO_KEYS` (inline JSON key set), `EPR_PASETO_KEYS_URL`, `EPR_TOKEN_ISSUER`
(default `authentication-service`), `EPR_TOKEN_AUDIENCE` (default `electronic-patient-record`), `EPR_REDIS_URL`
(production cache), `DATABASE_URL`, `PORT`.

## 3. Not built

Patient storage and CRUD; the resource store; FHIR endpoints and the CapabilityStatement; the policy engine;
consent; break-the-glass; any write to `audit_events`; the hash chain (the `digest` and `prev_digest` columns exist
but nothing computes them); OpenAPI; rate limiting; CORS allow-list; the search index.
A valid token is currently enough to call any route.
