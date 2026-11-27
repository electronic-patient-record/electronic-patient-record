**Keep the project on synthetic data.** The audit found reproducible defects in patient matching and chart refresh, alongside gaps in recovery and release verification that need closing before a clinical pilot.

[Read the full production-readiness audit and agent coding backlog](gpt-6-2-astra-high.md). It contains seven prioritized engineering tasks with source references, affected hazards, and acceptance tests.

| Priority | Finding | Acceptance criterion |
|---|---|---|
| 1 | The matcher returned `AutoLink` with score 1.0 when one record carried two distinct valid national numbers and shared only one with the other record. It also returned `AutoLink` for contradictory birth years. A person still has to authorize a merge. | Both cases produce an explicit conflict in either input order, through the matcher and the service API. |
| 2 | A background chart refresh that received 403 retained the patient on screen and marked the data stale. | Confirmed loss of access clears the chart and disables its actions. A late response cannot restore it. |
| 3 | When version 3 arrived before version 2, the refresh function displayed version 2. It also ignored a photo withdrawal and merge destination when the identity version stayed constant. | The chart preserves the newest authorized state, updates independent banner fields, and protects unsaved edits. |

The remaining four tasks cover replacement photos after withdrawal, authorization of stored retry responses, consistency between results and signatures during concurrent updates, and registry tags published before release checks finish.

**For production readiness, prioritize a witnessed restore alongside those fixes.** The repository sets a five-minute recovery-point objective and a four-hour recovery-time objective. Its readiness register records no backup procedure. A synthetic restore should demonstrate the recovered patient version, signature, audit checkpoint and pending delivery, with elapsed time recorded. [Recovery requirements](../../spec/07-non-functional-requirements.md) · [Readiness register](../../governance/production-readiness.md)

**Verification:** 136 matcher tests, 284 service-domain tests, and 441 UI unit tests passed. Two live-Redis tests skipped. Six load-harness tests failed in the restricted environment when loopback use was denied. The chart findings were verified against the actual refresh function with controlled responses. Full database, browser, mobile, partner and recovery runs remain unverified.

I saved the report in the existing empty audit file against local commit `e4853853`. The report is uncommitted.