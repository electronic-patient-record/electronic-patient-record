# Compatibility: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

## Definition

ISO/IEC 25010:2023 defines compatibility as the degree to which a product, system or component can exchange information with other products, systems or components, and/or perform its required functions while sharing a common hardware or software environment. Its two sub-characteristics are co-existence (performing efficiently alongside other products in a shared environment without harming them) and interoperability (exchanging information with other products and using the information exchanged) [1]. I saw an iso25000.com summary, not the standard's text (the ISO page gave HTTP 403) [2]. The 2011 edition had the same two sub-characteristics as I recall; I did not check. Interoperability has [its own document](../interoperability/index.md) here, so this note concentrates on co-existence and on the version and environment compatibility that surrounds it (browsers, databases, runtimes, API and FHIR versions).

The SEI view treats these as modifiability and integrability concerns: a change to one element should not force changes in others, and a scenario states which change, by whom, with what effort as the response measure [3]. Semantic versioning gives a public rule for "what breaks a consumer" [4].

## Why it matters for an EPR

An EPR is never alone in a hospital. It shares networks, identity providers, databases and endpoint devices with dozens of systems, and one shared dependency can stop all of them. The July 2024 global software-update fault disrupted hospitals in several countries that used different EHR products [5], and the October 2022 Oracle Cerner outage affected trusts nationally [6] (both seen as snippets only). Those are reliability events, but they show the co-existence principle: the environment the product shares is part of its quality.

Version compatibility is the second pressure. Hospitals run browsers and operating systems on managed schedules, and integrations run for years. FHIR itself separates releases by a `fhirVersion` MIME parameter and says it is an error to use two versions in one interaction [7]; HL7 v2 messages from sites vary by local Z-segments and site rules, so a receiver cannot assume a message is "standard" [8].

## What the project says today

- **API versioning** is the `Accepts-version` request header on the unversioned `/api` path; no version segment is allowed in a route (`AGENTS.md`, `spec/09-api-surface.md`).
- **FHIR releases.** R4 is stored; R5 is a read-only derived facade (`/fhir/R5`, IO-16, built 2026-10-29), where a loss or warning in conversion fails the response. `fhirVersion` is read; there is no status code for an unsupported version yet (IO-17 partly). R6 is watch-only (IO-15). The decision on where the version is chosen is open (PA-9, OQ-PA3).
- **Pinned platform.** Rust 1.99.1 pinned in `rust-toolchain.toml` with 1.97 as the minimum; Node 26, pnpm, Podman, PostgreSQL 18 for tests (`AGENTS.md`). Podman rather than Docker everywhere.
- **Co-existence in practice.** The guide records that ports 5432 and 6379 are often taken and gives override variables; `localhost` can resolve to IPv6 where another project listens, so tests use `127.0.0.1`; the UI test port is overridable (`PW_PORT`). These are developer-machine cases, not hospital cases.
- **Dependency coupling.** `axum-test` follows `axa` and must not be bumped alone (`AGENTS.md`); `cargo-deny` and Dependabot are in CI (NFR-11).
- **UI environment.** Browser support list is not stated in the specs I read; mobile viewability is proposed (NFR-4) and nothing is built. A stateless UI tier depends on `EPR_SESSION_REDIS_URL` (OS-8, built, not run behind two replicas).
- **Not built:** a deprecation policy for the `Accepts-version` values, a published support matrix, rolling-upgrade tests (old and new service replicas against one schema), a migration compatibility check, container image scanning (CQ-23, CQ-27).

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
|---|---|---|---|
| Supported-environment matrix coverage | Table of browsers, OS, Postgres and Redis versions, with the CI job that exercises each | Every row has a job; a row with none is labelled "untested" | not built; proposal below |
| Breaking API change count | Diff of the OpenAPI document between releases, classified by a diff tool | Zero breaking changes inside one `Accepts-version` value | `/api/openapi.json` freshness gate (OS-7) |
| Capability statement truth | Census: stated interactions equal mounted routes | Zero mismatch | IO-1 census test (built) |
| FHIR round-trip loss | Resources converted R4 to R5 with warnings or loss | Zero silent loss; loud failure | IO-16 tests (built); IO-17 |
| Shared-resource footprint | CPU, memory, connections, ports, file handles per replica under load | Fit the smallest named host; connections below the pool cap | not measured |
| Upgrade compatibility | Old binary against new schema and the reverse during a rolling deploy | Pass for one release back | not built |
| Dependency drift | Count of dependencies behind latest by major version | Patch and minor within a set time; majors planned | Dependabot, `cargo-deny` |
| Browser matrix | Playwright projects per engine, plus axe | Chromium, Firefox, WebKit at least; widen by usage data of deployers | Playwright config (check which projects exist) |

