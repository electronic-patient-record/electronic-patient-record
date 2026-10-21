# The `openehr` crate compared with this project

Written 2026-10-21 from `openehr` 0.10.0 (crates.io), read in full where it matters and run (its
examples 04 and 05, built with the repository's Rust 1.99.1). **The crate is the same author's
work as this project's other crates** (`hl7`, `er7`, `fhir`, `snomed`, the matcher), so this is a
comparison of two designs by one hand, not an outside review. Nothing here is a claim about
openEHR conformance: the crate states its own conformance gaps in its `spec/audit.md`, and I have
not tested them.

## Verdict

**Do not depend on `openehr` now. Take six ideas from it.** The plan already says the same
(`plan.md`: openEHR is *not* the persistence model in 1.0; study its versioning and archetypes).
The crate does not change that:

- It models the openEHR *Reference Model*. This project stores FHIR-shaped resources (plan §6.2), so
  adopting the types would mean a second clinical model next to the first, with a mapping between
  them that nobody has specified.
- It needs Rust **1.97**. This repository documented a minimum of 1.96 when this was written; on
  2026-10-21 the minimum became 1.97 (the current release, 1.99, minus two), so the version is no
  longer an objection. The other reasons below still hold.
- There is nothing for it to hold yet: no clinical content exists here (RC-1 onwards).
- It is small to carry (7 direct dependencies, 40 crates in all, no network code), so the cost of
  waiting is low.

Revisit if a deployment must exchange openEHR compositions (an EHRbase integration, say).

## Side by side

| Concern | `openehr` 0.10 | This project today | Reading |
|---|---|---|---|
| Append-only history | `VERSIONED_OBJECT`: a commit naming a predecessor that is not the latest is refused (V8.4), each refusal distinguishable (V8.5); deletion is a version (V8.10) | `*_versions` tables with `UPDATE`/`DELETE`/`TRUNCATE` refused by triggers; stale `expected_version` or a race is 409; no delete, `MarkEnteredInError` is a later version | **Same design.** Ours is enforced by the database as well as by code; theirs is in-process only (persistence is out of its scope) |
| Atomic multi-object change | `CONTRIBUTION`: one audit for a set of versions | None. A write and its audit row are two transactions (known gap, `tasks.md` AU-1) | **Theirs is ahead.** The contribution idea is the fix for our gap |
| Tamper-evident chain | Over *versions*, in memory: unkeyed or HMAC-SHA-256, key id travels with each tag (additive rotation), constant-time compare, `genesis_after` and `resume_from` (never backfill), `checkpoint()` for an **external witness** | Over *access events*, in the database: keyed MAC via `integrity-mac` (key ids and retired keys supported), advisory lock, triggers, verified in batches, `GET /api/audit/verify` | **Different layer, same primitive.** Both are keyed-or-unkeyed hash chains; see the three gaps below |
| What a verification can say | Five distinct answers: empty, unkeyed only, verified, broken, **unknown key** (reported as a key problem, *not* tampering) | `intact` or `broken` | **Theirs is more honest**: ours would report an unheld key as tampering |
| Removed newest rows | Not detected by the chain alone, but `checkpoint()` exists so the head can be published somewhere the database owner cannot write | Not detected, and nothing to publish (stated in the verify response) | **Partly closed 2026-10-21 (the unheld-key distinction is also done, as `RowCheck::KeyNotHeld` and 424):** the verify answer carries `head` and accepts a checkpoint; publishing it is up to the operator |
| Access control | `EHR_ACCESS` with default-deny, a reference scheme, lossless carriage of schemes it cannot evaluate; dispatch by scheme name, never by object shape | Roles × resources × actions × labels × relationship × purpose × consent × break-the-glass (`policy.rs`, 1,008,000-request matrix) | **Ours is far richer** and is the only one that decides. Their rule that an unknown scheme is carried and **denied**, never reinterpreted, is worth copying for consent |
| Redaction | Masks as `272｜masked｜`, keeps the document valid, reports a *count* of what was withheld, never the content | Policy returns a `Mask` obligation; nothing applies it (SE-4 not built) | **Theirs is a worked design for SE-4** |
| Absence | Four null flavours: no information, unknown, masked, not applicable. The crate refuses to let them collapse | `None` plus a reason in the matcher; `null`, never zero | **Same principle; theirs is finer.** Matters for allergies and observations, not for identity |
| Partial values | `semantic_cmp` returns `None` for a month against a day inside it, `5 mg` against `5 mL`; deliberately **not** `PartialOrd` | Matcher: a partial birth date is compared at its own precision; a missing field blocks auto-link | **Same stance.** Their reason for avoiding `PartialOrd` (equality is record identity) applies to any clinical value type we add |
| Validation | Two gates: constructors for data built, `validate()` for data received, because `serde` never calls a constructor | `patients_store::validate`, `consent::validate`, `relationships::validate` on every write; an unknown role, purpose, resource or label is a 422, never dropped | **Same.** Worth stating in `AGENTS.md` |
| PHI in output | No `Display` renders an identifier; no error echoes a submitted value | Audit `detail` carries ids and counts only (tested); errors from the store are logged, not returned | **Same intent**; theirs is enforced by type, ours by convention and review |
| Queries | AQL lexed, parsed, statically checked; **no execution** | None | Neither executes. FHIR search is ours to build |
| Terminology | openEHR's own groups; external codes carried opaquely | SNOMED identifiers checked for shape (`interop::terminology`); no lookup | Equal. Both wait on a terminology server (plan OQ-7) |
| Units | No UCUM conversion, on purpose ("a wrong conversion is a thousand-fold dosing error") | None | Agree; keep it that way until a UCUM implementation is verified |
| Cost | 39,227 lines, 7 direct dependencies | `fhir` R4 alone adds ~135,000 generated lines to a build | `openehr` is the lighter dependency |

## What to take (in order of value)

1. **A published checkpoint of the audit head.** `GET /api/audit/verify` should also return the head
   digest and row count, and an operator should copy them somewhere the database owner cannot
   write. That is the only answer to "someone deleted the newest rows". Small; no new dependency.
2. **Report an unheld key as a key problem, not as tampering.** Our `Chain::matches` returns
   false for both. A key rotation done wrong would look like an intrusion. Small.
3. **A contribution** for the clinical store (RC-1): one commit = one audit row = many versions, in
   one transaction. This also closes the "write and audit are two transactions" gap.
4. **Null flavours** in the clinical resource store, as four distinct codes, before the first
   allergy or observation is stored. Retrofitting is expensive.
5. **Mask with a count** for SE-4: replace, keep the document valid, report how much was hidden and
   never what.
6. **Deny and carry unknown consent or access schemes** (copy their `SchemeNotImplemented`
   behaviour) so a later consent format can be stored without ever being silently misread.

None of these needs the `openehr` crate.

## Things I noticed in the crate

- `examples/04_versioning_and_audit.rs` prints "revision history, most recent first" above a list
  that is oldest first. Its own spec (V8.7a) says oldest first, and records that openEHR
  contradicts itself on this. The label is wrong, the data is right.
- Its README is candid about what is not built (no whole-archetype parser; "16 of §15's 33
  requirements have no code"), which is the standard this repository holds itself to.

## What I did not do

- Run the crate's own test suite or benchmarks.
- Check any of its openEHR conformance claims against the openEHR specifications.
- Try mapping a FHIR `Patient` or this project's identity record into openEHR `DEMOGRAPHIC`
  classes. That mapping is the real cost of adopting it and is unestimated.
