# Masking (EH-8, the design for SE-4)

**The rules are built as two pure libraries 2026-11-25 (`masking.rs`, `labelling.rs`); nothing calls them, no record carries a label, and no route masks.** No clinician, Clinical Safety Officer, data protection officer or person with a protected record has reviewed this. NOT FOR CLINICAL USE. The idea's source is [openEHR and `openehr-rust`](../lessons/openehr-rust/index.md) (replace, keep the answer valid, count, never say what, fail closed); the rest is the author's reasoning. Companion: [Protected information](protected-information.md), which lists the routes by which a record leaks.

## Rules

1. **A whole entry is withheld, never a part.** A title without its code, or a code without its title, still tells. An entry with two labels is withheld when the viewer may not see one of them.
2. **The answer counts for the care team, and never says what** (OQ-PI5). A patient, a proxy and anyone without a treating relationship get `unstated`, with nothing withheld as with something, so that a person being watched gives nothing away; it is not "whole" and not "none". For one viewer the answer is the same whatever the withheld entries hold; only their number can differ. A test compares answers for every role and label, with the hidden entry's title, certainty, status, dates, cause and id changed.
3. **It fails closed.** A code this version does not know withholds the entry from everyone, since it may be the strictest of all. A refusal withholds. **Break-glass shows everything**, an entry with such a code included (OQ-PI2), to a treating role reading clinical content, with the access flagged for review; to a colleague with a relationship as to someone without, so that the team does not see less than a stranger.
4. **The answer stays valid.** A list with something withheld is `partial` with a count, so an empty rest is never "none" or `not-recorded`. Whatever is worked out from a list (a summary, a group by cause, a banner) is worked out from what is shown, and says partial.
5. **A shown entry does not point at a withheld one.** A problem does not say that a withheld problem causes it, and does not say that it has a cause: "caused by something restricted" is part of what is restricted.
6. **A write does not undo what the writer could not see.** Every change sends a whole record, so someone who was shown no cause would remove it. A withheld cause stays unless the writer names one they can see, which replaces it; the earlier version stays in the history.
7. **The decision is the policy engine's** (`evaluate`), asked once for each entry with that entry's labels (with every label, for one whose codes cannot be read). Masking adds no rule on who sees what.

## Who labels (OQ-PI7, `labelling.rs`)

| Act | Who |
|---|---|
| Add a label | A clinician with a treating relationship, on their own judgement or because the patient asked. No words beside it, since words can say what it hides |
| Lift one | Another clinician with a treating relationship, with a reason of 1 to 500 characters. Never the one who added it. Someone who may not is not told whether it is there |
| Lift the one for custody | As above, and only because the patient asked ([custodial health](../custodial-health/index.md)); a rule of the pack is not built |
| Add one with no person | Only the service, only for a stay in custody, only as `R`. Any other rule needs a reviewed list |

Nothing is removed: who added, who lifted, on whose decision and why stay, and a label may be added again. A code this version does not know is kept, so it withholds and is not lost. Two things here are the author's reading of the decision: that "a named person" who lifts is a treating clinician, and that custody is `R`.

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
| A review of the three decisions of 2026-11-25 (OQ-PI2, OQ-PI5, OQ-PI7) | Clinical Safety Officer, data protection officer, clinical advisers |
| The role lists for each label, which are the author's draft | Clinical Safety Officer, privacy officer |
| Hazard rows: a fact hidden when it was needed, a partial list read as whole, a label missing, a label lost in an output | with the first code that stores a label |

Then, in order: the grant looked up for a reader with a relationship too (`controllers/common.rs` looks only for one without, which is all that matters while nothing is labelled); a label column on the three entry tables, written only by a role that may; masking in `entries_store.rs` so that no handler can read a list without it; the places above; the census turned from gaps into tests; then results, observations, allergies and the rest, where a hidden allergy is the hardest case.

## Not done, not verified

No store, route, screen or FHIR. The test of rule 2 covers the problem list's summary and groups, and not an HTTP answer. Relative history, typing, observations, results and allergies have no rule for what points at a withheld entry. A viewer with several roles has the best of them, as in the policy engine. One gap in an emergency: a reader on the team who may see every known label without break-glass (the patient having permitted substance-use content) is still refused an entry whose code cannot be read, since the engine then has nothing to use the grant for.
