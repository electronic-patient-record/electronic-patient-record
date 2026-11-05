# Local CI/CD: more of the checking on our own machine (LC)

**Status: a proposal from a research pass (plan.md §26, 2026-11-04). Nothing here is built; no tool was installed or run.** It says how the repository's checks could run
locally first with GitHub as a second, independent witness, and what that does and does not prove. Tasks: `LC-1` to `LC-13` in [`../tasks.md`](../tasks.md).

## Where the checking happens today

| Tier | What runs | Where | Source |
|---|---|---|---|
| 0 | UI lint, `cargo fmt --check`, generated docs, links, only for staged files | the author's machine, on commit | `.githooks/pre-commit` |
| 1 | The gates in AGENTS.md: fmt, clippy, tests, docs, UI lint, check, unit and browser tests | the author's machine, by hand before a push | AGENTS.md "Gates" |
| 2 | Fifteen jobs behind one required check, `ci-ok`; a clean x86-64 Linux machine each time | GitHub Actions on `ubuntu-latest` | `.github/workflows/ci.yml` |
| 3 | The release build on a version tag | GitHub Actions | `.github/workflows/release.yml` |

Measured 2026-11-04 (one run and the last 200 statuses; the 30-day table is task LC-1): about 40 job-minutes and 11 to 16 minutes by the clock per push; 143 success, 14 failure, 39 cancelled. The repository is public, so
standard hosted minutes are free; the cost is waiting.

## Baseline, measured 2026-11-04 (LC-1)

`python3 scripts/ci-baseline.py --runs 200 --job-runs 40` (reads with `gh`; changes nothing). 197 completed runs on `main`, 2026-10-23 to 2026-11-04 (11.9 days, **16.5 runs a day**).

| Outcome | Runs | Share |
|---|---|---|
| success | 144 | 73% |
| failure | 14 | 7% |
| cancelled by a newer push | 39 | 20% |

Successful runs: wall time **median 13.4 min, p90 15.6, max 38.3**. Job minutes per successful run (newest 32): **median 42.3, max 48.7**. At 16.5 runs a day that is about 700 job-minutes a day.

| Job (newest 32 successful runs) | Median min | Max |
|---|---|---|
| Container images build (Podman) | 14.8 | 15.8 |
| CodeQL (rust) | 9.7 | 11.9 |
| API (Postgres 18 tests) | 4.5 | 5.8 |
| API (fmt, clippy, database-free tests) | 4.0 | 6.1 |
| FHIR validator | 2.2 | 6.3 |
| UI (check, unit, browser tests) | 2.1 | 2.8 |
| Coverage floor | 1.9 | 3.5 |
| CodeQL (javascript-typescript) | 1.4 | 1.7 |
| The other eight jobs together | about 3.4 | |

**Reading it.** Two jobs, the container build and CodeQL for Rust, are about 24.5 of the 42.3 job-minutes (58%), and the container build alone sets the wall time. Trimming those (LC-7) or moving them off every push saves more than any speed-up of the Rust tests (LC-8).

**Failures in the newest 40 runs (five jobs, three runs):** the UI type check and the UI image build (2026-11-03 06:04); the database-gated test step three times (2026-11-01 22:01 and 22:46, 2026-11-02 01:33). Each failed in a step that runs our tests or our build, not in checkout, tool install or runner start-up, which is what a platform fault would look like. **I did not read the logs**, so "ours" is an inference from the step names; the other nine failures were outside this window and were not looked at.

**Not measured:** minutes are start to end, not GitHub's per-job rounding up; queue time before a job starts is not in these numbers (it is where a platform slowdown would show); one 38.3-minute wall time was not looked into.

## One command (LC-2, built 2026-11-05)

`sh scripts/ci-local.sh --fast` runs fmt, clippy, the docs, the database-free tests, the UI's lint, type checks and unit tests, and the document checks. `--full` adds the database tests, the UI browser tests, the FHIR validator and both container builds. `--list` names the gates.

It prints a table of gate, result and seconds, keeps each gate's output in `target/ci-local/`, and runs every gate even after a failure, so one run shows everything that is wrong. It exits 0 when all passed, 1 when a gate failed, and 2 when a gate could not run. A gate that cannot run says `SKIP` and why, and that fails the run, so nothing is skipped silently. `EPR_CI_ALLOW_SKIP=1` lets it exit 0; the table still shows the skip.

It bypasses the `kache` compiler cache by default, because kache has served stale libraries and fails under load (`EPR_CI_KEEP_CACHE=1` keeps it). The database gate uses `DATABASE_URL` if set, or starts a throwaway Postgres 18 and removes it afterwards.

| CI job | Here |
|---|---|
| Matcher, API, UI, docs, FHIR validator, API database tests | The same gates, except that UI unit tests run without the coverage report |
| Container images | Built. **Not** the checks that the API refuses to start with no keys and demands a token |
| `cargo deny`, CodeQL, `pnpm audit`, the secret scan, the coverage floor, the fresh clone, the two hazard-log pull-request checks | GitHub only |


1. **Tier 1 becomes one command and a hook** (`scripts/ci-local.sh --fast` on pre-push; `--full` before a batch of pushes), against a scratch database, with a preflight for the known local traps.
2. **Tier 2 stays on GitHub as the independent check**, trimmed so it does not repeat work that cannot change (path filters, heavy jobs on `main` or nightly).
3. **A local pass is a convenience and a self-attested record, never the release build** and not clinical-safety evidence until OQ-LC3 is answered.
4. **GitHub being down must delay a release, not block it**: a written by-hand release route and a second remote of the source.

## Options considered

The table is in [plan.md §26.3](../plan.md): local gates (do first), one definition per gate (after), trimming GitHub (do), `act` (spike), a self-hosted GitHub runner (**not adopted**), Forgejo with a Podman runner (spike, if a second forge is acceptable), Woodpecker (not assessed), Dagger (defer), faster builds (measure first).

## What a local pass proves, and what it does not

- It proves the gates passed **on that machine, at that commit, with a clean tree** if the record says so. It does not prove they pass on x86-64 Linux, on a fresh clone, with a fresh database or with the pinned tool versions of CI.
- It is **self-attested**: the author runs the script and holds the key. Sources on SLSA say a self-hosted build does not reach Build Level 2 by itself; the project does not claim SLSA.
- The clean-room jobs (`fresh-clone`, `secrets`, `deny`, CodeQL, the container build) stay on GitHub.

## Not adopted

*(Filled by LC-13.)* A self-hosted GitHub runner on the public repository: GitHub says such runners "should almost never be used for public repositories"; the runner still depends on GitHub dispatching jobs, which failed for self-hosted runners on 19 to 20 July 2026; the per-minute fee for self-hosted use is postponed, not cancelled. Reopen if the repository becomes private.

## Not done, not verified

The baseline (LC-1), every spike (LC-8 to LC-10) and every speed-up are unmeasured. Service-container support in `act`, Forgejo and Woodpecker, hosted-runner prices, the licence terms of running CodeQL locally and the effect of arm64 against x86-64 on our tests were not checked.
