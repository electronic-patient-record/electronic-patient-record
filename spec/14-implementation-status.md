## 14. Implementation Status

**Not for clinical use.** The repository is still the M0 scaffold in its release status, but code now exists for patient
identity, consent, care relationships, allergies, observations, amendment requests, closed-loop referrals, photos, merge and
unmerge, break-the-glass, an HL7 ADT feed and a hash-chained audit trail. What is built and what is not is stated per feature in each project's `spec/index.md`
([service](../electronic-patient-record-service-with-rust/spec/index.md),
[matcher](../electronic-patient-record-matcher-with-rust/spec/index.md),
[UI](../electronic-patient-record-ui-with-svelte/spec/index.md)) and in the entries of [`CHANGELOG.md`](../CHANGELOG.md),
which also say what was not run. The table that stood here was hand-written and went stale; the counts below are generated.
CI runs on GitHub on each push; a run's result is on the repository's Actions page, not here.

<!-- generated:start -->
<!-- This block is written by scripts/generate-status.sh; do not edit it by hand. -->

**Generated facts** (counted from the source tree by `scripts/generate-status.sh`;
no network, database or compilation; counts are source lines, not test runs):

| Fact | Value | Method |
|---|---|---|
| Service version | 0.1.0 | `version` in the service `Cargo.toml` |
| Matcher version | 0.7.0 | `version` in the matcher `Cargo.toml` |
| Front-end version | 0.1.0 | `version` in the front-end `package.json` |
| Migrations | 29 | files named `*.sql` in `migrations` |
| Route registrations | 86 | `.add(` calls in `src/controllers/*.rs` |
| Service tests | 693, plus 176 `#[ignore]`d (database gated) | `#[test]` and `#[tokio::test]` lines; `#[ignore]` lines |
| Matcher tests | 110, plus 0 `#[ignore]`d | the same patterns in the matcher |
| Front-end unit tests (vitest) | 304 | `it(` / `test(` lines in `tests/unit` |
| Front-end end-to-end tests (Playwright) | 99 | `test(` lines in `tests/e2e` |
| Locales | 10 | entries of `LOCALES` in `src/lib/locales.ts` |

**Task boxes in [`tasks.md`](../tasks.md)** (ticked / open, by prefix):

| Prefix | Ticked | Open |
|---|---|---|
| SF | 5 | 4 |
| ID | 2 | 6 |
| AU | 1 | 2 |
| RC | 1 | 11 |
| SE | 4 | 9 |
| DO | 0 | 8 |
| OR | 0 | 9 |
| IO | 1 | 18 |
| CA | 0 | 8 |
| AS | 0 | 18 |
| CQ | 24 | 15 |
| TR | 2 | 11 |
| PN | 0 | 11 |
| EM | 0 | 16 |
| PA | 1 | 16 |
| EC | 0 | 13 |
| OS | 4 | 10 |
| OF | 2 | 5 |
| PC | 1 | 8 |
| PR | 4 | 4 |
| LC | 2 | 11 |
| XP | 0 | 14 |
| VC | 0 | 12 |
| SM | 0 | 5 |
| MT | 0 | 4 |
| DX | 0 | 10 |
| PM | 0 | 5 |
| TO | 0 | 9 |
| QA | 0 | 12 |
| RA | 0 | 7 |
| PI | 2 | 7 |
| HM | 0 | 5 |
| VW | 0 | 6 |
| BV | 5 | 4 |
| SG | 4 | 1 |
| SW | 4 | 3 |
| TT | 1 | 6 |
| PX | 6 | 0 |
| WL | 3 | 9 |
| RS | 0 | 5 |
| **Total** | **79** | **337** |

<!-- generated:end -->
