# Reads as at a time (EH-2)

**Built 2026-11-24, service only**, for allergies (with their statement and banner), the problem list and relative history. No screen. NOT FOR CLINICAL USE. Hazard H-068. The idea's source is openEHR's `version_at_time` ([lessons](../lessons/openehr-rust/index.md)).

## What it answers

*What did the record say at this time?* For example, what the allergy list showed when a medicine was given. The append-only versions already held the answer and no route could ask for it. It is **not** the record as it stands, and not today's record with the later lines left out.

## Rules

1. `GET …/as-at?time=` with an RFC 3339 time and an offset or `Z`. Missing, unreadable or in the future is a 400. There is no default: now would pass for an answer about the past.
2. Each entry is the latest version written at or before the time. One first written later is not there. One marked entered in error later **is** there, as it was, with `changed_since: true`.
3. **Merges are those that stood then** (`links::linked_ids_at`), so a merge made later is not followed and one reversed since is.
4. **Access is decided as it stands today**: role, relationship, purpose, consent. Someone who may not see the list now may not see it as at a time.
5. The answer carries `as_at` and `read_at`, and the audit row holds the time asked for.
6. **It says what it is** (AG-7): `basis` is `reconstruction-of-stored-versions`, and `left_out` lists what could have changed what was seen: amendment notices, the access rules and the identity at the time, what a screen showed, and whether a write in progress had finished. It never says what a clinician saw or knew.

## Limits

- **It is what was stored, not what was on a screen.** What a reader saw depends on the screen, a cache and the second (EH-10).
- The time of a problem or relative history version is the clock when the row was written, and it can be seen only after its transaction commits, a moment later, so an answer for that moment has a version nobody could read (`tests/entries_store.rs` holds this). An allergy version's time is when its transaction **began** (`now()`), so a write in progress at the time asked may be included.
- Consent, relationships and the patient's identity are not given as at the time. A patient who exists now is enough; whether they were registered then is not asked.
- Amendment notices are not part of the answer.
- Observations, results, referrals, adjustments and the rest have no such read yet (EH-15).
