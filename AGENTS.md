# AGENTS.md — Electronic Patient Record

Entry point for AI coding agents at the repository root. Three projects live here; each has its own `AGENTS.md`.

| Project | Directory | Guide |
| --- | --- | --- |
| Service (loco.rs REST + FHIR API, PostgreSQL) | `electronic-patient-record-service-with-rust` | [AGENTS.md](electronic-patient-record-service-with-rust/AGENTS.md) |
| Matcher (pure Rust library, patient identity) | `electronic-patient-record-matcher-with-rust` | [AGENTS.md](electronic-patient-record-matcher-with-rust/AGENTS.md) |
| Clinician UI (SvelteKit, Svelte 5, backend-for-frontend) | `electronic-patient-record-ui-with-svelte` | [AGENTS.md](electronic-patient-record-ui-with-svelte/AGENTS.md) |

Version 0.1.0. Rust 1.99.1 (pinned in `rust-toolchain.toml`; minimum supported 1.97), Node 26, pnpm, Podman.
**NOT FOR CLINICAL USE.** This is a scaffold (milestone M0): no clinical record exists yet.

## How work is done here

1. **Spec first.** `spec/` and each project's `spec/index.md` are the source of truth. A behavioural change is one
   commit with a spec edit, a code edit and a test edit ([`agents/spec-driven-development.md`](agents/spec-driven-development.md)).
2. **One item, one commit, one push**, then look at CI. Stage named files, never `git add -A`.
3. **Say plainly what is not built or not tested.** Every change to `CHANGELOG.md` states what was not done or not
   verified. Never weaken a guard or a census test to get green; fix the cause or record why.
4. **Clinical risk.** If a change could alter what a clinician sees, decides or signs, say so and update `governance/clinical-safety/hazard-log.csv` (task AS-1) in the same commit; a `clinical-impact` PR without that change fails CI. Never set a hazard to `accepted` or `closed`: only the Clinical Safety Officer does. When unsure, treat it as clinical.
5. **Run the gates before pushing** (below), and `sh scripts/generate-status.sh` after adding tests or ticking tasks, and
   `python3 scripts/generate-llms.py` after adding or moving a document it lists (`llms.txt`, `llms.json`), or the docs job fails.

## Gates

```bash
# matcher and service
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test
# service database tests need Postgres 18 (ports 5432/6379 are often taken: override them)
#   TEST_DB_PORT=5543 TEST_REDIS_PORT=6390 podman compose -f electronic-patient-record-service-with-rust/compose.test.yaml up -d --wait
#   DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test cargo test --test audit_immutable -- --ignored --test-threads=1
# UI (another project may hold port 4173: use PW_PORT)
cd electronic-patient-record-ui-with-svelte && pnpm run lint && pnpm run check && pnpm run check:tests && pnpm exec vitest run && PW_PORT=4183 CI=true pnpm exec playwright test
# docs
python3 scripts/check-links.py . && sh scripts/generate-status.sh --check && python3 scripts/generate-llms.py --check
```

## Things that are easy to get wrong

- API versioning is the `Accepts-version` request header on the unversioned `/api` path. Never add `/api/v1`, `/api/v2` or any
  version segment to a route ([`spec/09-api-surface.md`](spec/09-api-surface.md)).
- The crate and binary are `electronic-patient-record`; the migration crate is `electronic-patient-record-migration`;
  the directories keep their `-with-rust` / `-with-svelte` names. The service's `src/bin/main.rs` needs the explicit
  `[[bin]]` entry in `Cargo.toml`.
- `localhost` can resolve to IPv6 `::1` where another project is listening. When curling a local service, use
  `127.0.0.1`.
- The matcher's `Patient` is the API identity DTO. Do not add an adapter type; change the matcher and its spec.
- The clinical record is append-only. There is no delete path for clinical data and no migration may add one;
  `audit_events` is protected by database triggers.
- Absent evidence is `null` with a reason, never zero. A missing field blocks an automatic patient link; it never
  counts as agreement.
- SvelteKit 3: environment variables are declared in `src/env.ts` with `defineEnvVars` and read from
  `$app/env/private`, not `$env/dynamic/private`. Computed styles use Svelte's `style:` directive; the CSP forbids
  inline `style=""`.
- `axum-test` and `jsonwebtoken` follow `loco-rs`; do not bump them alone.
- Run `sh scripts/install-hooks.sh` once per clone: it makes git run `.githooks/pre-commit` (UI lint, `cargo fmt --check`, generated docs, links; only what the staged files need). Skip once with `--no-verify`; CI runs everything either way.
- Use Podman (`podman build`, `podman compose`), not Docker, everywhere: instructions, scripts and CI. Container files are named `Containerfile` and `.containerignore`.
- The Podman VM's clock drifts (after sleep, say). Keycloak then issues tokens that are already expired by the host's clock, and the API answers
  401 `keycloak: … InvalidSignature` (the verifier keeps only the last key's error, so expiry looks like a bad signature). Compare `date -u +%s`
  with `podman exec <container> date -u +%s`; reset with `podman machine ssh "sudo date -u -s @$(date -u +%s)"`.
- Translations are machine-written until a speaker reviews them. Never claim a translation is reviewed.
