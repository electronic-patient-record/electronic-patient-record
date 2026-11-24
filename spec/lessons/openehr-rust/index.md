# openEHR and `openehr-rust`: lessons for this project

Read 2026-11-24 from a local checkout of `openehr-rust/openehr-rust` at commit `320e1e84` (crates 0.11.0, with unreleased changes): the README, `COMPARISONS.md`, the changelog since 0.10, the production readiness section of `plan.md`, `openehr/spec` sections 4 to 8, 11 and the headings of 15, `openehr-store/src/store.rs` (its documentation), the terminology tables, and the `openehr-axa` README. **The crates are by this project's author**, so this is one hand comparing two designs and not an outside review. No test of theirs was run, no openEHR specification was read directly, and no archetype was opened. It extends [the 2026-10-21 comparison](../../interoperability/openehr-comparison.md) of version 0.10. Plan: [`plan.md`](../../../plan.md) §40. Tasks EH-1 to EH-17.

openEHR® is a registered trademark of the openEHR Foundation. This project claims no openEHR conformance and implements no openEHR interface.

## Why it was read

Customers want more electronic health record (EHR) capabilities. openEHR is a specification of what a health record **is**, where FHIR is mostly a specification of what systems exchange. So the question here is not "which crate to depend on" but "what does a record do, by this model, that ours does not".

## Advice in brief

1. **Still do not depend on the crates.** The reasons of 2026-10-21 hold: a second clinical model beside the FHIR-shaped one, with a mapping nobody has specified. Their own `COMPARISONS.md` says to choose another tool for a running clinical data repository, and their archetype support is partial (16 of 34 requirements have no code, by their plan). Revisit for a deployment that must exchange openEHR compositions (EH-14).
2. **Take the record-keeping ideas.** The gaps below are in what our record can **say**, not in how it is stored: our append-only store, chained audit, consent and policy are ahead of theirs.
3. **Build first what the stored data can already answer, or what customers have asked for.** That gave the three items built on 2026-11-24.

## What was found, and what to do

| openEHR idea (their requirement) | This project before 2026-11-24 | Advice | Task |
|---|---|---|---|
| `EVALUATION`: an opinion at the time of recording; superseding it is a new version, not an event (`E6.17`); certainty apart from status | No problem list at all | **Built**: [problem list](../../problem-list/index.md) | EH-1 |
| A persistent composition has no event context: a running list belongs to the patient, not a visit (`E6.6b`) | Nothing to apply it to | **Built** into EH-1: a problem has no encounter and refuses one | EH-1 |
| `LINK` between entries (`M5.9`) | None | **Built** for one link, a problem's cause: the multi-system condition of CF-4 | EH-1 |
| `version_at_time`: what was current then, with a version whose time is not established skipped, not guessed (`V8.6`) | The versions were stored and no route could ask | **Built**: [reads as at a time](../../as-at/index.md) | EH-2 |
| An entry's subject is explicit; an entry about a relative that does not say so reads as the patient's (`E6.14`, `M5.13a`) | No relative history | **Built**: [relative history](../../relative-history/index.md), its own kind of record | EH-3 |
| Partial values are compared only where that is established (`D3.14`) | The matcher did; clinical dates did not | **Built** for problem dates (`dates::partial_order`) | EH-1 |
| `INSTRUCTION` and `ACTION` with one state machine for every order (`E6.18` to `E6.23`); the order's narrative is the human's words, never generated | No orders, no medicines | Use one state machine for OE and MA before either is built, and keep the narrative | EH-4 |
| `CONTRIBUTION`: one audit for a set of versions (`V8.15`, `V8.16`) | One write, one audit row | Add before the first screen that saves several things together | EH-5 |
| `EHR_STATUS`: `is_modifiable` and `is_queryable` apart (`E6.3`, `E6.4`) | A merged record is read-only; nothing else is | Add: death, transfer and opt-out need a record that is closed to writing and open to reading | EH-6 |
| Four null flavours (`R4.5` to `R4.7`) | Observations have a reason for absence; others do not | Use the same four wherever a value may be absent; *masked* is the one SE-4 needs | EH-7 |
| Redaction masks and counts, and never says what (`X11.20` to `X11.24`) | `Mask` is decided and nothing applies it | The design for SE-4; CF-2 (custody) is blocked on it | EH-8 |
| Version lifecycle: incomplete, complete, inactive, abandoned (`V8.8`) | Every version is complete | Needed by drafts (CN-1) | EH-9 |
| `ATTESTATION` with the view that was signed (`V8.19`) | A result sign-off names a version, not what was on the screen | Keep what was shown | EH-10 |
| `FEEDER_AUDIT`: where an entry came from and its original content (`M5.10`, `M5.11`) | The HL7 feed keeps refused messages | Add to entries that arrive from elsewhere | EH-11 |
| `PARTICIPATION`, and the care **setting** of an event | Author only | Add with encounters (CA-1); the setting list also helps CF-2 and CF-3 | EH-12 |
| Archetypes and templates: content defined as data | Assessment forms are versioned data | Keep to forms and packs; watch their archetype work | EH-13 |
| Canonical JSON | FHIR R4 and R5 | A one-way export, only for a deployment that needs it | EH-14 |
| AQL | Fixed routes | Not adopted: SQL-on-FHIR is the plan (IO-7) | none |
| `DELETE` answers 501, a deleted composition 410, a deletion is a version (`V8.10`) | No delete path | Same stance; nothing to do | none |

## Found in this repository while comparing

- **A refused allergy put the SNOMED CT code it was sent into its audit row** (`controllers/allergies.rs`), against the module's own rule. Fixed on 2026-11-24 with a test that failed first. Fourteen other controllers recorded the store's words for a refusal too; all now record its kind (EH-16, done the same day).

- **One thing not to copy**: the `subject_relationship` group has one code for "step **or** half brother" and one for "step **or** half sister", which joins no shared genes with a quarter. [Relative history](../../relative-history/index.md) asks for the genetic link instead.

## Not done, not verified

No openEHR specification, archetype or conformance test was read or run; what the table says of openEHR is what the crate's requirements say. No clinician, customer or openEHR practitioner was asked. Which EHR capabilities customers want most was not asked: the order is the author's. The readiness figures are theirs.
