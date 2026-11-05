# Death assertions

The rules for recording that a person has died (BV-1). A first draft for clinical and legal review: no clinician, medical examiner or bereaved person has reviewed it. The reasons and sources are on [bereavement](index.md).

**Built:** the rules and the read model, pure, in `electronic-patient-record-service-with-rust/src/death.rs`. **Not built:** storage, a route, a screen, or anything that a hold acts on. Until a death can be recorded, a report that arrives is refused rather than dropped: an HL7 message with PID-29 or PID-30 is answered `AE`, changes nothing and waits in the feed queue for a person, and a FHIR write with `deceased[x]` gets a 422. The service has no outbound contact yet, so nothing is held today.

## The model

A death is an assertion with evidence, not a true-or-false field on the identity record. One patient can have several assertions, from different sources, and each may be wrong.

| Status | Meaning | Holds contact | Confirmed |
|---|---|---|---|
| `reported` | Someone or something said so | Yes | No |
| `verified` | A person established that the patient has died | Yes | Yes |
| `certified` | The cause has been certified; the cause is a separate, later fact | Yes | Yes |
| `registered` | The death has been registered | Yes | Yes |
| `withdrawn-in-error` | A person found the assertion wrong | No | Final |

## Rules

1. **Any report holds contact at once**, whoever or whatever made it, including a feed ([OQ-BV1](../16-open-questions.md)).
2. **Only a person moves an assertion on.** A feed can report. It cannot confirm or withdraw, even when the source is a registrar.
3. **Steps go forward only**, and may be skipped. A person told by a registrar can go from `reported` to `registered`.
4. **Withdrawal is possible from any status and is final.** The assertion stays in the history and stops counting. A new report is a new assertion.
5. **Every report and step needs an author and a reason.** The reason is not optional on a withdrawal: a corrected record with no reason is the failure the US audit found ([bereavement](index.md#a-death-recorded-in-error)).
6. **A date may be `YYYY`, `YYYY-MM`, `YYYY-MM-DD` or an RFC 3339 instant, or absent with a reason.** It cannot be later than now; a partial date is compared at its start.

## The one answer

All of a patient's assertions, each at its current version, give one answer:

| Assertions that stand | Answer | Holds contact |
|---|---|---|
| None | `not-known-to-have-died` | No |
| One or more | `asserted`, with the furthest status, whether it is confirmed, the date from the furthest assertion that has one, and `dates_differ` if those that stand disagree | Yes |

`not-known-to-have-died` never means the person is known to be living, so no screen shows the word "alive". A withdrawal releases the hold only when no other assertion stands.

## Not settled

- **Who may take each step (OQ-BV2).** The rules name a person, not a role. The draft for review: any member of staff may report, a clinician or nurse may verify, and only a clinician may withdraw.
- **Does the matcher's `Patient` carry a date of death (OQ-BV3).** Recommendation: no. The assertion lives apart from identity, as this page describes, and a date of death is not used as match evidence. Not yet decided.
- **How soon a held contact is reviewed, and who owns that.** OQ-BV1 says a person confirms; it does not say by when.
- **Other fields on [the proposal](index.md#what-this-project-would-need).** `found_at`, `place`, `verified_at` and `coroner_referred` are not in this slice. They belong with the store.
