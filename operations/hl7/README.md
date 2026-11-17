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
| `EPR_MLLP_TLS_CERT_FILE`, `EPR_MLLP_TLS_KEY_FILE` | PEM certificate chain and private key: TLS on the port, and then nothing in plain text (IH-10). Both or neither | unset |
| `EPR_MLLP_TLS_CLIENT_CA_FILE` | PEM CA: every sender must present a certificate that chains to it. The only way the listener can know who is sending. Needs the two above | unset |

A malformed setting, or a port that cannot be opened, stops the service starting. Run it on loopback and put a TLS
terminator (stunnel, an integration engine's TLS listener) or a private network in front of anything remote. The peer's
address is recorded on every audit row.

## TLS

With `EPR_MLLP_TLS_CERT_FILE` and `EPR_MLLP_TLS_KEY_FILE` the port speaks only TLS (rustls with ring; TLS 1.2 and 1.3):
a client that does not finish a handshake within `EPR_MLLP_IDLE_SECONDS` is closed, and a plain-text client gets no
answer. With `EPR_MLLP_TLS_CLIENT_CA_FILE` too, a sender without a certificate from that CA is refused at the handshake.
A missing, empty or unreadable file, a file with no certificate or key, or a key that does not fit the certificate stops
the service starting. A non-loopback bind still needs `EPR_MLLP_ALLOW_REMOTE=true`. Certificates are read once, at start:
a renewed certificate needs a restart.

## What a message does

- It is matched on the identifiers it carries: the sender's own number (`urn:hl7v2:<assigning authority>`), then a valid national
  health number. One match is an **update**, none is a **new patient**, more than one is refused.
- A new patient that the matcher thinks may already exist is **refused** and kept for a person (`AR`), not created. A person decides.
- An update **never blanks** a field the message lacks, and **never changes a birth date or an national health number**: those are
  corrected by a person with a reason. The first name is replaced by the message's first name; other names, telecom and
  addresses are added.
- A repeat of what is held writes nothing (`AA`, "unchanged"). Every message, accepted or refused, writes an audit row with
  the event, the control id and the result, never the message. The control id (MSH-10) is kept only as a keyed
  fingerprint of the feed and the id (IH-3), in the audit row, the work queue, the version history and the log, because
  some senders build it from a patient's number. The plain id is in the stored message and in the acknowledgement.
- A late update never undoes a newer one (IH-5): each applied message's time (EVN-2, or MSH-7) is kept per feed and
  patient, and an update older than the newest one applied is kept for a person (`AR`, "older than one this feed has
  already applied"). A time with no offset is read as UTC, so a sender must keep one convention. A message with no
  usable time is applied as before.
- No message value is written anywhere but the stored message and the patient record (IH-4): reasons and warnings name
  the field, never its value, and a frame that is not HL7 is logged only as its shape, with every value `***`.
- Acknowledgements (IH-1): `AA` accepted. `AR` refused and **kept for a person**: `MSA-3` starts "kept for review by a
  person; do not resend", then the reasons. A message type that is not handled is `AR` too. `AE` only when a resend can
  help: the database unavailable, or the refusal could not be stored. `EPR_MLLP_KEPT_ANSWER=AE` answers kept refusals `AE`
  instead (with the same words), for a sender that must have it; a type not handled stays `AR`. A stream that is not
  MLLP, a frame that is not HL7 and text that is not UTF-8 close the connection.

## Counting

With `EPR_METRICS=true`, `/metrics` has `epr_feed_messages_total{sender,event,answer}` (IH-9): `sender` is
`EPR_MLLP_AGENT`, `event` a handled trigger or `other`, `answer` `AA`, `AE`, `AR` or `none` (the frame was not HL7). Alert
on `answer="AE"` rising (the database or the queue is failing) and on any `answer="AR"` (a person has work). Nothing from
a message is a label. `epr_feed_refused_messages_total` stays as before.

## Health and alarm

While the listener is on, the feed is judged every minute (IH-8): `unknown` (it has sent nothing), `healthy`, `warning`
(refusals in the last hour, or any waiting) or `critical` (silent longer than the window allows, or many refusals in an
hour). Windows are the site's local day (weekdays 08:00 to 17:59), evening and weekend.

| Variable | Meaning | Default (a placeholder) |
|---|---|---|
| `EPR_FEED_SILENCE_MINUTES` | Longest silence per window, `day=N,evening=N,weekend=N` | `day=60,evening=240,weekend=720` |
| `EPR_FEED_REFUSALS_WARNING` | Refusals in an hour that make a warning | 1 |
| `EPR_FEED_REFUSALS_CRITICAL` | Refusals in an hour that make it critical | 10 |

A change of state is logged once ("HL7 feed alarm", an error when critical) and stored, so a restart does not log it
again. Alert on that line or on the gauge `epr_feed_health{sender}` (3 is critical). `GET /api/feed/health` shows the
state, the window, why, since when and the facts, to a clinical administrator. Nothing pages anyone by itself.

## Refused messages

A refused message is kept as received in `feed_refusals` until a clinical administrator closes it
(`GET /api/feed/refusals`, `GET /api/feed/refusals/{id}`, `POST /api/feed/refusals/{id}/resolve` with `outcome`
`handled` or `dismissed` and a reason). The queue is for the person who fixes the cause (a corrected birth date, a
registration) through `/api/patients`; the sender does not need to resend. If it does, the same message from the same
feed with the same control id, while its entry is still open, adds no row and gets the same answer; once the entry is
closed, a resend is new work and is kept again. The stored text is patient data.
The screen is `/admin/feed` in the UI. The navigation shows how many are waiting on the admin pages only. There is no alert: someone has to look.

## Going live

How a feed moves here from the system that serves it today, with the register of differences that must be signed
first: [going live with an HL7 feed](going-live.md) (IH-12; never used).

## What is not built

`A40` merges, `PV1` and visit data, `ORU` results, `SIU`, a notifier for the feed's alarm, the health or count outside the admin pages, character sets other than UTF-8, and any conformance run against a real sending system.
