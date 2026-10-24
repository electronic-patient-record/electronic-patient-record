## 14. Implementation Status

**Not for clinical use.** The repository is still the M0 scaffold in its release status, but code now exists for patient
identity, consent, care relationships, allergies, photos, merge and unmerge, break-the-glass, an HL7 ADT feed and a hash-chained
audit trail. What is built and what is not is stated per feature in each project's `spec/index.md`
([service](../electronic-patient-record-service-with-rust/spec/index.md),
[matcher](../electronic-patient-record-matcher-with-rust/spec/index.md),
[UI](../electronic-patient-record-ui-with-svelte/spec/index.md)) and in the entries of [`CHANGELOG.md`](../CHANGELOG.md),
which also say what was not run. The table that stood here was hand-written and went stale; the counts below are generated.
Nothing has run on GitHub yet (CQ-20).

<!-- generated:start -->
<!-- This block is written by scripts/generate-status.sh; do not edit it by hand. -->

**Generated facts** (counted from the source tree by `scripts/generate-status.sh`;
no network, database or compilation; counts are source lines, not test runs):

| Fact | Value | Method |
|---|---|---|
| Service version | 0.1.0 | `version` in the service `Cargo.toml` |
| Matcher version | 0.2.0 | `version` in the matcher `Cargo.toml` |
| Front-end version | 0.1.0 | `version` in the front-end `package.json` |
| Migrations | 12 | files named `m2*.rs` in `migration/src` |
| Route registrations | 42 | `.add(` calls in `src/controllers/*.rs` |
| Service tests | 264, plus 82 `#[ignore]`d (database gated) | `#[test]` and `#[tokio::test]` lines; `#[ignore]` lines |
| Matcher tests | 96, plus 0 `#[ignore]`d | the same patterns in the matcher |
| Front-end unit tests (vitest) | 112 | `it(` / `test(` lines in `tests/unit` |
| Front-end end-to-end tests (Playwright) | 40 | `test(` lines in `tests/e2e` |
| Locales | 10 | entries of `LOCALES` in `src/lib/locales.ts` |

**Task boxes in [`tasks.md`](../tasks.md)** (ticked / open, by prefix):

| Prefix | Ticked | Open |
|---|---|---|
| SF | 5 | 4 |
| ID | 2 | 6 |
| AU | 1 | 2 |
| RC | 0 | 12 |
| SE | 3 | 9 |
| DO | 0 | 8 |
| OR | 0 | 9 |
| IO | 0 | 15 |
| CA | 0 | 8 |
| AS | 0 | 12 |
| CQ | 20 | 18 |
| TR | 2 | 11 |
| PN | 0 | 11 |
| EM | 0 | 16 |
| PA | 1 | 16 |
| EC | 0 | 13 |
| **Total** | **34** | **170** |

<!-- generated:end -->
