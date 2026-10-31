# Retention schedule (DRAFT)

**The periods are not set.** They are the controller's to set from the NHS Records Management Code of Practice (or the local equivalent) and its own policy. Nothing here is advice, and **no retention period has been reviewed by anyone**. [`personal-data-inventory.csv`](personal-data-inventory.csv) gives every database column a retention class; `tests/personal_data_inventory.rs` fails when a column is missing from it.

| Class | Covers | Period | Today |
| --- | --- | --- | --- |
| `record` | Patient identity, consent, relationships, links, photos, allergies, observations, amendment requests, the hazard register (each with its `_current` view) | Not set | Kept for ever. The clinical record is append-only: there is no delete path and no migration may add one |
| `audit` | `audit_events`, `break_glass_grants`, `break_glass_reviews` | Not set | Kept for ever by default; `audit_events` is protected by database triggers and a hash chain, is partitioned by month, and can move whole old partitions to the `audit_archive` schema when `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` is set (off by default; moves, never deletes; H-042, OQ-OS7). `audit_archive_anchors` records each move |
| `feed` | `feed_refusals` and `feed_refusal_resolutions`: HL7 messages the service refused, with the raw message | Not set | Kept for ever. The raw message holds identity and may hold more, so it is the strongest case for a shorter period |

## What is not decided

- **Erasure against append-only.** A period that ends in deletion conflicts with the rule that clinical data is never deleted. Which of retention, archive, redaction or a legal exemption applies is for the controller, the Clinical Safety Officer and a privacy lead. No deletion job exists and none should be written before that is decided.
- **Sessions.** The UI keeps no personal data in the database; its server-side sessions are in the UI's memory. Their lifetime is a UI setting, not part of this schedule.
- **Backups.** Not covered: no backup procedure exists yet.
