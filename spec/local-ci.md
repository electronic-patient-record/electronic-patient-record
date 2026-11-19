# Local CI/CD: more of the checking on our own machine (LC)

**Status: a proposal from a research pass (plan.md §26, 2026-11-04). Nothing here is built; no tool was installed or run.** It says how the repository's checks could run
locally first with GitHub as a second, independent witness, and what that does and does not prove. Tasks: `LC-1` to `LC-13` in [`../tasks.md`](../tasks.md).

## Where the checking happens today

| Tier | What runs | Where | Source |
|---|---|---|---|
| 0 | UI lint, `cargo fmt --check`, generated docs, links, only for staged files | the author's machine, on commit | `.githooks/pre-commit` |
| 0b | `scripts/ci-local.sh --fast` for the projects the pushed commits touch | the author's machine, on push | `.githooks/pre-push` (LC-5) |
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

It bypasses the `kache` compiler cache by default, because kache has served stale libraries and fails under load (`EPR_CI_KEEP_CACHE=1` keeps it). The database gates use a scratch database (LC-4, below).

## A scratch database per run (LC-4, built 2026-11-13)

`--full` makes a database for the run, named `epr_ci_<UTC time>_<pid>_<random>_test` (the suffix is what `tests/chaos_api.rs` asks of a database it may write to). It applies `ci/postgres-init/`, runs the migrations as a gate of their own (`service-migrate`: `electronic-patient-record db migrate`), points `DB_URL` at the new database for `service-database`, and drops it when the run ends, after an interrupt too. When `DB_URL` is set it names only the server: its own database is used to create and drop the scratch database, and the run writes nothing to it. When `DB_URL` is not set, a throwaway Postgres 18 is started on a free port (`EPR_CI_PG_PORT` chooses one) and removed after, so two runs no longer meet on port 5544. The functions are in `scripts/scratch-db.sh`.

So two runs at once do not see each other's rows, and a row left in a shared test database cannot fail a run. `scripts/test_scratch_db.sh` checks this: two runs at once on one server with a leftover table in the shared database, then two runs at once with no `DB_URL`. Each run must see only its own row and none of the leftover table, the names must differ, and nothing may be left behind. It is the `scratch-db-test` gate here and a step of CI's `api-db` job. Two broken versions were run against it and both failed: one where every run got the same name, and one that used the shared database.

**Not done:** a run killed with `kill -9` leaves its scratch database (find them by the `epr_ci_` prefix; nothing sweeps them). The role needs `CREATEDB` on a given server. Two runs at once still share `target/` (cargo makes the second wait), `target/ci-local/` logs (the later run overwrites them), the Playwright port and the container image tags.

## One definition per gate (LC-3, built 2026-11-15)

Each gate is one script, `scripts/gates/<name>.sh`, named as in `ci-local.sh`'s table. It changes to its own directory, so
it runs the same from anywhere, and both `ci.yml` (`run: sh scripts/gates/<name>.sh`) and `ci-local.sh` call it: 30
scripts. What a gate does can no longer differ between here and CI, though the machine, the tools installed (nextest,
for one) and the environment still can.

`scripts/check-gates.py` (the `docs-gates` gate) reads every `run:` step in `ci.yml`. Each one must call a gate script that
exists, or be listed in the check with its reason:
- **setup:** installing packages, the browser, podman-compose and gitleaks, deciding which jobs run, preparing CI's
  database;
- **GitHub only:** the secret scan, the two pull-request checks, `ci-ok`, the toolchain pin and pnpm audit;
- **CI only:** the two `podman compose` start-up checks, which need a clean runner's free ports.

It also fails when a gate script has no local twin, or a listed step no longer exists, so the list stays true.
`scripts/test_check_gates.sh` adds an inline gate, a call to a missing script and a stale entry to copies of the
workflow, and each fails. CodeQL and cargo deny are GitHub actions with no `run:` step and no local twin; the check
names them.

`ci-local.sh --full` gained the two coverage floors (`coverage-matcher`, `coverage-service`), which only CI ran before.
They SKIP without `cargo-tarpaulin` (until 2026-11-17, `cargo-llvm-cov`).

## Speed spike (LC-8, run 2026-11-15)

