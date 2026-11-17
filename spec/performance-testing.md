# Performance testing: load tests and benchmarks

Status: built 2026-11-15 (OS-5, SF-2). How this repository measures speed, what the numbers are for, and what they
are not. Results go in [BENCHMARKS.md](../BENCHMARKS.md); targets are [NFR-2](07-non-functional-requirements.md).

**No number here is a capacity claim.** A run says what one build did on one machine, once. What a deployment can
carry depends on its hardware, its database, its network and its users, none of which this repository has.

## Two kinds of measurement

| | Benchmarks | Load tests |
|---|---|---|
| What | One function, in a loop, in one process | The running service, over HTTP, with its database |
| Tool | criterion 0.8 (`benches/` in the matcher and the service) | `scripts/load/run.mjs` (Node 26, no dependencies) |
| Answers | Did this code get slower? Which part of a request costs what? | What latency does a user see, and where does it break? |
| Database | None | A scratch PostgreSQL 18 database, made and dropped per run |

### Benchmarks

- **Matcher** (`electronic-patient-record-matcher-with-rust/benches/matcher.rs`): name folding (four names, from
  ASCII to Welsh and accented), transliteration, Soundex, nicknames, partial dates, postcodes, phone numbers, the
  national number and IHI checks (valid and invalid), one comparison in six shapes (identical, a typo, a nickname
  with no identifier, a conflicting identifier, a different person, a sparse record), and one record scored against
  1, 50 and 1,000 candidates. 50 is the most the service's search returns.
- **Service** (`electronic-patient-record-service-with-rust/benches/service.rs`): the work each request does
  outside the database. The audit row's digest (unkeyed SHA-256, computed inside the chain's lock, CQ-18), the
  policy decision in three cases, parsing and validating a registration, a comparison with the site's rules, an
  observation's validation and flag, an HL7 v2 registration parsed, and the request-duration histogram. Added
  2026-11-17: the audit digest under a key (HMAC-SHA-256, as a production deployment computes it) and the keyed
  fingerprint that replaces an HL7 control id (IH-3); a patient's consents reduced for one disclosure, with 1, 10 and
  50 active consents (SE-2); and the rest of an HL7 message's work outside the database: its message time (IH-5), the
  masked shape a log line may show (IH-4), the acknowledgement for an accepted and a kept message and a long `MSA-3`
  (IH-2), ten pipelined MLLP frames taken from one read, and a feed's health judged (IH-8).
- `cargo bench --bench <name>` measures. `cargo bench --bench <name> -- --test` runs each benchmark once without
  measuring: that is the gate (`matcher-bench`, `service-bench`), so a benchmark that no longer builds or panics
  fails. `cargo clippy --all-targets` compiles them on every push.
- criterion keeps its last result in `target/criterion/`, so a second run on the same machine prints the change.
  That comparison is the regression check for benchmarks. It is local only: no stored baseline is kept in the
  repository, since another machine's numbers would not compare.

### Load tests

`scripts/load/run.mjs` drives the service; `scripts/load/run-local.sh` builds a release binary, makes a scratch
database (`scripts/scratch-db.sh`), migrates it, starts the service with login off (`APP_ENV=container-dev`) on a
free port with log level `warn` and a pool of 20, runs the harness, and removes it all after.

**Scenarios.** Each is one thing a user or a system does, timed end to end. Every request the service answers is
audited, so every scenario also measures the audit chain.

| Scenario | Requests |
|---|---|
| `register` | `POST /api/patients`: validation, the duplicate check, the write, the audit row |
| `read` | `GET /api/patients/{id}` |
| `search` | `GET /api/patients?family=` by prefix, at most 50 back |
| `chart-open` | The reads the chart page makes, side by side as the page makes them: the patient, allergies, observations, results, referrals, adjustments, watch list and death status (8 requests); the time is the slowest |
| `fhir-read` | `GET /fhir/R4/Patient/{id}` |
| `fhir-search` | `GET /fhir/R4/Patient?family=` |
| `compare` | `POST /api/patients/compare`: the matcher and an audit row |
| `record-observation` | `POST /api/patients/{id}/observations` (a heart rate) |
| `record-allergy` | `POST /api/patients/{id}/allergies` |
| `hl7-register` | An HL7 v2 A28 for a new patient over MLLP: read, matched, created, audited, answered `AA` (only with `--mllp`) |
| `hl7-update` | An A08 changing the phone of a patient the feed registered: matched, the order check (IH-5), updated, `AA` (only with `--mllp`) |
| `mixed` | A mix by weight: chart-open 30, search 20, read 15, fhir-read 10, record-observation 10, register 5, record-allergy 5, compare 5. **A guess, not a measured clinic**: no usage data exists |

