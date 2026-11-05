# Performance efficiency: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

## Definition

ISO/IEC 25010:2023 defines performance efficiency as the degree to which a product performs its functions within time and throughput limits while using resources (processor, memory, storage, network, energy and others) efficiently. Its sub-characteristics are time behaviour (response times and throughput meet requirements), resource utilization (amounts and types of resources meet requirements) and capacity (maximum limits of system parameters meet requirements) [1]. I saw an iso25000.com summary, not the standard's text (the ISO page gave HTTP 403) [2]; I did not confirm whether 2011 differs.

Two outside framings help set targets. Jakob Nielsen's three limits: about 0.1 s feels instant, about 1 s keeps the user's train of thought, about 10 s is the limit of attention [3]. Google's SRE book names latency, traffic, errors and saturation as the four golden signals, says to separate the latency of successful and failed requests ("a slow error is even worse than a fast error"), and warns that means hide the tail, so latency should be recorded as a distribution [4]. Bass, Clements and Kazman express performance as scenarios with a response measure such as latency or throughput under a stated load [5].

## Why it matters for an EPR

Speed is a patient-safety and workload matter, not a nicety. A KLAS Arch Collaborative report (as summarised by trade press; I saw search snippets only) found that 55 percent of clinicians said response time met expectations, against 75 percent who called their EHR reliable, and that slowness shaped perceived reliability [6]. The same snippets note that slow screens can come from hardware and Wi-Fi, so a service budget is only part of the experience. A vendor-sponsored survey of 99 respondents reported log-in times over 10 s for 60 percent [7]; its size and sponsor make it weak evidence.

Capacity failures are visible at national scale: in October 2022 an Oracle Cerner outage sent UK hospitals to downtime procedures [8], and a 2024 global software update fault disrupted hospitals using several EHRs [9]. These are availability events, covered in [reliability](../reliability/index.md); they are listed because overload and dependency behaviour are the capacity side of this attribute. Nottingham's critical incident after EPR go-live (snippet only) shows that load at go-live is a real planning case [10].

## What the project says today

- **NFR-2** (`spec/07-non-functional-requirements.md`): chart open p95 at most 1 s; patient search or typeahead p95 at most 200 ms; order sign p95 at most 2 s; FHIR read p50 at most 10 ms. The spec says these are targets and nothing has been measured against them.
- **First numbers, one run.** `BENCHMARKS.md` (2026-10-30, OS-5, partly landed): an Apple M4 Max with Postgres 18 in a Podman VM, `scripts/load/measure.mjs`, about 5,400 patients, 10 s per step, every request audited, debug JSON logging on. For example opening a patient by id at 8 clients: p50 9.3 ms, p95 22.4 ms; registering a patient at 32 clients: 194 per second, p95 191 ms. The note says the audit append lock is a hypothesis, differences under about 15 percent are noise, and the 200 ms search target is not measured because the run used a database prefix search, not a typeahead index. FHIR read p50 and order sign were not run.
- **Built controls.** Fixed pagination caps with a `before` cursor; a database pool of 2 to 20 (`axa` production defaults); per-subject rate limits (NFR-6, OS-2, H-039) with 429 and `Retry-After`, per process only; a labelled request-duration histogram at `/metrics` (OS-6). OTLP tracing is not built (waits on OQ-OS4).
- **Planned.** A load test at 1 million patients (task at line 128 of `tasks.md`: search p95 at most 200 ms), a run on pushes to `main` with a regression gate (OS-5), criterion benches and fuzzing for the matcher (SF-2, not built).
- **Cost of integrity.** The write path appends a hash-chained audit row and refuses the write if that cannot be written (NFR-7). That serialises something; the benchmark's flat write throughput across 1, 8 and 32 clients is consistent with that, but the cause is not shown.

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
|---|---|---|---|
| Latency percentiles per route | Histogram of request duration, p50, p95, p99, success and failure apart [4] | Start from NFR-2; set each from a task budget (Nielsen limits [3]) and keep a margin for the browser and network | `/metrics` histogram (OS-6); `BENCHMARKS.md` |
| Throughput at a fixed concurrency | Requests per second at 1, 8, 32, then more clients | Record, do not target, until a real deployment size is named | `scripts/load/measure.mjs` |
| Saturation | Pool wait time, CPU, lock waits (`pg_stat_activity`), queue depth | Alert before the pool is exhausted; value set from a sweep | not collected; listed as not done in `BENCHMARKS.md` |
| Capacity | Patients, audit rows, observations at which p95 breaks target | Test at 10 times the intended site, then trim | 1M-patient load test (planned) |
| Resource use | Memory and CPU per replica at steady load; image size | Fit the smallest hardware named in the equity rule for a low-resource site | not measured |
| UI cost | Largest contentful paint, JS bytes, interaction latency on a mid-range phone | Budget per page, set after one measurement on a named low-end device | Playwright (candidate); [mobile viewability](../../spec/mobile-viewability.md) |
| Regression | Change in p95 against the previous `main` run | Fail above a noise band measured from repeated runs, not a guess | OS-5 gate (not built) |
| Tail under audit | Latency with audit append on versus off | Quantify the integrity cost; accept or tune with the Clinical Safety Officer | not run |

