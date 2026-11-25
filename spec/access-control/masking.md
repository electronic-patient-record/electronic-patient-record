# Masking (EH-8, the design for SE-4)

**The rules are built as a pure library 2026-11-25 (`masking.rs`); nothing calls them, no record carries a label, and no route masks.** No clinician, Clinical Safety Officer, data protection officer or person with a protected record has reviewed this. NOT FOR CLINICAL USE. The idea's source is [openEHR and `openehr-rust`](../lessons/openehr-rust/index.md) (replace, keep the answer valid, count, never say what, fail closed); the rest is the author's reasoning. Companion: [Protected information](protected-information.md), which lists the routes by which a record leaks.

## Rules

1. **A whole entry is withheld, never a part.** A title without its code, or a code without its title, still tells. An entry with two labels is withheld when the viewer may not see one of them.
2. **The answer counts and never says what.** For one viewer the answer is the same whatever the withheld entries hold; only their number can differ. A test compares answers for every role and label, with the hidden entry's title, certainty, status, dates, cause and id changed.
3. **It fails closed.** A code this version does not know withholds the entry from everyone, since it may be the strictest of all. A refusal withholds.
4. **The answer stays valid.** A list with something withheld is `partial` with a count, so an empty rest is never "none" or `not-recorded`. Whatever is worked out from a list (a summary, a group by cause, a banner) is worked out from what is shown, and says partial.
5. **A shown entry does not point at a withheld one.** A problem does not say that a withheld problem causes it, and does not say that it has a cause: "caused by something restricted" is part of what is restricted.
6. **A write does not undo what the writer could not see.** Every change sends a whole record, so someone who was shown no cause would remove it. A withheld cause stays unless the writer names one they can see, which replaces it; the earlier version stays in the history.
7. **The decision is the policy engine's** (`evaluate`), asked once for each entry with that entry's labels. Masking adds no rule on who sees what.

## Where a list's other answers would tell (to be done with the wiring, each with a test)

| Place | What it would tell |
|---|---|
| A refusal: "due_to names no problem on this list", "2 problem(s) name this as their cause" | That an id exists, or how many hidden problems name this one. The refusal must be the same as for an id that is not there; the count must be of what the writer sees, and a retraction that a hidden effect stops needs another answer |
| A version number, a 409, a 404 | A hidden entry must answer as one that does not exist |
| `…/versions` and `as-at` | A label applies to the entry at every version, by its **newest** label: a label added later hides the past, and a lift shows it |
| The audit row and the patient's access log | Counts of entries read must be of what was shown, with the number withheld kept for the auditor |
| Screens, FHIR, exports, the outbox, the as-at reads, analytics | As the [leakage routes](protected-information.md); `tests/label_census.rs` lists each |
| Time taken | A list read that decides each entry takes longer with more of them. Not measured |

## What is needed before it is wired

| Needed | Who |
|---|---|
| What break-glass shows of a labelled record. Today nothing: hiding a medicine or an interaction in an emergency is a hazard as serious as showing it (OQ-PI2) | Clinical Safety Officer, clinical advisers |
| Whether a viewer is told the count at all; for a patient or a proxy the count itself can harm (OQ-PI5). The three lists this would start with refuse both | Clinical Safety Officer, data protection officer |
| Who may put a label on an entry, who may lift it, and which are on by default (OQ-PI7) | maintainer, Clinical Safety Officer |
| The role lists for each label, which are the author's draft | Clinical Safety Officer, privacy officer |
| Hazard rows: a fact hidden when it was needed, a partial list read as whole, a label missing, a label lost in an output | with the first code that stores a label |

Then, in order: a label column on the three entry tables, written only by a role that may; masking in `entries_store.rs` so that no handler can read a list without it; the places above; the census turned from gaps into tests; then results, observations, allergies and the rest, where a hidden allergy is the hardest case.

## Not done, not verified

No store, route, screen or FHIR. The test of rule 2 covers the problem list's summary and groups, and not an HTTP answer. Relative history, typing, observations, results and allergies have no rule for what points at a withheld entry. A viewer with several roles has the best of them, as in the policy engine.
