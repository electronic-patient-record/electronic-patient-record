# Benchmarks

**One laptop run on 2026-10-30. Not a capacity claim.** Nothing here says what a deployment can carry; it says what
this build did once, on one machine, with one client on the same machine.

| | |
|---|---|
| Build | release profile of commit `fdcd091` plus uncommitted working-tree changes (OS-9, OS-4) |
| Machine | Apple M4 Max, 16 cores; Postgres 18 in a Podman VM with 8 CPUs and 12 GiB |
| Service | `container-dev` profile: login off, pool 2 to 20, JSON logs at debug level, hash-chained audit on |
| Client | `scripts/load/measure.mjs` (Node 26, `fetch`), on the same host; raw numbers in `scripts/load/results-2026-10-30.json` |
| Data | 200 seeded patients plus whatever the write steps added (about 5,400 in total); 10 seconds per step |

## Results

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

## What it shows

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

## Not done and not verified

- No database-side evidence: no `pg_stat_activity` wait events, no pool-size sweep, no run with the audit append
  removed. The lock is a hypothesis.
- One client process on the same machine as the service and the database VM; the client may be part of the limit.
- Debug-level JSON logging was on, which costs throughput; a production profile would differ.
- Ten seconds per step, one run, no warm-up step, no variance: treat differences under about 15 percent as noise.
- No run on a CI machine, nothing recurring, and no regression gate. The OS-5 acceptance asks for a run on pushes to
  `main`; that is not built.
- Roughly 5,400 patients, not the 1 million of `plan.md` §6.6; no clinical data exists to load.
- No Redis, no authentication, no TLS and no reverse proxy in the path.
- Reproduce with the commands at the top of `scripts/load/measure.mjs` (Postgres from `compose.test.yaml`,
  `db migrate`, `LOCO_ENV=container-dev`, a release build).
