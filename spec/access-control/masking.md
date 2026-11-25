# Masking (EH-8, the design for SE-4)

**Built 2026-11-25 for three lists: problems, relative history and compatibility typing.** The rules are two pure libraries (`masking.rs`, `labelling.rs`); `entries_store.rs` applies them, so that no handler reads a list without a viewer; labels are in the append-only table `entry_labels`. **Nothing else can carry a label, and no screen shows or sets one.** No clinician, Clinical Safety Officer, data protection officer or person with a protected record has reviewed this. NOT FOR CLINICAL USE. The idea's source is [openEHR and `openehr-rust`](../lessons/openehr-rust/index.md) (replace, keep the answer valid, count, never say what, fail closed); the rest is the author's reasoning. Companion: [Protected information](protected-information.md), which lists the routes by which a record leaks.

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

## Where a list's other answers would tell

| Place | What is done |
|---|---|
| A refusal ("due_to names no problem on this list", "2 problem(s) name this as their cause") | The rules are judged on what the writer sees, so the words are the same as for an id that is not there and the count is of what they see. **So a retraction that only a hidden effect would stop is allowed**, and the effect is then in `cause_not_on_list` for whoever sees it. A loop through a hidden problem is not refused either; the walk ends at a loop |
| A version number, a 409, a 404 | A hidden entry answers 404, with a stale version too |
| `…/versions`, `…/labels` and `as-at` | By the entry's labels **now**, at every version and as at any time: a label added later hides the past, and a lift shows it |
| `summary` | `partial: true`, and `not-stated` for a state or an agreement of blood groups that a withheld entry could change. A disagreement that is seen is still said |
| The audit row | Counts of what was shown, and `withheld` when any were. A change of labels says that they changed and not which |
| Screens, FHIR, exports, the outbox, analytics | Nothing of these three lists reaches them. `tests/label_census.rs` fails when a label or the masking reaches another file |
| Time taken | Measured once 2026-11-25 in the store, a debug build on a busy laptop, the best of 15: 1,000 problems took 8 ms with none labelled and 15 ms with all; 100 took 1 ms and 2 ms. **So a long list that takes longer can tell that much is labelled.** Nobody who is not told the count reads these lists today. A hidden id and one that is not there do the same work |
| A retried write (`Idempotency-Key`) | Answers with what the writer was answered at first, if the entry has been labelled since. Not changed |

## What is needed before real use

| Needed | Who |
|---|---|
| A review of the three decisions of 2026-11-25 (OQ-PI2, OQ-PI5, OQ-PI7) | Clinical Safety Officer, data protection officer, clinical advisers |
| The role lists for each label, which are the author's draft | Clinical Safety Officer, privacy officer |

Then: a screen that says what is withheld and lets a clinician label (none shows these lists today); results, observations, allergies and the rest, where a hidden allergy is the hardest case; the label for a stay in custody, which needs a record of custody.

## Not done, not verified

No screen and no FHIR. Someone who adds a substance-use label cannot then see the entry without the patient's consent or break-glass, and cannot lift it. A patient's refusal of consent that names a label withholds in an emergency too (OQ-PI8). Someone reading in an emergency is told the count, as the care team is. Nobody is asked to review a label. Observations, results and allergies have no rule for what points at a withheld entry. A viewer with several roles has the best of them, as in the policy engine. One gap in an emergency: a reader on the team who may see every known label without break-glass (the patient having permitted substance-use content) is still refused an entry whose code cannot be read, since the engine then has nothing to use the grant for.
