## 14. Implementation Status

**Milestone M0 (scaffold), 2026-10-20.** Not for clinical use; no clinical record exists.

| Area | State |
|---|---|
| Matcher | Built: `Patient`, engine, NHS number validation. Missing: labelled corpus, fuzz, benches |
| Service | Built: app wiring, auth guard and startup gate, compare endpoint, `audit_events` table. Missing: everything else |
| UI | Built: shell, locale URLs, headers/CSP, proxy, compare page. Missing: sign-in, patient pages, widgets, translations |
| CI, release, compose, Containerfiles | Written; see the changelog for what was run |

<!-- generated:start -->
<!-- This block is written by scripts/generate-status.sh; do not edit it by hand. -->

**Generated facts** (counted from the source tree by `scripts/generate-status.sh`;
no network, database or compilation; counts are source lines, not test runs):

| Fact | Value | Method |
|---|---|---|
| Service version | 0.1.0 | `version` in the service `Cargo.toml` |
| Matcher version | 0.1.0 | `version` in the matcher `Cargo.toml` |
| Front-end version | 0.1.0 | `version` in the front-end `package.json` |
| Migrations | 1 | files named `m2*.rs` in `migration/src` |
| Route registrations | 1 | `.add(` calls in `src/controllers/*.rs` |
| Service tests | 8, plus 2 `#[ignore]`d (database gated) | `#[test]` and `#[tokio::test]` lines; `#[ignore]` lines |
| Matcher tests | 13, plus 0 `#[ignore]`d | the same patterns in the matcher |
| Front-end unit tests (vitest) | 26 | `it(` / `test(` lines in `tests/unit` |
| Front-end end-to-end tests (Playwright) | 6 | `test(` lines in `tests/e2e` |
| Locales | 10 | entries of `LOCALES` in `src/lib/locales.ts` |

**Task boxes in [`tasks.md`](../tasks.md)** (ticked / open, by prefix):

| Prefix | Ticked | Open |
|---|---|---|
| SF | 5 | 4 |
| ID | 0 | 7 |
| AU | 0 | 2 |
| RC | 0 | 9 |
| SE | 0 | 7 |
| DO | 0 | 6 |
| OR | 0 | 7 |
| IO | 0 | 8 |
| CA | 0 | 7 |
| AS | 0 | 11 |
| **Total** | **5** | **68** |

<!-- generated:end -->