## Trade-offs with other attributes

- **Maintainability and flexibility.** Supporting many versions slows change; pinning (toolchain, `axa` and the `axum-test` that follows it) helps maintainability but narrows compatibility ([maintainability](../maintainability/index.md), [flexibility](../flexibility/index.md)).
- **Security.** Old browsers, TLS versions and libraries are the usual price of wide compatibility; the NFR-6 floor of TLS 1.2 is a choice ([security](../security/index.md)).
- **Performance.** Derived facades (R5 from R4) add conversion cost per call ([performance efficiency](../performance-efficiency/index.md)).
- **Interaction capability.** Wide device support, including low-end phones, serves the equity rule and constrains the UI ([interaction capability](../interaction-capability/index.md)).
- **Interoperability.** Narrow, tested conformance claims are more honest than broad ones ([interoperability](../interoperability/index.md)).

## Gaps and risks

1. No published support matrix, so deployers cannot tell what works.
2. No deprecation or sunset policy for API versions.
3. Rolling upgrade is untested; migrations are append-only but their compatibility with a running old binary is not checked.
4. A pinned toolchain with a "minimum supported" version needs a job that actually builds on the minimum, or the minimum is a claim (I did not check CI).
5. Shared environment risks (a port held by another process, a shared Redis) are met only in development.
6. A facade for R5 may drift from the specification as R5 errata appear.

## Ideas

1. Write `docs/compatibility/matrix` as a generated table from CI job names, with "untested" rows (extends `scripts/generate-status.sh`).
2. Add an OpenAPI breaking-change check to the OS-7 freshness gate.
3. Add a CI job that builds on the minimum Rust version.
4. Add a rolling-upgrade test: migrate with the new binary, run the old test suite against it.
5. State a deprecation rule for `Accepts-version` values (for example, a minimum notice) and put it in `spec/09-api-surface.md`; needs maintainer decision.
6. Return a defined error for an unsupported `fhirVersion` (IO-17) once the FHIR rule is checked.
7. Add the Playwright browser projects and record them in the matrix.

## Open questions

- Which browsers and OS versions do target deployers manage (hospital trust estates, low-resource clinics)?
- Is the UI expected to run on tablets that are several OS versions behind?
- How many API versions are supported at once, and who decides when one ends?

## Not verified

- ISO/IEC 25010:2023 text and the 2011 comparison.
- The outage reports (snippets only).
- Which Playwright browser projects and CI Rust versions exist; I did not open `ci.yml` or the Playwright config.
- Nothing here was measured.

## Sources

1. https://iso25000.com/index.php/en/iso-25000-standards/iso-25010 : compatibility, co-existence, interoperability definitions (summary).
2. https://www.iso.org/standard/78176.html : ISO catalogue page (403 on fetch).
3. https://people.ece.ubc.ca/matei/EECE417/BASS/ch04lev1sec3.html : quality attribute scenarios (Bass et al.).
4. https://semver.org/ : semantic versioning rule (cited from memory; not fetched in this session).
5. https://ukauthority.com/articles/nhs-systems-recover-from-global-it-outage : 2024 global outage and UK health service recovery (snippet).
6. https://www.theregister.com/2022/10/12/oracle_cerner_outage_uk/ : Oracle Cerner UK outage, October 2022 (snippet).
7. https://hl7.org/fhir/R4/http.html : `fhirVersion` parameter and CapabilityStatement rules.
8. https://www.hl7.org/fhir/R4B/comparison-v2.html : HL7 v2 variation and Z-segments (snippet).
