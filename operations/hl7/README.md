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
| `EPR_MLLP_SENDING_APPLICATIONS` | Comma-separated `APP` (MSH-3) or `APP^FACILITY` (MSH-3 and MSH-4), first components only: the only senders this feed takes (IH-6). Unset takes any | unset |
| `EPR_MLLP_VERSIONS` | Comma-separated HL7 versions (MSH-12), such as `2.5,2.5.1` | unset |
| `EPR_MLLP_EVENTS` | Comma-separated trigger events this feed takes, from those it handles | unset |
| `EPR_MLLP_PROCESSING_ID` | `P`, `T` or `D`: the processing id (MSH-11) this deployment takes; a message with another is refused `AR` and kept, so a test message never reaches a live record (IH-2). Unset takes any. A message outside any of these four lists is refused `AR` and kept, with a reason that quotes none of its values | unset |
| `EPR_MLLP_TLS_CERT_FILE`, `EPR_MLLP_TLS_KEY_FILE` | PEM certificate chain and private key: TLS on the port, and then nothing in plain text (IH-10). Both or neither | unset |
| `EPR_MLLP_TLS_CLIENT_CA_FILE` | PEM CA: every sender must present a certificate that chains to it. The only way the listener can know who is sending. Needs the two above | unset |
| `EPR_MLLP_TLS_CLIENT_NAMES` | Comma-separated DNS names or IP addresses: the sender's certificate must also be valid for one of them (its subject alternative names, not its common name), and the name it matched is recorded on every audit row as `peer_certificate` (IH-6). Needs the client CA | unset |

A malformed setting, or a port that cannot be opened, stops the service starting. Run it on loopback and put a TLS
terminator (stunnel, an integration engine's TLS listener) or a private network in front of anything remote. The peer's
address is recorded on every audit row.

## TLS

With `EPR_MLLP_TLS_CERT_FILE` and `EPR_MLLP_TLS_KEY_FILE` the port speaks only TLS (rustls with ring; TLS 1.2 and 1.3):
a client that does not finish a handshake within `EPR_MLLP_IDLE_SECONDS` is closed, and a plain-text client gets no
answer. With `EPR_MLLP_TLS_CLIENT_CA_FILE` too, a sender without a certificate from that CA is refused at the handshake.
With `EPR_MLLP_TLS_CLIENT_NAMES` as well, a certificate from that CA that is valid for none of the names is closed before
anything is read, so one CA can serve several senders and each listener takes only its own.
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
  MLLP, a frame that is not HL7 and text that is not UTF-8 close the connection, except a frame that starts as an `MSH`
  but does not read, which gets an `AE` echoing its MSH-10. Every acknowledgement copies MSH-11 (`P` when there is
  none), and `MSA-3` is one line of at most 199 characters with HL7's delimiters escaped (IH-2).

## Stopping

On a stop the listener stops accepting, closes idle connections, finishes the message each connection is handling (the
change committed and the answer sent) and waits up to 10 s; anything still unfinished is rolled back and gets no answer,
so the sender sends it again (IH-7). A repeat of a message that was applied changes nothing.

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

## Samples and a test sender

`samples/` holds synthetic messages, one per case ([what each shows](samples/README.md)). To send them to a test
listener and see each answer (IH-11):

```sh
node scripts/hl7/send.mjs --port 2575 operations/hl7/samples/a28-register.hl7 operations/hl7/samples/a08-update.hl7
node scripts/hl7/send.mjs --tls --ca ca.pem --cert client.pem --key client.key --count 10 operations/hl7/samples/*.hl7
```

To see what one message would do without keeping anything, a clinical administrator can post it to
`POST /api/feed/dry-run` (`{"message": "MSH|…"}`): it runs through the listening feed's rules, matching and checks in
a transaction that is rolled back, and answers what it would do (`created`, `updated`, `unchanged`, `refused`, `retry`
or `not-hl7`), why, and the acknowledgement the feed would send. Nothing is stored, queued or counted; the call itself
is audited.

Every send gets a new control id; `{{mrn}}` is one patient for the run (`--mrn` to choose it). It exits 0 only when
every answer is `AA`. Never point it at a live feed.

## Going live

How a feed moves here from the system that serves it today, with the register of differences that must be signed
first: [going live with an HL7 feed](going-live.md) (IH-12; never used).

## What is not built

`A40` merges, `PV1` and visit data, `ORU` results, `SIU`, a notifier for the feed's alarm, the health or count outside the admin pages, character sets other than UTF-8, and any conformance run against a real sending system.
