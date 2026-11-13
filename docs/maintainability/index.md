# Maintainability: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

## Definition

ISO/IEC 25010:2023 keeps maintainability as one of nine product quality characteristics. Its sub-characteristics are modularity, reusability, analysability, modifiability and testability. The 2011 edition had the same five names; the 2023 edition's change notes I saw (a summary at arc42) mention no change to maintainability ([arc42 summary](https://quality.arc42.org/articles/iso-25010-update-2023)). I did not read the standard itself, only summaries and search snippets, so the wording of each sub-characteristic below is the usual reading, not a quotation.

- Modularity: a change to one component has little effect on others.
- Reusability: an asset can serve in more than one system.
- Analysability: how easily the impact of a change, or the cause of a fault, can be found.
- Modifiability: the product can be changed without introducing defects or degrading quality.
- Testability: test criteria can be set and tests run to decide whether they are met.

The SEI quality attribute scenario method (source, stimulus, artefact, environment, response, response measure) has separate chapters for modifiability and testability in the 4th edition of Software Architecture in Practice. I saw only the contents list ([Pearson](https://www.pearson.com/en-us/subject-catalog/p/software-architecture-in-practice/P200000000111/9780137468218)).

## Why it matters for an EPR

- Maintenance dominates the life of a record system. A study of Kaiser Permanente Northern California's 21 hospitals found ongoing maintenance needs to be substantial and pervasive: six years of changes touched more than 130 EHR tools and 150 user roles, and the authors note that far less attention has gone to maintenance than to implementation ([Liu et al., AJMC 2019](https://ajmc.s3.amazonaws.com/_media/_pdf/AJMC_01_2019_Liu%20final.pdf); seen as a search snippet and its first page).
- Customisation debt is a documented risk. Trade press reports Cone Health investing at least 40 million US dollars to rebuild the foundation of its Epic system after years of customisation ([Becker's](https://www.beckershospitalreview.com/healthcare-information-technology/ehrs/cone-health-invests-40m-to-rebuild-epic-after-years-of-technical-debt/); snippet only, a single report).
- In a clinical product, a change that is hard to analyse is a safety issue: IEC 62304 expects a documented software lifecycle including maintenance and problem resolution ([overview](https://jamb.readthedocs.io/en/latest/_sources/iec-62304/overview.md.txt); a secondary summary).

## What the project says today

- Layout: three projects (service, matcher, UI) with separate guides ([AGENTS.md](../../AGENTS.md)); the matcher is a pure library with no I/O ([NFR-9](../../spec/07-non-functional-requirements.md)). Module groups and slots are in the [module map](../../spec/19-module-map.md), proposed and unconfirmed (OQ-M1).
- Spec first: a behavioural change is one commit with spec, code and test edits ([spec-driven development](../../agents/spec-driven-development.md)). This is a modifiability and analysability control by process.
- Census and guard tests are the project's main testability device: route census, OpenAPI freshness (`tests/openapi_census.rs`, OS-7), documentation census (`tests/doc_census.rs`, OS-9), the audit-refusal census (CQ-39), error census (CQ-11). The [testing strategy](../../spec/11-testing-strategy.md) says no comparator found had the census tests or the hand-run mutation pass.
- Coverage (OS-4): floors of 95 percent of lines for the matcher and 50 for the service's database-free tests, UI floors of 90 percent lines and 84 percent branches; measured 97.65 and 52.24 on 2026-10-30 ([testing](../../agents/testing.md)). The jobs have never run on GitHub (tasks.md OS-4). Line coverage shows what ran, not what was asserted.
- Mutation testing: `cargo mutants` on the matcher's `engine.rs` and `national_number.rs`: 147 caught, 5 unviable, none missed (CQ-30, CHANGELOG 2026-10-23). A weekly mutation run was considered and not adopted (PR-8); the reasons are the author's and unmeasured.
- Dependency hygiene: pinned toolchain, Dependabot, cargo-deny, SBOM ([NFR-11](../../spec/07-non-functional-requirements.md)). `axum-test` must follow `axa`, a coupling AGENTS.md records: a second copy gives errors that say `TestServer` does not match `TestServer`.
- Not done: typed service layer (CQ-13) covers only the handlers FHIR uses; other handlers still build a `Response`. Store race tests exist for 2 of 6 stores (CQ-12).

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
|---|---|---|---|
| Line and branch coverage | `cargo llvm-cov`; `vitest --coverage` | Ratchet: a floor never lowered without a written reason | `coverage` job (OS-4), unrun on GitHub |
| Mutation score on safety-relevant code | `cargo mutants` per file | Zero missed on matcher and consent rules; report survivors per file | Hand-run; CHANGELOG (CQ-30) |
| Census coverage | Count of route, write and doc census tests against routes, writes and documents | Each new route, write or document fails CI without a row | `tests/*census*` |
| Lead time of a change | Commit timestamps of spec, code and test for one item | Watch the trend, set no number first | git history (not collected) |
| Change coupling | Files changed together per item | A rising count of cross-project files signals poor modularity | git history (not collected) |
| Dependency age | Days behind latest compatible release | Set after one baseline reading | Dependabot, `cargo deny` |
| Time to analyse a fault | Time from a failing request to the owning module, in a timed exercise | Choose after a first exercise | Not collected |
| Documentation drift | Doc census failures per month | Zero on `main` | `doc_census`, `generate-status.sh --check` |

## Trade-offs with other attributes

- Census tests raise analysability and safety but add edit cost to every route (more files per change); the workflow accepts that on purpose.
- Strict coverage floors can reward assertion-free tests; mutation checks counter that at a time cost.
- A pack mechanism (see [flexibility](../flexibility/index.md)) improves modifiability for jurisdictions but adds a validation surface to maintain.
- Performance work ([performance-efficiency](../performance-efficiency/index.md)) can push toward denormalised views that cost analysability.
- Append-only data ([auditability](../auditability/index.md)) makes data migrations harder to reverse.

## Gaps and risks

- One maintainer, so the bus factor is one; census tests lower but do not remove the risk.
- Coverage jobs, CodeQL and several workflows have never run on GitHub (no remote at the time of writing in the spec).
- The service floor of 50 percent excludes database-gated tests, so the part that matters most for the record is outside the number.
- No mutation run over the service, the UI or most matcher files.
- Machine-written translations and many specs are large single documents; analysability of tasks.md (long entries) is already strained.

## Ideas

1. Put mutation runs on the consent, policy and audit-refusal code, run by hand per release, record survivors (extends CQ-30; weekly run stays declined, PR-8).
2. Add a coverage ratchet file so floors can only rise (OS-4).
3. Collect lead time and change coupling from git into `scripts/generate-status.sh` output (new task, not yet numbered).
4. Finish the typed service layer (CQ-13) so every handler is testable without HTTP.
5. Add an architecture decision record folder if none exists, to aid analysability (new task).
6. Time a fault-finding exercise with a new contributor once there is one.

## Open questions

- Is one item per commit still workable once more than one person contributes?
- Which files deserve mutation testing first by hazard weight?
- Should the service floor count database-gated tests once CI has Postgres?

## Not verified

I read none of the ISO text itself. No coverage, mutation or build figure was re-run by me; all figures are quoted from repository files. The Becker's and AJMC items are from snippets.

## Sources

- https://quality.arc42.org/articles/iso-25010-update-2023 - summary of 2023 changes
- https://www.pearson.com/en-us/subject-catalog/p/software-architecture-in-practice/P200000000111/9780137468218 - SEI book contents
- https://ajmc.s3.amazonaws.com/_media/_pdf/AJMC_01_2019_Liu%20final.pdf - EHR maintenance study (Kaiser)
- https://www.beckershospitalreview.com/healthcare-information-technology/ehrs/cone-health-invests-40m-to-rebuild-epic-after-years-of-technical-debt/ - Cone Health rebuild (snippet)
- https://jamb.readthedocs.io/en/latest/_sources/iec-62304/overview.md.txt - IEC 62304 overview (secondary)
