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

## The model proposed

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
