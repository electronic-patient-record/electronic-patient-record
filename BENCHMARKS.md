# Benchmarks

**One laptop run on 2026-11-15. Not a capacity claim.** Nothing here says what a deployment can carry; it says what
this build did on one machine, with one client on the same machine. How the numbers are made, and what each one
leaves out, is [performance testing](spec/performance-testing.md). Raw results: `scripts/load/results-2026-11-15/`.

| | |
|---|---|
| Build | release profile of commit `46fdc48` plus the uncommitted harness and benchmarks of the commit that adds this page (no service code differed) |
| Machine | Apple M4 Max, 16 cores, 128 GiB; Postgres 18 from mise on the same host (a throwaway server, not the Podman VM) |
| Service | `sh scripts/load/run-local.sh`: `container-dev` (login off), pool 20 (2 minimum), log level `warn`, hash-chained audit on and unkeyed, a scratch database per run |
| Client | `scripts/load/run.mjs`, Node 26, on the same host |
| Data | 200 seeded patients, each with an allergy and three heart rates, plus whatever the write steps added |

## Load: the standard profile

Each step: 2 s of warm-up, then 10 s counted, three times; the median of the three is shown. Every response was a
success; no request failed.

| Scenario | Workers | Per second | p50 ms | p95 ms | p99 ms | Spread of p95 over the three |
|---|---|---|---|---|---|---|
| Register a patient | 1 | 141 | 6.4 | 11.9 | 24.4 | |
| Register a patient | 8 | 84 | 85.9 | 183.0 | 266.5 | |
| Register a patient | 32 | 90 | 336.6 | 411.8 | 468.8 | 368 to 433 |
| Open a patient's identity record | 1 | 202 | 3.6 | 11.1 | 26.2 | |
| Open a patient's identity record | 8 | 1,198 | 5.7 | 14.6 | 19.2 | |
| Open a patient's identity record | 32 | 1,251 | 21.4 | 46.1 | 58.7 | 45 to 49 |
| Search by family prefix | 1 | 81 | 12.1 | 14.9 | 16.8 | |
| Search by family prefix | 8 | 530 | 13.6 | 25.2 | 36.7 | |
| Search by family prefix | 32 | 701 | 42.9 | 80.7 | 122.1 | 68 to 90 |
| **Chart open** (8 reads side by side) | 1 | 118 | 8.0 | **12.4** | 16.3 | |
| **Chart open** | 8 | 168 | 46.5 | **67.9** | 80.0 | |
| **Chart open** | 32 | 152 | 205.2 | **289.0** | 341.8 | 267 to 373 |
| FHIR Patient read | 1 | 192 | **4.4** | 9.4 | 18.6 | |
| FHIR Patient read | 8 | 860 | **8.1** | 21.0 | 33.2 | |
| FHIR Patient read | 32 | 933 | **32.5** | 47.5 | 53.4 | 45 to 73 |
| FHIR Patient search | 1 | 22 | 40.0 | 119.8 | 185.3 | |
| FHIR Patient search | 8 | 148 | 56.8 | 94.8 | 143.0 | |
| FHIR Patient search | 32 | 266 | 109.3 | **247.7** | 292.6 | 209 to 320 |
| Compare two records | 1 | 332 | 2.8 | 4.5 | 9.6 | |
| Compare two records | 8 | 1,012 | 7.2 | 14.2 | 20.6 | |
| Compare two records | 32 | 1,100 | 28.5 | 41.4 | 54.8 | 40 to 47 |
| Record a vital sign | 1 | 266 | 3.5 | 6.2 | 8.2 | |
| Record a vital sign | 8 | 961 | 8.0 | 13.6 | 22.2 | |
| Record a vital sign | 32 | 3,346 | 9.3 | 12.8 | 22.7 | 12 to 38 |
| Record an allergy | 1 | 402 | 2.5 | 2.8 | 3.0 | |
| Record an allergy | 8 | 992 | 3.5 | 28.8 | 104.8 | |
| Record an allergy | 32 | 202 | 149.6 | 346.2 | 398.4 | 236 to 374 |

