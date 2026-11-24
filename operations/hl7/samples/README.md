# HL7 v2 sample messages

Synthetic messages for the ADT feed (IH-11): no real person, address or number (the national number `9434765919` is a
published test value). One file per case, a segment a line; the sender and the tests turn line breaks into `\r`.

| File | What it shows |
|---|---|
| `a28-register.hl7` | A new patient with a local and a national number |
| `a08-update.hl7` | An update: a changed address and phone |
| `a01-admit.hl7`, `a04-outpatient.hl7`, `a31-update-person.hl7` | The other handled events, for the same patient |
| `a08-death.hl7` | A report of death (PID-29, PID-30) |
| `a28-training.hl7` | A training message (MSH-11 `T`), refused where `EPR_MLLP_PROCESSING_ID=P` |
| `oru-not-handled.hl7` | A type the feed does not handle: `AR` |
| `no-pid.hl7` | An ADT with no PID: `AE` |
| `bad-national-number.hl7` | A national number that fails its check: `AE` |

`{{control}}` and `{{mrn}}` are filled in by the sender (`scripts/hl7/send.mjs`): `{{control}}` with a new control id
per send, `{{mrn}}` with the value given, or a new one. `tests/hl7_samples.rs` reads every file here, with fixed values
for both, and compares the reader's acknowledgement byte for byte with `tests/golden/hl7-acks/<name>.ack`. A golden
file is rewritten only by `EPR_BLESS=1 cargo test --test main hl7_samples::`, and only when the change is meant: never to make
a failing test pass.

Every ADT sample is the same patient, so sent in order (register, update, admit, outpatient, update person, death) to one
feed they all apply. The golden files are the **reader's** answer, before the database: in the running feed a message
that is refused and kept for a person is answered `AR` (IH-1), so `no-pid` and `bad-national-number` get `AR` there,
not the reader's `AE`; `tests/mllp_feed.rs` sends the set through the feed and checks that.