## Trade-offs with other attributes

- **Auditability and security** cost time: every read is audited (NFR-7) and policy is checked on each call. The project's choice is to refuse an unaudited write, not to skip audit for speed ([auditability](../auditability/index.md), [security](../security/index.md)).
- **Reliability.** Rate limits protect capacity but are a refusal a clinician could meet; the defaults sit far above clinical use and are hazard H-039 ([reliability](../reliability/index.md)).
- **Interoperability.** Bulk `$export` and subscriptions compete with clinical reads for the same database; they need their own limits ([interoperability](../interoperability/index.md)).
- **Interaction capability.** Perceived speed depends on feedback in the UI as much as on the server ([interaction capability](../interaction-capability/index.md)).
- **Flexibility.** Country packs and configuration add lookups on the hot path ([flexibility](../flexibility/index.md)).

## Gaps and risks

1. Targets are unmeasured: no chart, typeahead, order sign or FHIR read result exists.
2. Benchmarks are one run, one machine, a client on the same host, debug logging on.
3. No clinical data yet, so no realistic query shapes (a chart is many resources, not one row).
4. No regression gate; a slowdown would pass CI.
5. Rate limits are per process, so N replicas allow N times the limit.
6. Low-resource and slow-network use is promised by the equity rule but has no budget.

## Ideas

1. Run `measure.mjs` on `main` in CI on a fixed runner and store the JSON (OS-5); fail only outside a measured noise band.
2. Add FHIR read and a synthetic chart-open step so NFR-2 rows each have a number, even a poor one.
3. Repeat the audit-append test with the append disabled to turn the lock hypothesis into a result (extends OS-5).
4. Seed 1 million patients and a plausible clinical load for the planned capacity test.
5. Define a "low-resource profile" (one core, 2 GiB, 3G latency) as a named test target, tied to the equity rule.
6. Write NFR-2 as six-part scenarios with load stated [5].
7. Add pool wait and saturation metrics to `/metrics` before OTLP (OS-6).

## Open questions

- What deployment size (patients, concurrent clinicians) is the reference for capacity?
- Is the 1 s chart-open budget server-side or end to end including the browser?
- Should audit writes be batched or asynchronous? That conflicts with refusing an unaudited write; this needs the Clinical Safety Officer.

## Not verified

- ISO/IEC 25010:2023 wording (summary only).
- KLAS figures: seen in trade-press snippets, not the report.
- Every performance number in this note is copied from `BENCHMARKS.md`; I ran nothing.
- The 2022 and 2024 outage reports were seen as snippets.

## Sources

1. https://iso25000.com/index.php/en/iso-25000-standards/iso-25010 : sub-characteristics of performance efficiency (summary).
2. https://www.iso.org/standard/78176.html : ISO catalogue page (403 on fetch).
3. https://www.nngroup.com/articles/response-times-3-important-limits/ : Nielsen, 0.1, 1 and 10 second limits.
4. https://sre.google/sre-book/monitoring-distributed-systems/ : four golden signals, distributions, slow errors.
5. https://people.ece.ubc.ca/matei/EECE417/BASS/ch04lev1sec3.html : quality attribute scenario parts.
6. https://KLASresearch.com/archcollaborative/report/ehr-response-time-and-reliability/423 : KLAS EHR response time and reliability (snippet).
7. https://www.healthcareitnews.com/news/docs-struggle-slow-clinical-information-systems : log-in survey, 99 respondents (snippet).
8. https://www.theregister.com/2022/10/12/oracle_cerner_outage_uk/ : Oracle Cerner UK outage, October 2022 (snippet).
9. https://ukauthority.com/articles/nhs-systems-recover-from-global-it-outage : NHS recovery from the 2024 global outage (snippet).
10. https://www.digitalhealth.net/2025/11/nottingham-hospitals-declares-critical-incident-after-epr-go-live : Nottingham critical incident (snippet).
