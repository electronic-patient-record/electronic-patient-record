# AGENTS.md — Service

loco.rs (Axum 0.8) service on PostgreSQL 18. Read [`spec/index.md`](spec/index.md) first.

- **Status**: early. Built: app wiring, secure-by-default PASETO and Keycloak guard (`src/auth.rs`), the policy engine and
  its route gate (`src/policy.rs`, `src/authz.rs`, `src/routes.rs`), the hash-chained audit (`src/audit.rs`),
  `POST /api/patients/compare`, and patient create, read, update and search over append-only versions
  (`src/patients_store.rs`, `src/controllers/patients.rs`), and patient consent (`src/consent.rs` the rules,
  `src/consents_store.rs` the append-only versions, `src/controllers/consents.rs`; a refusal hides a patient's identity
  from the named roles). Not built: the rest of the resource store, FHIR endpoints, break-the-glass grants, masking, a
  any export. Care relationships (`src/relationships.rs`) are recorded by an administrator; nothing derives them from encounters.
- **Two token kinds**: `v4.…` goes to the PASETO verifier, anything else to Keycloak (when `EPR_KEYCLOAK_URL` is set). Never route one to the other. The token's `role` attribute (Keycloak: `EPR_KEYCLOAK_ROLE_MAP_FILE=operations/keycloak/role-map.json`) is the only thing authorization reads. See `operations/keycloak/README.md`.
- **Auth is on unless explicitly off.** `settings.require_auth: false` is set only in development/test config.
  The service refuses to start with login off in `production`, and with login on and no keys or key URL.
- **Append-only.** No migration may add a delete path for clinical data. `audit_events` has triggers refusing
  UPDATE/DELETE/TRUNCATE; keep the DB-gated test green.
- The binary needs the explicit `[[bin]]` entry in `Cargo.toml` (it lives in `src/bin/main.rs`).
- `loco-rs` has `default-features = false` and no `auth` feature (no `jsonwebtoken`/`rsa`).
- Gates: `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --lib`; then
  the DB-gated tests (see the repository `AGENTS.md`).
- **Every new endpoint needs a row in `src/routes.rs` `TABLE`** (method, path template, resource, action). A route
  without one is refused with 403 for every caller, and `tests/route_census.rs` fails. Pick the resource and action
  deliberately; do not widen a role in `policy.rs` to make a test pass. A handler that loads a stored record must also
  call `policy::evaluate` with the record's labels and the caller's relationship, honour its `Obligation`s (mask,
  audit, break-the-glass flag), and audit with `audit::append` before it returns data (see `compare_audited`).
- The permission table, label lists and purpose rules in `policy.rs` are an engineer's first draft, not clinical
  governance. Say so in any change to them and update the hazard log (AS-1).