**Against NFR-2** (`scripts/load/budgets.json`): chart open p95 is within 1 s at every level (289 ms at 32 workers);
search p95 is within 200 ms at every level (81 ms at 32). **Two budgets broke at 32 workers:** FHIR read p50 was
32.5 ms against 10 ms (it met it at 1 and 8 workers), and FHIR search p95 was 248 ms against 200 ms. NFR-2 names no
load, so the harness holds every budget at every level; whether 32 clients at once is the case NFR-2 means is open.

## Load: stress, soak and the pool

- **Stress** (the mix, workers doubling, one 10 s run each): p95 stayed under the 1 s budget up to 32 workers
  (706 ms) and broke it at 64 (1,149 ms), where the run stopped. No request failed. Throughput did not rise with
  workers past 4 (132 a second at 4, 80 to 144 from 8 to 64), and moved about from level to level.
- **Soak** (the mix, open loop at 50 a second, 3 minutes in 30 s windows, not the profile's 10): 9,000 requests,
  none failed or dropped, p95 12.9 ms; the last window's p95 was 1.06 times the first's, so no drift showed in 3
  minutes. A 10 minute or longer soak was not run.
- **Pool size** (register, then read, at 32 workers, one 10 s run each, a fresh database each time):

| Pool | Register per second | Register p95 ms | Read per second | Read p95 ms |
|---|---|---|---|---|
| 5 | 194 | 233.0 | 249 | 241.6 |
| 20 | 324 | 210.4 | 1,857 | 25.5 |
| 50 | 323 | 155.3 | 1,867 | 21.0 |

  Reads need more than 5 connections; past 20 neither rate moved. Registration was faster here than in the
  standard run (324 against 90 a second) on a fresh, smaller database after a shorter run. The difference is not
  explained and was not investigated.

## The audit lock (CQ-18): first database-side evidence

`run-local.sh` sampled `pg_stat_activity` five times a second during each run. In the standard run, of 35,000
samples of busy sessions, **29,300 (84 percent) were waiting on `Lock advisory`**, the audit chain's lock; 4,700
were running. The stress, soak and pool runs show the same order (`*-waits.txt` beside the results). With a pool
of 50 more sessions waited on the lock than with 20, and no more work was done.

This moves CQ-18 from a hypothesis towards a finding: most of the database's time under load is spent queueing for
the one lock. **Still not shown:** a run with the append taken out, and which scenarios' sessions hold or wait (the
samples are not labelled by request). Two numbers do not fit a simple story and are recorded as they are: recording
a vital sign reached 3,346 a second at 32 workers, faster than any other write, while recording an allergy fell
from 992 a second at 8 workers to 202 at 32. Both are audited; why they differ was not looked into.

## Benchmarks (criterion, one process, no database)

`cargo bench` with 1 s of warm-up and 3 s of measurement per benchmark; the middle of criterion's interval.

| Matcher | Time | | Service | Time |
|---|---|---|---|---|
| Fold a name ("Smith" / a Welsh name) | 194 ns / 536 ns | | Audit row digest (unkeyed SHA-256) | 1.40 µs |
| Soundex | 170 ns | | Policy decision (3 cases) | 7.6 to 24.4 ns |
| National number check | 204 ns | | Parse a registration body | 850 ns |
| Compare two records (6 shapes) | 3.2 to 7.8 µs | | Validate a registration | 252 ns |
| Score against 50 candidates | 315 µs | | Compare with the site's rules | 6.1 µs |
| Score against 1,000 candidates | 6.77 ms | | Parse an HL7 v2 registration | 13.6 µs |
| | | | Record a request's duration | 52 ns |

The service's own work per request is microseconds; the request times above are milliseconds. The time goes to the
database and the lock, not to this code. Scoring a register of 1,000 takes 6.8 ms, which matters only if a search
ever returns that many (it returns at most 50).

## Not done and not verified (this run)

