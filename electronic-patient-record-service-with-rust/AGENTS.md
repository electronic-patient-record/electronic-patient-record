# AGENTS.md — Service

loco.rs (Axum 0.8) service on PostgreSQL 18. Read [`spec/index.md`](spec/index.md) first.

- **Status**: scaffold. Built: app wiring, secure-by-default PASETO guard (`src/auth.rs`), `POST
  /api/patients/compare` over the matcher, the append-only `audit_events` migration. Not built: patient storage,
  the resource store, FHIR endpoints, the policy engine, audit of any route but compare.
- **Auth is on unless explicitly off.** `settings.require_auth: false` is set only in development/test config.
  The service refuses to start with login off in `production`, and with login on and no keys or key URL.
- **Append-only.** No migration may add a delete path for clinical data. `audit_events` has triggers refusing
  UPDATE/DELETE/TRUNCATE; keep the DB-gated test green.
- The binary needs the explicit `[[bin]]` entry in `Cargo.toml` (it lives in `src/bin/main.rs`).
- `loco-rs` has `default-features = false` and no `auth` feature (no `jsonwebtoken`/`rsa`).
- Gates: `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --lib`; then
  the DB-gated tests (see the repository `AGENTS.md`).
- Every new endpoint must be audited with `audit::append` before it returns data (see `compare_audited`) and policy-checked (SE-1); until SE-1 exists, do
  not add an endpoint that reads stored patient data.