**The HL7 feed.** `run-local.sh` opens the listener on a free loopback port (`EPR_MLLP_BIND`, 64 connections,
agent `mllp:load`; `EPR_LOAD_MLLP=0` leaves it off) and passes `--mllp host:port`; without it the `hl7-*` scenarios
are left out of the default list, and naming one is refused. Connections are pooled and reused, as a sender keeps
one open. `AA` is a success; `AE`, `AR` or no answer is a failure. Seeding registers a fifth as many feed patients as
REST ones (at least one) for `hl7-update`. Message times are now, to the second, so the order check applies every
update. NFR-2 names no feed target, so the budget is no failures only. The feed's messages are not in `mixed`.

**Data.** Before the timed steps the harness registers synthetic patients (`Load<stamp><letters>`, birth dates
spread so the duplicate check refuses none, no identifier), each with an allergy and three observations, so a chart
has something in it. A refused seed stops the run with exit 3: numbers about refusals would mean nothing.

**Two ways to drive load.**

- *Closed loop* (`--concurrency`): N workers, each sending its next request when the last one answers. It shows
  what the service sustains. When the service slows, so does the load, which hides queueing.
- *Open loop* (`--rate`): requests sent at a fixed rate whatever the service does. Each latency runs from when the
  request was due, not when it was sent, so a stall is charged to every request it delayed (no coordinated
  omission). Past `maxInFlight` (1024) a due request is not sent; it counts as dropped and failed.

**Profiles.**

| Profile | What | Checks |
|---|---|---|
| `smoke` | Every scenario at 2 workers for 3 s, 20 patients | Failures only |
| `standard` | Every scenario at 1, 8 and 32 workers, 10 s each after 2 s of warm-up, three times (the median, with the spread of p95), 200 patients | Failures and latency |
| `stress` | `mixed` with workers doubling from 1 to 256, stopping at the first level that breaks a budget | Failures and latency |
| `soak` | `mixed` open loop at 50 a second for 10 minutes in 30 s windows | Failures, and drift: the last window's p95 within 1.5 times the first's |

**Budgets** (`scripts/load/budgets.json`). From NFR-2: `chart-open` p95 ≤ 1 s, `search` and `fhir-search` p95
≤ 200 ms, `fhir-read` p50 ≤ 10 ms; `mixed` p95 ≤ 1 s with at most 1 percent failures; every other scenario no
failures. A budget applies at every concurrency a run uses, since NFR-2 names no load. A failure is a 4xx or 5xx
answer, or none. `--no-latency-budgets` checks failures only, whatever the profile.

**Regression.** `--baseline <earlier run.json>` compares step by step and fails a step whose p95 grew, or whose rate
fell, by more than `--tolerance` (default 0.15, about the noise of one laptop run).

**Output.** A table on standard output and, with `--out`, a JSON file carrying the statement that it is one run on
one machine, the commit and whether the tree was dirty, the host (OS, CPU, cores, memory), Node's version, the
settings, every step, the budget checks, drift and the comparison. Exit 0 passed; 1 a budget, drift or regression
failed; 2 a usage error; 3 the service could not be reached or refused the seeding.

## Where each runs

| | Every push | `ci-local.sh --full` | Nightly CI (`performance` job) |
|---|---|---|---|
| Harness tests (`docs-load-harness`: statistics, both loops, budgets, baseline, options, whole runs against a fake service) | yes | yes (`--fast` too) | yes (docs job) |
| Benchmarks compile (`clippy --all-targets`) | yes | yes | yes |
| Benchmarks run once (`matcher-bench`, `service-bench`) | no | yes | yes |
| Load smoke (`service-load-smoke`) | no | yes | yes |
| Standard load run, kept as an artefact | no | no | yes, failures checked, latency recorded not checked |

Latency is not a gate anywhere in CI: a GitHub-hosted runner shares its machine, and its numbers would fail and
pass by chance. Latency budgets are checked by a person running `standard` or `stress` on a known machine and
recording it in `BENCHMARKS.md`.

## Not done and not verified

- **Chart open is still thin.** The 8 reads return little: no encounters, medicines or notes exist yet, so the 1 s
  target is measured against a chart much smaller than a real one.
- **Search is not a typeahead.** It is a prefix search in the database; the typeahead index NFR-2 describes does not
  exist. Search is also over a few thousand synthetic patients, not the 1 million of `plan.md` §6.6.
- **Login is off.** Token verification, the request limits (which count only a token subject) and the role views
  are not in the path; nor are TLS, a reverse proxy, Redis or the UI's backend-for-frontend.
- **Order sign (NFR-2, p95 ≤ 2 s) is not measured:** no orders exist.
- **One client process,** on the same host as the service and the database, may itself be part of the limit.
- **The mix is a guess** with no usage data behind it.
- **The audit lock (CQ-18) is still a hypothesis.** The harness can show that audited writes flatten; it does not
  look at database wait events or run without the append.
- **The HL7 feed is plain TCP on loopback** in the load run: no TLS, no client certificates, one sender's rules.
- **The 2026-11-17 benchmarks and the `hl7-*` scenarios have not been measured yet**: they were checked to run, while
  other builds were using the machine; `BENCHMARKS.md` has no numbers for them.
- **No stored baseline.** `--baseline` needs a run from the same machine; none is kept in the repository.