**Where the time goes.** In CI's last full run (`d520ab6`) the API container image took **679 s**, a release build from
nothing on every push, and sets the wall time of a code push. Then the database tests (150 s), the FHIR validator (135 s)
and the API's database-free tests (100 s, of which the tests ran about 8 s; the rest builds and links over 50 test
binaries). Locally, a clean debug build of the service with `--timings` took 105 s by the clock and 658 s of compile time
over 554 units at a load of 18 to 29. The largest were `fhir-r5` (48 s), `fhir-r4` (43 s), the service's library test
build (40 s), `aws-lc-sys` (30 s) and `ring`, built twice (15 s each). Both TLS back ends were built then because `sqlx`
pulled `ring` and `reqwest` pulled `aws-lc-rs`. Since `authentication-verifier` 0.9.3 (2026-11-17) `reqwest` takes no
TLS provider of its own and `ring` is the only one, so `aws-lc-sys` and its 30 s are no longer built; these timings were
not re-measured after that change.

**Rebuild after an edit (measured 2026-11-17, one run, load average about 90 from other work on the machine).** After a
one-file change to a controller: the service library alone rebuilt in 109 s; the library and every test program in
240 s, wall clock, of which 83 build units took about 3,560 s of CPU between them, about 44 s for each of the ~80
integration test programs (each file in `tests/` is its own program, built and linked against the library). The domain
crate split (2026-11-17, [architecture](08-architecture.md)) spares the rules' share of the library; the test programs
are most of the cost, and folding them into one program is task LC-15. Not measured on a quiet machine.

**cargo-nextest: adopted.** It runs the same tests: 117 in the matcher, 553 in the service with the same 182 database
tests left out, and neither project has doc tests, which nextest does not run (`cargo test --doc` follows it anyway).

| Where | `cargo test` | nextest | Load or cache |
|---|---|---|---|
| Matcher, here, warm, three runs | 25 to 41 s | 15 to 22 s | load 18 to 24 |
| Service, here, warm, three runs | 10 to 12 s | 8 s | load 30 to 41 |
| Service in CI, build and run, cold, one run | 262 + 13 s | 274 + 11 s | no cache |
| Service in CI, build and run, warm, two runs | 130 + 9, 128 + 8 s | 105 + 12, 96 + 8 s | cache restored |

CI's API job now uses `cargo nextest run` then `cargo test --doc`. `ci-local.sh`'s `matcher-test` and `service-test`
use them when `cargo-nextest` is installed, and plain `cargo test` when it is not. The matcher's CI job was not
measured and is unchanged. Why the warm nextest build was faster is not known; one cold run showed no difference.

**mold: not adopted.** In CI, warm, the mold variants built in 123 s on both runs, no faster than `cargo test` and
slower than nextest alone; cold, mold took 325 s against 262 s. On macOS the system linker is used.

**sccache: not tried.** CI already restores the dependency build with `Swatinem/rust-cache` in every Rust job, so
sccache would cache what is already cached. Locally it would replace kache.

**The lever that is left** is the API image, about two thirds of a code push's wall time. A layer cache does not
survive between hosted runs, so it needs either a dependency layer kept in a registry, or the release binary built on
the runner with its cache and copied in. Both change how the image is built: task LC-14.

**Not measured:** two warm samples and one cold sample per variant on hosted runners, whose speed varies by tens of
seconds between identical runs. The spike ran on the branch `spike/lc-8` (workflow `speed-spike.yml`, run 43694245176,
three attempts), which was then deleted.

## A signed local gate record (LC-6, built 2026-11-15)

`sh scripts/ci-local.sh --full`, on a tree whose tracked files match HEAD, ends by writing
`target/ci-local/<commit>.json` and its signature, `<commit>.json.sig` (`scripts/gate-record.sh`). The record holds the
commit and its tree, the number of untracked files, the host's OS and architecture, the versions of rustc, cargo, node,
pnpm, podman, java and python, each gate's result, seconds and note, and `passed`, which is true only when every gate
passed (a run narrowed by `EPR_CI_ONLY`, or with a `SKIP`, never passes). Its `statement` reads:

> Self-attested, not a hosted build. This record says what one machine ran on one commit, signed with its author's
> key; it is not evidence for the hazard log or the safety case (OQ-LC3).

It is signed with git's `user.signingkey` by `ssh-keygen -Y sign` in the namespace `epr-ci-local`, so a signature made
for a commit or for anything else cannot stand in for it. A tracked file that differs from HEAD, or HEAD moving during
the run, writes no record. A failed signing keeps no new record and leaves an earlier one as it was.