- Everything in [performance testing § not done](spec/performance-testing.md#not-done-and-not-verified): a thin
  chart, no typeahead, login off, no order sign, one client on the same host, a guessed mix.
- One standard run (three repeats per step), one stress, one 3 minute soak, one pool sweep. The spreads above show
  p95 moving by up to half between repeats at 32 workers: treat differences under about 30 percent there as noise.
- The tree was dirty (the harness itself, not the service), so `dirty` is true in the JSON.
- The database ran on the same host as the service, not in the Podman VM the 2026-10-30 run used, so the two runs
  do not compare directly.
- No run on CI yet: the nightly `performance` job is new, and its first run has not been looked at.

## The 2026-10-30 run
**One laptop run on 2026-10-30, kept for comparison. Not a capacity claim.** Nothing here says what a deployment can carry; it says what
this build did once, on one machine, with one client on the same machine.

| | |
|---|---|
| Build | release profile of commit `fdcd091` plus uncommitted working-tree changes (OS-9, OS-4) |
| Machine | Apple M4 Max, 16 cores; Postgres 18 in a Podman VM with 8 CPUs and 12 GiB |
| Service | `container-dev` profile: login off, pool 2 to 20, JSON logs at debug level, hash-chained audit on |
| Client | `scripts/load/measure.mjs` (Node 26, `fetch`; replaced by `scripts/load/run.mjs` on 2026-11-15, in git history), on the same host; raw numbers in `scripts/load/results-2026-10-30.json` |
| Data | 200 seeded patients plus whatever the write steps added (about 5,400 in total); 10 seconds per step |

### Results

| Step (every request is audited) | Concurrency | Per second | p50 ms | p95 ms | p99 ms |
|---|---|---|---|---|---|
| Register a patient | 1 | 177 | 4.5 | 11.9 | 18.9 |
| Register a patient | 8 | 166 | 32.3 | 110.7 | 158.5 |
| Register a patient | 32 | 194 | 164.5 | 191.0 | 203.4 |
| Open a patient by id | 1 | 97 | 9.6 | 18.9 | 26.9 |
| Open a patient by id | 8 | 722 | 9.3 | 22.4 | 40.1 |
| Open a patient by id | 32 | 698 | 42.8 | 65.4 | 99.2 |
| Search by family prefix | 1 | 182 | 4.5 | 13.2 | 19.9 |
| Search by family prefix | 8 | 863 | 7.7 | 19.8 | 36.6 |
| Search by family prefix | 32 | 730 | 35.9 | 91.4 | 132.1 |

Every response was a success (201 or 200); no errors.

### What it shows

- **Audited writes do not scale with concurrency.** Throughput stays between 166 and 194 a second from 1 to 32
  clients while latency grows in proportion, which is what one global lock would produce. The audit chain takes one
  advisory lock per append (`audit.rs`, CQ-18). **This is consistent with the lock being the limit, not proof:** the
  run did not take the lock out, did not look at Postgres wait events and did not vary the pool size. CQ-18's decision
  still waits on that experiment.
- **Reads and searches scale to about 8 clients** and then flatten at 700 to 860 a second on this machine.
- **Against the targets in `spec/07` NFR-2:** "open a patient by id" has a p95 of 18.9 ms at one client and 65 ms at
  32, but that is one identity record, not a chart (no encounters, results or medicines exist yet), so the 1 s chart
  target is **not measured**. "Search" at 13 ms p95 is a database prefix search, not the typeahead index the target
  describes, so the 200 ms target is **not measured** either. FHIR read p50 was not run, and neither was order sign.

### Not done and not verified

- No database-side evidence: no `pg_stat_activity` wait events, no pool-size sweep, no run with the audit append
  removed. The lock is a hypothesis.
- One client process on the same machine as the service and the database VM; the client may be part of the limit.
- Debug-level JSON logging was on, which costs throughput; a production profile would differ.
- Ten seconds per step, one run, no warm-up step, no variance: treat differences under about 15 percent as noise.
- No run on a CI machine, nothing recurring, and no regression gate. The OS-5 acceptance asks for a run on pushes to
  `main`; that is not built.
- Roughly 5,400 patients, not the 1 million of `plan.md` §6.6; no clinical data exists to load.
- No Redis, no authentication, no TLS and no reverse proxy in the path.
- `scripts/load/measure.mjs` is gone; `sh scripts/load/run-local.sh --profile standard` is the nearest run now.
