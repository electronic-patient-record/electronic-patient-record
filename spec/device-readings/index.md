# Device readings (EM-11 design, first part)

**Post-1.0, nothing built.** Bedside devices (monitors, pumps, ventilators) send readings through a gateway, as HL7 v2
ORU or FHIR. This page holds the design EM-11 will be built from. Its first rule comes from the Welsh Intensive Care
Information System, which charts device-captured vital signs and calculated scores ([NHS Wales](../nhs-wales/index.md),
WL-10). Whether any of it is a medical device under the rules where it is used is OQ-6, and it is not decided here.

## A device reading counts once a clinician validates it (WL-10)

A reading from a device is held as the device sent it, with its **source kept**: the device (its identifier, type and
model, and the gateway it came through), the **capture time** the device stamped, and the time the service received it.
The capture time is the reading's time, never the receipt time, so a reading that arrives late or in a batch stays where
it belongs on the chart. Until a clinician **validates** it, a reading is shown as device-captured and unvalidated,
apart from validated values, and it **does not count toward any score** (NEWS2 or another early-warning score, a fluid
balance, a calculated index) or trigger an alert that a validated value would. Validation names the clinician and the
time, and may cover a set of readings over a stated period (an hour of a chart) rather than one at a time. A clinician
may instead **reject** a reading as an artefact (a probe off, a line flushed, the patient moving), with a reason; the
rejected reading stays in the record, never deleted, and never counts. A clinician who disagrees with a value records
their own observation, which names the device reading it replaces; the device's value is never edited. This is the
RC-11 rule (readings tagged by source stay out of flowsheets and scores until accepted) applied to bedside devices.

**What the record lacks for it today.** An `Observation` (`src/observations.rs`) holds a code, a value or an absent
reason, a unit and ranges. It has no source, no device, no capture time apart from the version's recorded time, and no
validated or rejected state. EM-11 and RC-11 add them; no score is computed anywhere yet, so nothing counts an
unvalidated reading today.

## Not decided

- How long an unvalidated reading may wait before it is flagged as unvalidated on a handover (a unit's policy).
- Whether a validated set can be unvalidated, and who may do it.
- Charting intervals, device protocols and which gateway (the WICIS sources found none, see the NHS Wales page).
- Whether this is a regulated medical device function (OQ-6).

No clinician, intensive care nurse or device engineer has read this page.
