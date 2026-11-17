# HL7 v2 ADT feed (MLLP)

The service can listen for **HL7 v2 ADT** messages (A01, A04, A08, A28, A31) over **MLLP** and apply them to the patient
identity record. It is **off by default**. **Development and evaluation only**: not for clinical use, and MLLP itself has
**no authentication and no encryption**.

## Turning it on

| Variable | Meaning | Default |
|---|---|---|
| `EPR_MLLP_BIND` | Address to listen on, e.g. `127.0.0.1:2575`. Unset = the listener is off | off |
| `EPR_MLLP_ALLOW_REMOTE` | `true` to allow a non-loopback address. Without it a non-loopback bind refuses to start | unset |
| `EPR_MLLP_AGENT` | The audit agent recorded for every message (never taken from the message) | `mllp:adt-feed` |
| `EPR_MLLP_MAX_CONNECTIONS` | Connections served at once; others are closed on accept | 8 |
| `EPR_MLLP_IDLE_SECONDS` | A silent connection is closed after this long | 60 |
| `EPR_MLLP_KEPT_ANSWER` | How a refusal kept in the work queue is answered: `AR` (kept for a person, do not resend) or `AE` for a sender that needs it. Anything else refuses to start (IH-1) | `AR` |

A malformed setting, or a port that cannot be opened, stops the service starting. Run it on loopback and put a TLS
terminator (stunnel, an integration engine's TLS listener) or a private network in front of anything remote. The peer's
address is recorded on every audit row.

## What a message does

- It is matched on the identifiers it carries: the sender's own number (`urn:hl7v2:<assigning authority>`), then a valid national
  health number. One match is an **update**, none is a **new patient**, more than one is refused.
- A new patient that the matcher thinks may already exist is **refused** and kept for a person (`AR`), not created. A person decides.
- An update **never blanks** a field the message lacks, and **never changes a birth date or an national health number**: those are
  corrected by a person with a reason. The first name is replaced by the message's first name; other names, telecom and
  addresses are added.
- A repeat of what is held writes nothing (`AA`, "unchanged"). Every message, accepted or refused, writes an audit row with
  the event, the control id and the result, never the message.
- Acknowledgements (IH-1): `AA` accepted. `AR` refused and **kept for a person**: `MSA-3` starts "kept for review by a
  person; do not resend", then the reasons. A message type that is not handled is `AR` too. `AE` only when a resend can
  help: the database unavailable, or the refusal could not be stored. `EPR_MLLP_KEPT_ANSWER=AE` answers kept refusals `AE`
  instead (with the same words), for a sender that must have it; a type not handled stays `AR`. A stream that is not
  MLLP, a frame that is not HL7 and text that is not UTF-8 close the connection.

## Refused messages

A refused message is kept as received in `feed_refusals` until a clinical administrator closes it
(`GET /api/feed/refusals`, `GET /api/feed/refusals/{id}`, `POST /api/feed/refusals/{id}/resolve` with `outcome`
`handled` or `dismissed` and a reason). The queue is for the person who fixes the cause (a corrected birth date, a
registration) through `/api/patients`; the sender does not need to resend. If it does, the same message from the same
feed with the same control id, while its entry is still open, adds no row and gets the same answer; once the entry is
closed, a resend is new work and is kept again. The stored text is patient data.
The screen is `/admin/feed` in the UI. The navigation shows how many are waiting on the admin pages only. There is no alert: someone has to look.

## What is not built

`A40` merges, `PV1` and visit data, `ORU` results, `SIU`, TLS, authentication, an alert or count for the refusal queue, character sets other than UTF-8, and any conformance run against a real sending system.