`sh scripts/ci-local.sh --verify <commit>` finds the signer in git's `gpg.ssh.allowedSignersFile`, checks the signature,
checks that the record names that commit and its tree and carries the statement, then prints which gates passed.

**Acceptance, run 2026-11-15** on `d520ab6`, the tree clean: all 28 gates passed, the record was written and signed with
the maintainer's key, and `--verify d520ab6` printed "verified, signed by an allowed key; 28 gates, all passed" and the
statement. `scripts/test_gate_record.sh` (the `docs-gate-record` gate and a step of CI's docs job) uses a throwaway key
and repository and checks:
- a record verifies;
- one changed, copied to another commit, or signed by a key not on the list fails;
- a dirty tree is refused;
- a failed run names its gates;
- a signing failure leaves an earlier record verifying.

**Not done:** records stay in `target/`, which is not committed or published. A record shows that the key's holder ran
the gates, not that they would pass on another machine. Whether a record may ever count as safety evidence is OQ-LC3,
for the Clinical Safety Officer; until then it may not.

## CI runs only what a push needs (LC-7, built 2026-11-15)

A `changes` job in `ci.yml` lists the files changed and runs `scripts/ci-changes.sh` on them, which answers two
questions: does Rust need checking, and does the UI?

| Job | Runs when |
|---|---|
| Matcher, API, API with Postgres, FHIR validator, cargo deny, coverage | Rust |
| UI, pnpm audit | UI |
| Container images | Rust or UI |
| CodeQL | per language: Rust or UI (a skipped language still starts a runner for a few seconds) |
| Docs, secret scan, toolchain pin, hazard log, `ci-ok` | every push |

Files in a project's directory need that project. The UI's `src/` needs Rust too, because the service's risk and grief
censuses read it. Documents need neither, except the ones a project's tests read (`SECURITY.md`,
`governance/personal-data-inventory.csv`, `governance/production-readiness.md`, `governance/translation-review.md`,
`operations/keycloak/`). Any file the script does not know, such as a workflow or a script, needs both.

**What a push is compared with.** For a push it is the last commit on the branch whose CI passed, not the commit
before it: a newer push cancels a run in progress (20% of runs, LC-1), and comparing with the previous commit would
then never check the cancelled run's files. For a pull request it is where the branch left its base. A nightly run
(03:17 UTC), a manual run, or a branch with no green commit runs everything. The nightly run catches what no file shows,
such as a new advisory for a locked crate or a changed base image. The concurrency group names the event, so a push
does not cancel the nightly run.

`ci-ok` waits on `changes` as well as every job, and stays the single required check. A job the `changes` job left out
is `skipped`, which passes; a failed `changes` job fails `ci-ok`.

**Keeping it honest.** `scripts/test_ci_changes.sh` (the `docs-ci-changes` gate and a step of CI's docs job) has fixed
cases, and a census. It finds every file a project's tests or source name outside the project's own directory, as a
`"../path"` literal or a `join("dir/...")` from the repository root, and fails unless a change to that file runs the
project that reads it. It found 10 such files, among them one a hand-made list had missed: a UI test reads the
service's `electronic-patient-record-domain/src/adjustments.rs`. Run with `SECURITY.md` dropped from the list, it failed.

**Measured 2026-11-15.** The documents-only push `792d5dc` ran the docs, secret-scan, toolchain and `changes` jobs, two
CodeQL runners that skipped their steps, and `ci-ok`; the 11 build and test jobs were skipped. It took **0.6
job-minutes and 18 s** of wall time, and `ci-ok` passed. The full run just before it (`3bf1757`, which changed the
workflow, so everything ran) took 35.6 job-minutes and 10.1 min; LC-1's median was 42.3 and 13.4. Over the last 200
commits on `main`, `ci-changes.sh` sorts 22 as documents only (11%), 99 as Rust only, 10 as UI only and 69 as both.
From that run's job times, the saving is about 35 job-minutes on a documents-only push, 3.7 on a Rust-only push (the
UI job, pnpm audit and CodeQL for JavaScript) and about 21 on a UI-only push. That is **about 6.8 job-minutes a push on
average, some 16% of LC-1's median**. The wall time of a code push does not change: the container build still sets
it. The estimate takes one run's job times as typical and the last 200 commits as the future mix.

**Not done:** the pre-push hook (LC-5) still maps files by directory and does not use `ci-changes.sh`, so a change to
`SECURITY.md` alone runs only the document checks before a push. The census sees only literal paths; a path built at
run time would escape it, and the nightly run is then the backstop.

