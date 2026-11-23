# Electronic Patient Record

Open-source electronic patient record software: a safe, append-only, audited longitudinal patient record,
with FHIR R4 as both data model and API, and explainable patient identity matching.

> **Pre-release scaffold. Not for clinical use. Do not enter real patient data.** There is no clinical record yet;
> see [`spec/14-implementation-status.md`](spec/14-implementation-status.md) for exactly what is built, and
> [the path to production](governance/production-readiness.md) for what production means and what must be done first.

Project site: <https://electronic-patient-record.github.io> (source: [electronic-patient-record.github.io](https://github.com/electronic-patient-record/electronic-patient-record.github.io)).

| Subproject | What it is | Run | Test |
|---|---|---|---|
| [`electronic-patient-record-matcher-with-rust`](electronic-patient-record-matcher-with-rust) | Rust library: the `Patient` identity type and the explainable matching engine | `cargo run -- a.json b.json` | `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check` |
| [`electronic-patient-record-service-with-rust`](electronic-patient-record-service-with-rust) | Axum and `axa` service; the API identity DTO is the matcher's `Patient` | `cargo run -- start` (needs Postgres) | `cargo test --lib`; `cargo test --test audit_immutable -- --ignored` (needs Postgres) |
| [`electronic-patient-record-ui-with-svelte`](electronic-patient-record-ui-with-svelte) | SvelteKit 3, Svelte 5 UI with a backend-for-frontend proxy | `pnpm install && pnpm dev` | `pnpm run check`, `pnpm exec vitest run`, `pnpm exec playwright test` |
| [`electronic-patient-record-mobile-app-with-swift`](electronic-patient-record-mobile-app-with-swift) | iOS app: Swift 6, SwiftUI, Lily Design System for Swift; a scaffold | `sh scripts/fetch-lily.sh`, then open `App/ElectronicPatientRecord.xcodeproj` | `swift test`; the iOS Simulator build |
| [`electronic-patient-record-mobile-app-with-kotlin`](electronic-patient-record-mobile-app-with-kotlin) | Android app: Kotlin, Jetpack Compose, Lily Design System for Kotlin; a scaffold | `sh scripts/fetch-lily.sh && ./gradlew :app:installDebug` | `./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug` |

The Rust toolchain is pinned in [`rust-toolchain.toml`](rust-toolchain.toml) (1.100.0). The UI needs Node 26 and
pnpm; its browser tests stub the backend, so they do not need the service.

## Run the API from a fresh clone

`compose.yaml` runs the API with its database and cache. Login is enforced, so the API needs the public keys of
whatever issues your tokens; without them it refuses to start.

```sh
POSTGRES_PASSWORD=change-me EPR_PASETO_KEYS_URL=https://auth.example.com/.well-known/paseto-keys \
  podman compose up -d --build --wait
curl -fsS http://localhost:1241/_health/live
```

Keycloak (development) and standalone PostgreSQL/Redis under Podman: [`operations/keycloak/README.md`](operations/keycloak/README.md).

## Where things live

- [`spec/`](spec/index.md): the living specification (start at `spec/index.md`).
- [`plan.md`](plan.md): the research and decisions (standards, products, evidence, architecture, milestones).
- [`tasks.md`](tasks.md): the backlog of record; nothing beyond M0 scaffolding is ticked.
- [`agents/`](agents/index.md): guidance for AI agents. Each subproject has an `AGENTS.md`.
- [`LICENSES/`](LICENSES): the licence texts, one `<id>/index.md` each.

## About the project

[Contributing](CONTRIBUTING.md) · [Governance](GOVERNANCE.md) · [Maintainers](MAINTAINERS.md) ·
[Security](SECURITY.md) · [Changelog](CHANGELOG.md) · [News](NEWS.md) · [Request for comments](RFC.md) ·
[AI statement](AI_STATEMENT.md) · [How to cite](CITATION.cff)

## Licence

Any of these, or contact us for custom terms. [`LICENSE.md`](LICENSE.md) explains the choice.

* [MIT](LICENSES/mit/index.md) · [Apache-2.0](LICENSES/apache-2.0/index.md) · [BSD-3-Clause](LICENSES/bsd-3-clause/index.md) ·
  [GPL-2.0-only](LICENSES/gpl-2.0-only/index.md) · [GPL-3.0-only](LICENSES/gpl-3.0-only/index.md) ·
  [OGL-UK-3.0](LICENSES/ogl-uk-3.0/index.md) · [CC-BY-NC-SA-4.0](LICENSES/cc-by-nc-sa-4.0/index.md)

SPDX expression: `MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only OR OGL-UK-3.0 OR CC-BY-NC-SA-4.0`.
