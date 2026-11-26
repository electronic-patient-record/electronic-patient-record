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
| Matcher version | 0.9.0 | `version` in the matcher `Cargo.toml` |
| Front-end version | 0.1.0 | `version` in the front-end `package.json` |
| Migrations | 50 | files named `*.sql` in `migrations` |
| Route registrations | 133 | `.add(` calls in `src/controllers/*.rs` |
| Service tests | 1090, plus 283 `#[ignore]`d (database gated) | `#[test]` and `#[tokio::test]` lines; `#[ignore]` lines |
| Matcher tests | 136, plus 0 `#[ignore]`d | the same patterns in the matcher |
| Front-end unit tests (vitest) | 400 | `it(` / `test(` lines in `tests/unit` |
| Front-end end-to-end tests (Playwright) | 124 | `test(` lines in `tests/e2e` |
| Hazards | 80, of which 59 open and 0 accepted or closed; 49 name a test | `scripts/check-hazard-log.py --counts` |
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
| PR | 8 | 1 |
| LC | 10 | 6 |
| XP | 3 | 13 |
| VC | 0 | 12 |
| SM | 2 | 3 |
| MT | 0 | 4 |
| DX | 0 | 10 |
| PM | 0 | 5 |
| TO | 0 | 9 |
| QA | 7 | 10 |
| RA | 0 | 7 |
| PI | 3 | 7 |
| HM | 0 | 5 |
| VW | 0 | 6 |
| BV | 5 | 4 |
| SG | 4 | 1 |
| SW | 4 | 3 |
| TT | 1 | 6 |
| PX | 6 | 0 |
| WL | 11 | 1 |
| RS | 6 | 2 |
| AX | 8 | 0 |
| IW | 0 | 8 |
| SC | 0 | 7 |
| CN | 0 | 7 |
| OE | 0 | 8 |
| NF | 0 | 9 |
| MA | 0 | 8 |
| PP | 0 | 10 |
| AN | 0 | 10 |
| PH | 0 | 10 |
| RD | 0 | 7 |
| TH | 0 | 1 |
| IH | 12 | 0 |
| DEP | 0 | 2 |
| TL | 6 | 9 |
| MP | 3 | 30 |
| CF | 0 | 4 |
| EH | 2 | 15 |
| AG | 2 | 14 |
| QI | 9 | 71 |
| RF | 2 | 10 |
| DR | 1 | 11 |
| DM | 0 | 5 |
| **Total** | **163** | **569** |

<!-- generated:end -->
