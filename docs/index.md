# System quality attributes

Research read 2026-11-04 plus this repository's own files (requested by the author). One document per attribute. The basis is the ISO/IEC 25010:2023 product quality model (nine characteristics); **interoperability** (a sub-characteristic of compatibility in 25010), **privacy** and **auditability** (not separate in 25010) get their own documents because they are central to an EPR. Every document has the same shape: definition, why it matters for an EPR, what the project says today, measures, trade-offs, gaps, ideas, open questions, what was not verified, sources. **These are proposals and readings: no target is a commitment, and the ISO wording was read from secondary summaries, not the standard.**

| Attribute | In 25010:2023 | What the project says (requirement) |
|---|---|---|
| [Functional suitability](functional-suitability/index.md) | characteristic | the FR-* list ([spec/06](../spec/06-functional-requirements.md)) |
| [Performance efficiency](performance-efficiency/index.md) | characteristic | NFR-2 |
| [Compatibility](compatibility/index.md) | characteristic | NFR-5, IO-* |
| [Interoperability](interoperability/index.md) | sub-characteristic of compatibility | [spec/interoperability](../spec/interoperability/index.md) |
| [Interaction capability](interaction-capability/index.md) | characteristic (was usability) | NFR-4, NFR-10 |
| [Reliability](reliability/index.md) | characteristic | NFR-3 |
| [Security](security/index.md) | characteristic | NFR-6 |
| [Privacy](privacy/index.md) | not separate | NFR-6, [DPIA](../governance/dpia.md) |
| [Auditability](auditability/index.md) | accountability and non-repudiation | NFR-7 |
| [Maintainability](maintainability/index.md) | characteristic | NFR-9, NFR-11 |
| [Flexibility](flexibility/index.md) | characteristic (was portability) | packs (OS-10), tenancy (SE-5) |
| [Safety](safety/index.md) | characteristic (new in 2023) | NFR-1; governed by [measuring clinical safety](../spec/clinical-safety/measurement.md) |

Specifications live in [`../spec/`](../spec/index.md); the plan is [`../plan.md`](../plan.md) §30.
