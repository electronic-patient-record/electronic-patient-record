# Benchmark against open-source EPR and EHR systems (2026-10-30)

How this repository stands against the field on **code quality, maintainability, security and scalability**, and what
the field offers in **capabilities and configuration**. Grounded on both sides: every "us" cell names a file here, every
"them" cell names a confirmed file or page there. Read with plan.md §22 (the assessment) and the `OS-` tasks in
[`../tasks.md`](../tasks.md). Mode: a standard scan (twelve comparators, searched until two query rounds added none).
Star counts and push dates are exact from the GitHub API on 2026-10-30; nothing was cloned.

**Verdict.** Engineering hygiene inside the repository (lint, census tests, audit refusal, provenance, SBOM, hand-run
mutation testing) is at or above the field. What the field has and we do not is **operational**: coverage and static
analysis published from CI, rate limiting, a background job runner, published performance numbers, tracing, an OpenAPI
document, a shared session store, and an extension mechanism. Functionally the repository is an M0–M1 scaffold and the
field is not; that gap is the roadmap, not a finding.

**Tier: LAGGING** (14 of 25 table-stakes items missing or partial) · **Coverage** `████░░░░░░` 44% (11/25 table-stakes met)
· **Field:** 12 repositories · **Since last scan:** coverage 44% to 44% (11/25, none of the partials became met), tier LAGGING unchanged, 14 gaps unchanged; three table-stakes items went from missing to partial (published coverage, SAST, performance numbers) and background jobs went from missing to partial, so 3 are still missing (SMART app launch, rate limiting, extension mechanism); two edge items are newly met (secret scanning, code of conduct); rubric version 1 unchanged, field not re-fetched (re-scored from repository evidence on 2026-10-30). Scan record:
`.sota/last-scan.json`; the fixed scoring axis: `.sota/rubric.open-source-ehr-engineering.json`.

## Field framing

- **Detected domain:** open-source electronic patient record / clinical data platform with a FHIR facade.
- **Adjacent domains considered:** FHIR servers (HAPI, Blaze, Haste, Helios) and openEHR repositories (EHRbase), because
  the service is also a FHIR facade; hospital information systems (Bahmni, Ozone, GNU Health, VistA) for capabilities.
- **Excluded:** proprietary EHRs (no code to read); data-pipeline tools (`google/fhir-data-pipes`); SDK-only crates.
- **Benchmark assumption:** the engineering bar is set by the actively pushed comparators; archived ones (HospitalRun)
  and off-GitHub ones (GNU Health) inform capabilities only.

## The field