## A preflight (LC-12, built 2026-11-15)

`sh scripts/ci-local.sh --full` first runs `scripts/preflight.sh`, which checks the traps that have failed a full run
here, reports every one it finds with the command that fixes it, and stops the run with exit 3 before any gate runs.
`EPR_CI_SKIP_PREFLIGHT=1` skips it. `--fast` does not run it: none of its gates meets these traps.

| Check | Fails when | Fix it names |
|---|---|---|
| Browser test port | `PW_PORT` (default 4183) is in use | a free `PW_PORT`, or `lsof` to find what listens |
| Throwaway Postgres port | `EPR_CI_PG_PORT` is set and in use | unset it, or a free port |
| A given Postgres | `DB_URL` names a port on this machine with nothing listening | start that server, or unset `DB_URL` |
| FHIR validator | `java` is on `PATH` and the pinned jar is not cached | `sh scripts/fhir-validator.sh` |
| Podman VM clock | more than 30 s from the host's | `podman machine ssh "sudo date -u -s @$(date -u +%s)"` |
| Podman VM memory | under 16,000 MB | `podman machine set --memory 32768` (stops every container) |
| Podman VM disk | under 20 GB free on `/var` | `podman system prune`, after reading what it lists |

A check whose tool is missing (no `podman`, no `java`) passes, because that gate already shows as `SKIP`. On Linux,
where Podman has no VM, the three VM checks do not run. Redis is not checked: no local gate uses it.

`scripts/test_preflight.sh` breaks each condition on purpose, with a listener it starts holding a port and faked VM
readings, and expects exit 3 and the named fix; a good machine must pass and two problems must both be reported. It is
the `docs-preflight` gate and a step of CI's docs job. Run against a preflight whose clock check never fired, it failed
twice. With port 4183 really taken, `--full` stopped in 1 s with the message and exit 3. The whole preflight takes about
1 s on this machine.

**Not done:** the thresholds (30 s, 16,000 MB, 20 GB) are judgements from the failures seen, not measured limits. The
VM checks were tried only on macOS with `applehv`. A remote `DB_URL` is not checked; the database gates find it.

## A pre-push hook (LC-5, built 2026-11-14)

`.githooks/pre-push` runs `sh scripts/ci-local.sh --fast` before each push, for the projects the pushed commits touch,
and `scripts/install-hooks.sh` installs it with the pre-commit hook. For each ref git pushes it lists the files changed
since the remote's commit, or since where a new branch left `origin/main`. A file under a project's directory selects
that project, `rust-toolchain.toml` selects both Rust projects, and the document checks run whenever anything is
pushed. A deleted branch, or a push with no new commits, checks nothing. When the remote's commit is not in the clone
(fetch first), every project is checked. `git push --no-verify` skips it once.

`ci-local.sh` takes the selection as `EPR_CI_ONLY` (`matcher`, `service`, `ui`, `docs`, space-separated). A gate left
out shows in the table as `off`, so a narrowed run never reads as a whole one, and an unknown project is refused (exit 64).

It checks the working tree, not the pushed commits. With uncommitted changes in a checked project it says so, because
a pass is then about files the push does not carry.

**Acceptance, run 2026-11-14.** A badly formatted function added to the matcher's `src/lib.rs`, with a push of a
commit that touches the matcher: `matcher-fmt` failed (and `matcher-clippy`, on the unused function), and the hook
exited 1 in 55 s. Measured with every project selected and the build caches warm, at a load average of 80: 107 s, of
which the matcher's tests were 50 s. After a dependency change the service's clippy and tests rebuild; that took
about 270 s in the `--full` run of the same day. `scripts/test_pre_push.sh` checks the selection in a throwaway
repository, plus `EPR_CI_ONLY`, and is the `docs-hooks` gate here and a step of CI's docs job. It was run against a
broken hook (an unknown remote commit checking nothing) and failed.

**Not done:** the hook does not stash uncommitted changes, so it can pass on files the push does not carry, or fail on
them. A push of tags alone checks the files the tag's commit brings, like a branch. Nothing measured a cold build on a
fresh clone.

| CI job | Here |
|---|---|
| Matcher, API, UI, docs, FHIR validator, API database tests | The same gates, except that UI unit tests run without the coverage report |
| Minimum Rust (`rust-msrv`, QA-8) | The same gate, in `--full` only (another toolchain's build); it installs the toolchain when missing |
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