| # | Repository | Type | Stars | Last push | Why it is in |
|---|---|---|---|---|---|
| 1 | [openemr/openemr](https://github.com/openemr/openemr) | popular | 5,559 | 2026-10-30 | most-starred OSS EHR; ONC-certified; SMART 2.2, `$export`, 30 advisories, Inferno in CI |
| 2 | [medplum/medplum](https://github.com/medplum/medplum) | popular | 2,772 | 2026-10-30 | FHIR-native platform; AccessPolicy, bots, OTel, rate limits, SOC 2 and ONC listing |
| 3 | [hapifhir/hapi-fhir](https://github.com/hapifhir/hapi-fhir) (+ [jpaserver-starter](https://github.com/hapifhir/hapi-fhir-jpaserver-starter)) | canonical | 2,424 (544) | 2026-10-30 | the reference FHIR server; partitioning, bulk export, batch jobs |
| 4 | [openmrs/openmrs-core](https://github.com/openmrs/openmrs-core) | canonical | 1,957 | 2026-10-29 | the reference modular EMR platform; OWASP gate, Scorecard, Initializer config |
| 5 | [samply/blaze](https://github.com/samply/blaze) | technically advanced | 231 | 2026-10-30 | strongest supply chain in the field: cosign, provenance, Trivy, Jepsen, codecov gate |
| 6 | [ehrbase/ehrbase](https://github.com/ehrbase/ehrbase) | canonical | 387 | 2026-10-29 | the open openEHR CDR; JaCoCo + SonarCloud + JMeter in every build |
| 7 | [metriport/metriport](https://github.com/metriport/metriport) | technically advanced | 795 | 2026-08-28 | production interoperability platform: WAF, DynamoDB rate limits, SQS pipelines |
| 8 | [HasteHealth/HasteHealth](https://github.com/hastehealth/hastehealth) | technically advanced | 84 | 2026-10-29 | Rust FHIR CDR: pedantic clippy, daily cargo-audit, Trivy, built-in OIDC and SMART |
| 9 | [HeliosSoftware/hfs](https://github.com/HeliosSoftware/hfs) | technically advanced | 57 | 2026-10-30 | Rust FHIR server: llvm-cov to codecov, Inferno, Playwright + axe, cluster smoke test |
| 10 | [earthians/marley](https://github.com/earthians/marley) (was frappe/health) | niche-relevant | 552 | 2026-10-29 | ERP-integrated HIS; DocType configuration, semantic-release, Crowdin |
| 11 | [WorldVistA/VistA](https://github.com/WorldVistA/VistA) (+ VistA-M) | canonical, pre-cloud | 143 | 2026-10-28 | the full-hospital reference; 166 packages; quality tooling is offline CTest |
| 12 | [HospitalRun/hospitalrun-frontend](https://github.com/HospitalRun/hospitalrun-frontend) | stale-reference | 6,889 | 2023-01-09, archived | what a community EHR UI did in 2020–22; not a bar |

Also read for capabilities only: Bahmni and Ozone HIS (OpenMRS distributions) and GNU Health (source on Codeberg).

## Engineering matrix

"Us" cites this repository; "SOTA" names who demonstrates it, with the confirmed file. Gap: **table-stakes** when most
active comparators have it, *edge* otherwise.

| Capability | Us | SOTA (who has it) | Gap? | Reference |
|---|---|---|---|---|
| Lint and format in CI | ✅ `ci.yml` jobs `matcher`, `api` (`clippy -D warnings`), `ui` (eslint, prettier, svelte-check) | all twelve | | Haste `test_units.yml` runs `clippy -D clippy::pedantic` |
| Unit and integration tests in CI | ✅ `ci.yml` `api`, `api-db` (Postgres 18), `ui`; 426 service tests, 96 matcher, 213 vitest, 81 Playwright | all | | Blaze `build.yml` ~20 named integration jobs |
| Coverage measured and published | ❌ no tarpaulin, llvm-cov or c8 anywhere | OpenMRS, OpenEMR, Medplum, EHRbase, Blaze, Helios, Marley | **table-stakes** | Helios `ci.yml` `coverage` job: `cargo-llvm-cov` → codecov; `codecov.yml` thresholds |
| SAST in CI | ❌ none (CQ-25 lists CodeQL) | OpenMRS, OpenEMR (semgrep), Medplum, HAPI, EHRbase, Haste (Sonar), Marley | **table-stakes** | OpenMRS `.github/workflows/codeql-analysis.yml`; OpenEMR `semgrep.yml` with SARIF upload |
| Dependency vulnerability scan in CI | ✅ `ci.yml` `deny` (cargo-deny advisories); ✅ since 2026-10-30 one `RUSTSEC` ignore with a reason and an enforced expiry (12 dead ones removed; `tests/advisory_expiry.rs`); `pnpm audit` added in CI (OS-4) | OpenMRS (OWASP `--failOnCVSS 6.2`), Haste (daily `audit_cargo.yml`), Blaze (OSV, Trivy) | | OpenMRS `dependency-check.yml` + `dependency-check-suppressions.xml` |
| Vulnerability disclosure policy | ⚠️ `SECURITY.md` exists but says break-glass and authorization are "not built" and that only compare writes audit | OpenMRS (7 advisories), OpenEMR (30), Medplum, EHRbase, Blaze, Haste | **table-stakes** | OpenEMR `.github/SECURITY.md` + GHSA list in `CHANGELOG.md` |
| Automated dependency updates | ✅ `.github/dependabot.yml` (cargo, npm, actions; no docker ecosystem) | most | | Blaze `renovate.json` `config:best-practices` with digest pinning |
| Hardened container | ⚠️ non-root `USER epr` / `USER node`; no digest pins; `debian:13-slim`, not distroless; no image scan (CQ-23, CQ-27) | Blaze (digest-pinned, `USER 1001`, Trivy job), HAPI starter (distroless `nonroot`), Medplum (hardened image, digest) | **table-stakes** | Blaze `Dockerfile` + `build.yml` `image-scan` |
| OIDC authentication | ✅ `src/auth.rs` Keycloak OIDC and PASETO v4; refuses to start without keys | OpenEMR, Medplum, EHRbase, Haste, Helios | | Haste `crates/server/src/auth_n/{oidc,mfa,session}` |
| SMART App Launch | ❌ planned (IO-2) | OpenEMR, Medplum, OpenMRS (module), Haste, Helios | **table-stakes** | OpenEMR `Documentation/api/SMART_ON_FHIR.md` + `smart-launch-test.yml`; Haste `/.well-known/smart-configuration` in `crates/server/src/server.rs` |
| Access policy with audit of reads and writes | ✅ `src/policy.rs` (RBAC + ABAC, deny by default), `src/audit.rs` keyed hash chain, write refused when unaudited (`tests/http_audit_refusal.rs`) | Medplum (`accesspolicy.ts`), OpenEMR (`EventAuditLogger.php`), Helios (BALP) | | we exceed the field: nobody refuses a write on audit failure |
| Rate limiting and brute-force protection | ⚠️ `src/rate_limit.rs`: per-subject fixed window in the process, compare limited apart, 429 with `Retry-After`; no address key, no lockout, not shared between replicas | Medplum, Metriport, Haste, OpenEMR, OpenMRS (lockout), Marley | **table-stakes** | Medplum `packages/server/src/ratelimit.ts` (Redis); Haste `crates/rate-limit/src/lib.rs` |
| Security headers and CSP | ✅ UI `src/lib/security-headers.ts`, nonce CSP in `vite.config.ts` | Medplum `app.ts`, Haste `middleware/security_headers.rs` | | Haste sets them on the API too; our API sets none (`config` cites a missing `http_security.rs`) |
| Horizontal scaling (stateless, shared session) | ⚠️ service stateless; UI session store is memory by default and, since 2026-10-30, an encrypted Redis backend when configured (`src/lib/session-backends.ts`), unproven behind two real replicas | Medplum (Redis sessions), OpenEMR (`LockingRedisSessionHandler.php`), Helios (`cluster-smoke.yml`), Metriport (ECS) | **table-stakes** | OpenEMR `src/Common/Session/Predis/`; Helios `cluster-smoke.yml` |
| Database pooling configured | ✅ `config/production.yaml` min 2 / max 20 | most | | Helios `HFS_PG_MAX_CONNECTIONS`, `pool_metrics.rs` |
| Background jobs / outbox | ⚠️ `src/outbox.rs` and migration 17 (append-only events and attempts, policy and consent checked before a sink); no sink, no loop, no producer, no worker in `app.rs` | Medplum `workers/`, HAPI Batch2, Blaze `job-*`, Metriport SQS, Haste `crates/worker`, Helios | **table-stakes** | Haste `crates/worker/src/async_jobs` + `20261014120000_async_jobs.sql` |
| Pagination limits | ✅ fixed caps with `limit+1` and a `before` cursor (`patients_store.rs`, `controllers/audit.rs`) | Medplum (`DEFAULT_MAX_SEARCH_COUNT=1000`), Metriport (max 500) | | Metriport `docs/medical-api/handling-data/pagination.mdx` |
| Performance benchmarks or load tests | ❌ none; `BENCHMARKS.md` does not exist; CQ-18 needs it | EHRbase (JMeter in CI), Blaze (`docs/performance.md`), Helios (`fhir-benchmark.yml`), Haste, OpenMRS (`-Pperformance-test`), Medplum | **table-stakes** | EHRbase `tests/perf/*.jmx` + `build_and_test.yml` `performance-test-run` |
| Observability (metrics, tracing, health) | ⚠️ three counters and, since 2026-10-30, a request-duration histogram labelled by route template and status at `/metrics` (`src/metrics.rs`); `/_readiness` pings database, queue and cache (loco); no OTLP (NFR-8 promises it; OQ-OS4) | Medplum `otel/`, HAPI starter (OTel agent, micrometer), Helios `crates/observability`, Haste (OTLP + Sentry) | **table-stakes** | Helios `crates/observability/src/{metrics,telemetry,pool_metrics}.rs` |
| API docs (OpenAPI and CapabilityStatement) | ⚠️ `/fhir/R4/metadata` with a census test; since 2026-10-30 an OpenAPI inventory derived from the routes at `/api/openapi.json` with a freshness census (`src/openapi.rs`, `tests/openapi_census.rs`) but no body schemas | OpenEMR (`swagger/openemr-api.yaml` + freshness gate), Medplum `openapi.ts`, HAPI, EHRbase (springdoc), Haste (`/openapi.json`) | **table-stakes** | OpenEMR `.github/workflows/api-docs.yml` fails when the document is stale |
| Architecture documentation current | ⚠️ `spec/08-architecture.md` lists tantivy, an outbox and row-level security, none in `Cargo.toml` or the migrations | Medplum, HAPI `server_jpa/architecture.md`, Blaze `docs/architecture.md`, Helios mdBook | **table-stakes** | Blaze `docs/architecture.md` |
| Release process and changelog | ✅ `release.yml` (tag equals versions, SBOM, provenance), root `CHANGELOG.md` | all | | Blaze `DEVELOPMENT.md` release steps |
| i18n infrastructure | ⚠️ ten locales routed; strings English only; one machine-written form (`translations.ts`) | OpenEMR (34 languages, RTL), OpenMRS (Transifex), Helios (Fluent `locales/`), Marley (Crowdin), HospitalRun (13 locales) | **table-stakes** | Helios `locales/{en,es,de}/main.ftl` |
| Pinned toolchain | ✅ `rust-toolchain.toml` 1.99.1, checked by the `fresh-clone` job | Haste, Medplum (`packageManager` with hash) | | |
| Extension mechanism | ❌ none; packs are planned (IO-18, IO-19) | OpenMRS modules + Initializer, OpenEMR `oe-module-*`, Medplum bots, HAPI interceptors, EHRbase plugins, Frappe apps | **table-stakes** | OpenMRS Initializer `readme/*.md` domains |
| Image signing, provenance, SBOM | ✅ `release.yml`: `attest-build-provenance`, CycloneDX for three projects checked by `check-sbom.py`; no cosign | Blaze only (cosign + provenance, no SBOM) | *edge* | we lead; Blaze `build.yml` `push-image` for cosign |
| Secret scanning | ✅ since 2026-10-30 `ci.yml` job `secrets` (gitleaks over full history, binary pinned by checksum), unrun on GitHub | nobody else in CI; Metriport at pre-commit (`.husky/pre-commit` gitleaks) | *edge* | we lead; Metriport `.husky/pre-commit` for the hook |
| Property and fuzz tests | ⚠️ hand-rolled seeded property test (`matcher/tests/properties.rs`), seeded hostile-input sweep (`tests/chaos_api.rs`); no `fuzz/` | Medplum (fast-check), Blaze (test.check, Jepsen), Haste (proptest), EHRbase (quickcheck) | *edge* | Haste `crates/fhir-search/proptest-regressions/` |
| Browser end-to-end with accessibility | ✅ Playwright + axe (`tests/e2e/smoke.spec.ts`, `axe-route-coverage.test.ts`) | Helios (Playwright + axe-core), Blaze, Medplum, OpenEMR | | |
| Bulk export | ❌ planned (IO-4) | OpenEMR, Medplum, HAPI, Helios | *edge* here, required by ONC (g)(10) | Helios `$export`, `$bulk-submit` |
| Audit retention and partitioning | ❌ `governance/retention-schedule.md` is a draft that keeps everything | HAPI `server_jpa_partitioning/`, EHRbase `data-retention` key, Blaze `job-prune` | *edge* | HAPI starter `partitioning` keys |
| Encryption-at-rest guidance | ⚠️ NFR-6 states AES-256; no deployer guidance | OpenEMR `src/Encryption/`, Haste `[security.encryption]` providers | *edge* | Haste `backend/documentation/server_configuration.md` |
| Conformance tests in CI | ⚠️ HL7 validator job; Inferno planned | OpenEMR (`inferno-test.yml`), Helios (`inferno-*.yml`), Haste (g10 TestScripts) | *edge* | OpenEMR `inferno-test.yml` |
| Identity matching | ✅ explainable matcher, absent is not zero, `$match` | HAPI MDM, Medplum `$match`, OpenMRS `mergePatients` | | we lead on explainability |
| Mutation testing | ✅ by hand (`matcher/tests/fields.rs` records killed mutants); not in CI | nobody | | |
| Census and tripwire tests | ✅ `route_census.rs`, `error_census.rs`, `sql_census.rs`, `fhir_census.rs`, `personal_data_inventory.rs`, support-only tripwire | nobody | | |
| In-product hazard register | ✅ `src/hazard.rs`, append-only, CSO-only retirement | nobody | | |
| CONTRIBUTING and code of conduct | ✅ `CONTRIBUTING.md`, `GOVERNANCE.md`, `CODE_OF_CONDUCT.md` | EHRbase, OpenEMR, HospitalRun; Medplum, HAPI, Blaze, Haste have none | *edge* | EHRbase `CODE_OF_CONDUCT.md` |

## Gaps, worst first

Effort is an estimate. Confidence: high when both sides were read in code or docs.

1. **Rate limiting and brute-force protection** (table-stakes, ~2 days, high). The config files promise it and nothing
   provides it; a login endpoint and a `$match` endpoint without a limit are an enumeration risk. Study Medplum
   `packages/server/src/ratelimit.ts` (Redis-backed, per subject) and Haste `crates/rate-limit/src/lib.rs` (a trait with
   a database store). First step: create `src/rate_limit.rs` with a `tower` layer keyed by subject or client address,
   backed by the Redis the config already names, mounted in `app.rs` after `request_id`; test in `tests/http_rate_limit.rs`
   that the 51st request in a minute is 429 with `Retry-After` and that the refusal is audited. **OS-2.**
2. **Background job runner and transactional outbox** (table-stakes, ~3 days, high). `spec/08` promises an outbox for
   subscriptions, webhooks, CDS Hooks and HL7 v2; `config` names a queue; `app.rs` registers nothing. Study Haste
   `crates/worker/src/async_jobs` with its `async_jobs.sql`, and Medplum `packages/server/src/workers/`. First step: a
   migration for `outbox_events` (append-only, with a `delivered_at`), `src/outbox.rs` with `enqueue(txn, event)` called
   inside the same transaction as the audit row, and one loco worker registered in `connect_workers`. **OS-3.**
3. **Coverage, SAST and audit published from CI** (table-stakes, ~1 day, high). Study Helios `ci.yml` (`cargo-llvm-cov`
   to codecov with `codecov.yml` thresholds) and OpenMRS `codeql-analysis.yml`. First step: a `coverage` job in
   `ci.yml` running `cargo llvm-cov --workspace --lcov` and `vitest --coverage`, uploaded as an artefact; then
   `codeql.yml` for JavaScript and TypeScript, and `semgrep` or CodeQL for Rust once its Rust support is confirmed.
   Extends CQ-25. **OS-4.**
4. **Published performance numbers** (table-stakes, ~2 days, medium). The audit lock serialises writes (CQ-18) and
   nobody has measured it. Study EHRbase `tests/perf/*.jmx` run in every build and Helios `fhir-benchmark.yml`. First
   step: `scripts/load/audited-writes.js` for k6 against compose, recorded in `BENCHMARKS.md` with the commit and
   machine; a CI job on pushes to `main` that stores the result as an artefact. Closes AS-6's first half. **OS-5.**
5. **Tracing and labelled metrics** (table-stakes, ~2 days, high). Study Helios `crates/observability/src/` and Medplum
   `otel/`. First step: `tracing-opentelemetry` with an OTLP exporter behind `EPR_OTLP_ENDPOINT`, a request-duration
   histogram labelled by route and status, and a readiness probe that checks the database. **OS-6.**
6. **OpenAPI document with a freshness gate** (table-stakes, ~2 days, high). Study OpenEMR `api-docs.yml` (CI fails
   when the committed document differs from the generated one) and Haste `crates/openapi-schema-generator`. First
   step: `src/openapi.rs` with `utoipa`, served at `/api/openapi.json`, and a census test that every route in
   `all_routes()` appears in it. **OS-7.**
7. **Shared session store for the UI** (table-stakes, ~1 day, high). Study OpenEMR `LockingRedisSessionHandler.php`.
   First step: a `SessionStore` interface in `src/lib/session.ts` with the `Map` and a Redis implementation. **OS-8.**
8. **Stale documents and comments** (table-stakes, ~half a day, high). Six places describe code that does not exist or
   no longer holds. Study OpenEMR's freshness gate for the idea. First step: `tests/doc_census.rs` that every
   `src/*.rs` path named in `config/*.yaml`, `SECURITY.md`, `deny.toml` and `tasks.md` exists, then fix the six. **OS-9.**
9. **Extension mechanism** (table-stakes, ~design first, medium). Study OpenMRS Initializer (metadata as versioned
   CSV and JSON domains, loaded idempotently), OpenMRS O3 `defineConfigSchema` (typed defaults with validators), and
   HAPI's `*_enabled` keys. This is the pack mechanism of IO-18 and IO-19; the design should borrow all three. **OS-10.**
10. **i18n infrastructure** (table-stakes, ~2 days, medium). Study Helios `locales/*/main.ftl` (Fluent) and OpenEMR's
    34-language catalogue. CQ-34 already holds the task; the benchmark adds the plural-rule and RTL requirement.
11. **Container digest pins and image scan** (table-stakes, ~half a day, high). Study Blaze `Dockerfile` and `image-scan`.
    CQ-23 and CQ-27 hold the tasks.
12. **SMART App Launch** (table-stakes, ~1 week, high). IO-2 holds the task; study OpenEMR `smart-launch-test.yml`
    for the CI shape.
13. **Secret scanning in CI** (edge, ~1 hour, high). Nobody in the field does it in CI; a `gitleaks` job would lead.
    **OS-11.**
14. **cargo-deny ignores with no expiry** (edge, ~1 hour, high). Each of the 13 ignores needs a reason and a re-check
    date; a test fails when a date passes. **OS-12.**
15. **Audit retention and partitioning** (edge, ~design first, medium). Study HAPI `server_jpa_partitioning/` and
    EHRbase's `data-retention` key. Waits for the retention schedule (governance). **OS-13.**

## What we already match or exceed

Lint and format, tests in CI, dependency scanning, Dependabot, OIDC, policy with audit of reads and writes (and a write
refused when it cannot be audited, which nobody else does), CSP and headers on the UI, pooling, pagination caps, release
with provenance and SBOM (Blaze signs, nobody else attests and ships an SBOM), Playwright with axe, an explainable
matcher, census and tripwire tests, hand-run mutation testing, an in-product hazard register.

## Capabilities and configuration of the field

A summary of the survey (sources in plan.md §22.6). Cells: Y yes, P partial, N none found, ? unverified. "via O3" means
inherited from OpenMRS 3. Our column is from `spec/14-implementation-status.md` and `tasks.md`.

| Capability | Us (built) | OpenMRS | OpenEMR | Medplum | Marley | Bahmni / Ozone | GNU Health | EHRbase | HAPI starter | VistA |
|---|---|---|---|---|---|---|---|---|---|---|
| Registration, identity, merge | Y (matcher, merge with `Patient.link`) | Y / P merge | Y / P dupscore | P / Y `$match` | Y / ? | Y via O3 | Y / ? | P | P / Y MDM | Y MPI |
| Scheduling | N (PA-1 port) | Y | Y | P beta | Y | Y | Y | N | P | Y |
| Encounters, notes, forms | N (DO-*) | Y form builder | Y LBF forms | P Questionnaire | Y DocTypes | Y | Y | Y templates | P | Y TIU |
| Problems, allergies, vitals | allergies Y, vitals Y, problems N | Y | Y | P resources | Y | Y | Y | P archetypes | P | Y |
| Medications, interactions | N (OR-*) | P | P (Weno) | P vendors | Y | P | P | N | N | Y pharmacy, BCMA |
| Orders and results | N | Y | Y HL7 results | P | Y | Y OpenELIS, Orthanc | Y LIMS | N | N | Y CPRS, Lab |
| CDS | N (OR-6) | P flags | Y rules | Y CDS Hooks, `$apply` | P | P | ? | N | Y `cdshooks.enabled` | Y reminders |
| Inpatient, beds, triage | N (EM-*, TR-*) | Y ward app | N | N | Y | Y | Y | N | N | Y |
| Billing | out of scope | P | Y X12 | P | Y | Y Odoo | Y | N | N | Y |
| Portal | P (`/my-record` access log) | N | Y | P example | Y | ? | Y MyGNUHealth | N | N | Y |
| Consent, break-glass | Y, Y | ?, N | ?, Y emergency login | P, N | ?, N | ?, N | ?, N | N, N | P, N | ?, N |
| Audit of reads and writes | Y, chained | P writes | Y | Y | P | P | P signatures | P enterprise | N | Y Kernel |
| Append-only, versioning | Y | P flags | P | Y `_history` | P | P | P digests | Y | Y | ? |
| Documents, DICOM | photos only | P | P | P agent C-STORE | P | Y dcm4chee / Orthanc | Y Orthanc | N | P | Y Imaging |
| Downtime / offline | N (CA-7) | ? | N | N | N | P offline repo | N | N | N | N |
| FHIR | R4 read and write facade, R5 read | R4 module | R4, `$export`, `$docref` | R4, `$everything`, `$export`, Subscriptions | N | via module | P in development | N (openEHR) | DSTU2–R5 | N |
| SMART | N (IO-2) | module | Y 2.2 | Y | N | ? | N | N | N | N |
| HL7 v2 | ADT in, MLLP | P queue | Y ORM/ORU | Y agent | N | P | N | N | N | Y |
| CDA / IPS | N (IO-5) | N | Y CCDA | Y `$ccda-export`, `$summary` | N | ? | N | N | P IG | N |
| Terminology loading | fixtures; SNOMED id shape check | concept dictionary, OCL | ICD/SNOMED/RxNorm loaders | `$import`, `$expand` | code systems | snowstorm-lite | ICD packages | external TS | Lucene or remote TS | NDF |
| Configuration mechanism | `config/*.yaml` + `EPR_*` env | global properties + Initializer CSV/JSON + O3 config schemas | `globals` table + admin UI | `medplum.config.json` / Parameter Store | Settings DocType | `.env` layers + distro `configs/` | `trytond.conf` | `application.yml` | `application.yaml` `*_enabled` | FileMan site parameters, KIDS |
| Roles and privileges | policy table in code (`routes.rs`, `policy.rs`) | roles + privileges UI | php-gacl | AccessPolicy (data) | Role Permission Manager | Keycloak + OpenMRS | Tryton | USER/ADMIN | none | Kernel keys |
| Multi-tenancy | planned (`tenant_id`, RLS) | locations | multi-site | Projects | company | locations | centres | ? | partitioning | sites |
| Deployment artefacts | `compose.yaml`, two Containerfiles | Dockerfile, compose | Dockerfile, compose, k8s (devops) | compose, Helm, CDK, Terraform | frappe_docker, Helm | compose per app, Maven distro | Ansible, pip | Dockerfile, compose | Dockerfile, compose, Helm | shell scripts, Vagrant |
| Backup, sizing | N | ? | P | P guides | ? | Y scripts | Y control tool | P | N | ? |

### Configuration patterns worth copying

1. **Metadata as versioned data, loaded idempotently** (OpenMRS Initializer: concepts, roles, privileges, global
   properties, forms, MDM rules as CSV and JSON domains). This is the shape for the IO-18 packs.
2. **Typed configuration schemas with defaults and validators**, merged from several files, later wins, with an
   implementer tool to try overrides live (OpenMRS O3 `defineConfigSchema`).
3. **Access policy as data, not code** (Medplum AccessPolicy: per-resource criteria, read-only, hidden fields, write
   constraints in FHIRPath, parameterised per tenant). Our `routes.rs` table is code; the ABAC part could become data.
4. **Behaviour as a subscription whose target is a function** (Medplum bots): integrations are data, not forks. Pairs
   with the outbox (OS-3).
5. **One `*_enabled` key per optional feature**, overridable by environment (HAPI starter).
6. **Distribution as a child project** that overrides only configuration directories and composes one file per app
   (Ozone). A deployer should never fork.
7. **Two-level model**: the record's shape is uploaded (archetypes and templates), not migrated (EHRbase). Relevant to
   the triage pack and forms.
8. **One control tool** for install, update and backup (GNU Health `gnuhealth-control`).

## Not done / not verified

- Twelve comparators were read through the GitHub API and their docs, not cloned; a few cells are marked unverified
  (Medplum i18n; HAPI rate limiting and CSP; Blaze CSP; Helios fuzzing; Haste secret scanning). GitHub code search was
  rate-limited in three of the five sweeps, so some "missing" cells rest on manifests alone.
- Bahmni's wiki returns nothing to a non-JavaScript fetcher; its cells rest on `bahmni-docker` compose files.
- Certification and compliance claims (Medplum SOC 2 and ONC listing, Metriport HITRUST, OpenEMR ONC) were read from
  the repositories' own pages, not from the certifying bodies.
- Star counts and push dates are from one API call on 2026-10-30.
- The functional column for us says what is built, not what is tested end to end; `spec/14` is the authority.
- Nothing in this document changes behaviour; no hazard row.
