# Suspicion along a referral

**Requirement (2026-11-09, the maintainer).** A Patient Administration System (PAS) must track **suspicion** during a
referral: what condition the referrer suspects, from when, on what grounds, and what became of it. The EPR records the
clinical suspicion; the PAS carries it on the referral and the waiting list and runs any clock it starts. A design and a
requirement on the PAS boundary ([scope](../02-scope.md)). Built: the pathway catalogue below (RS-6) and the stored
suspicion (RS-1 to RS-4, below); RS-5 is a mapping to the PAS, agreed on 2026-11-10 and built on the PAS side; the EPR's sender is not built.

## Why suspicion, not diagnosis

A referral is usually sent before anything is known. What decides how fast the patient must be seen is what the referrer
suspects, and in some pathways the waiting time is counted from the moment of suspicion, not from the referral's
arrival or a diagnosis. If the suspicion is not held as data, the PAS cannot prioritise by it, a clock cannot start
from it, and when the referral is triaged, redirected or rebooked, the reason for urgency can be lost.

## Cancer, the worked example

- **Wales: the Single Cancer Pathway** counts from the **point of suspicion** to the start of first definitive treatment,
  not from referral or diagnosis [GOVWNEW] [WCWT]. The target was at least 75% of patients within 62 days from
  December 2020; the national optimal pathways expect diagnosis and staging within 28 days of the point of suspicion
  [WNOP]. From February 2021 Welsh Government publishes only the suspected cancer pathway figures [WCWT]. Reporting that
  no health board has met 75% since August 2020 [IWA] is press, not official statistics.
- **England: NICE NG12** organises suspected cancer referral by symptom, with a very urgent (48-hour) route, an urgent
  suspected cancer route and direct-access tests; the Faster Diagnosis Standard expects cancer to be diagnosed or ruled
  out within 28 days of an urgent referral [NG12] (the 28-day figure from secondary summaries; confirm on NHS England's
  page).

What this asks of the record, using cancer:

1. **The point of suspicion is a recorded event**: who suspected (a clinician, or a screening programme or test result),
   when, the suspected site (for example lower gastrointestinal), and the grounds (the NG12 criterion met, a test result,
   a clinical judgement). It is set once and corrected only by an amendment with a reason, because the clock starts from
   it.
2. **The suspicion travels with the referral.** Triage, redirection to another team or rebooking changes the referral,
   not the suspicion or its date. A redirected referral keeps its point of suspicion.
3. **It ends only with a recorded outcome**: confirmed (with the diagnosis), ruled out (with who and on what), or
   withdrawn by a clinician (with a reason). Never by a referral being closed, cancelled or lost. A clinic letter that
   discharges the patient without saying "ruled out" leaves the suspicion open, on a list.
4. **Absence is not a ruling-out.** An unanswered test or a missed appointment keeps the suspicion open, with the gap
   named; it never counts as "not cancer".
5. **What the patient has been told is recorded.** NG12 asks that the referral and its reason be discussed with the
   person [NG12]. A letter, a portal message or a text from the PAS does not name the suspected condition unless the
   record says the patient was told; otherwise it says "an urgent appointment".

## Other conditions, the same shape

The same record serves any condition where a suspicion sets the urgency or the route. The intervals below are the
guideline's, checked on 2026-11-09 through search summaries of the NICE pages; each pathway's own rules decide.

| Suspected | What sets the urgency | Interval or route | Source |
|---|---|---|---|
| Type 1 diabetes in a child or young person | The suspicion itself | Same day, to a multidisciplinary paediatric diabetes team; suspected DKA straight to hospital | [NG18] |
| Heart failure | NT-proBNP | Above 2,000 ng/L: specialist assessment and echocardiography within 2 weeks; 400 to 2,000 ng/L: within 6 weeks | [NG106] |
| Parkinson's disease | The suspicion itself | Refer quickly and untreated to a specialist in its differential diagnosis | [NG71] |
| Dementia, including Alzheimer's disease | Suspicion that remains after reversible causes are looked for | To a specialist diagnostic service (memory service), if the person agrees | [NG97] |
| Obstructive sleep apnoea | Two or more features, then a sleep history | Sleep service; faster for vocational drivers, vigilance-critical jobs, unstable cardiovascular disease, pregnancy | [NG202] |

What the examples add to the cancer rules:

- **The grounds can be a measurement** (NT-proBNP), so the suspicion links to the result it came from, and the interval
  follows the value.
- **The suspicion can carry an instruction** (Parkinson's: untreated), which the receiving team needs to see.
- **The person's agreement can be part of it** (dementia): "suspected, the person declined referral" is a state, not an
  absence.
- **Priority can come from the person's circumstances** (sleep apnoea and driving), which the PAS needs for its list but
  which is personal and may be sensitive.
- **A same-day route is not a waiting list** (diabetes in a child): the PAS shows an open same-day suspicion as overdue
  by the end of that day.

## Pathways as configuration (RS-6)

**Requirement (2026-11-09, the maintainer).** Generalise what cancer needs (the suspicion, the target dates, the
appointment priority) to configurable conditions, such as sleep apnoea, diabetes and dementia.

Cancer is one **pathway** among several. A pathway version is data, `src/suspicion_pathways.toml` in the service, read
into the build (`src/pathways.rs`). A stored suspicion will name the pathway and version it was made under (RS-1), so a
new version never changes the dates of an old suspicion. Each pathway version has:

| Field | Meaning | Example |
|---|---|---|
| `id`, `version`, `title` | Versions run 1, 2, 3 and none is removed | `suspected-cancer-wales` v1 |
| `sources` | Keys in this page's source list | `WNOP`, `GOVWNEW` |
| `approved_by`, `approved_on` | Both, or neither; neither is a **draft**, shown as one | empty: every built-in pathway is a draft |
| `clock_from` | `point-of-suspicion` or `referral-sent` | Wales counts from suspicion; England's 28 days from referral |
| `priority` and `target`s | A referral priority (`routine`, `urgent`, `asap`, `stat`) and targets in days from the clock start (0 is the same day, at most 365) | diagnosis 28 days, first treatment 62 |
| `band`s, instead | The writer must choose one; each has its own priority and targets, and may need the grounds to be a result | heart failure: NT-proBNP above 2,000 ng/L, 14 days; 400 to 2,000, 42 days |
| `factor`s | Circumstances that raise the priority to at least their own | sleep apnoea: drives for a living, urgent |
| `needs_agreement` | The person's agreement must be recorded, yes or no; "declined" is a state | dementia |
| `instruction` | What the receiving team must see | Parkinson's: refer untreated |
| `no_target_reason` | Given exactly when there is no target anywhere, so a missing date is never read as "no hurry" | Parkinson's: "refer quickly", no interval |
| `subtypes` | A list a suspicion may name a subtype from, or with `required` must: a `scheme`, a title, a source and named codes. Without one, no subtype is allowed | Wales cancer v2: the primary cancer site, required; dementia v2: the type, optional |

For one suspicion, the plan is the pathway's (or band's) priority raised by every factor that applies, and each
target's **due day**: the clock start's day in the site's time zone (`EPR_SITE_TIME_ZONE`, UTC when unset; the plan names it as `time_zone`) plus the target's days. A target is **overdue** once that day has
passed, so a same-day target is overdue from the next day. These are the record's view of the dates, for the screen and
the referrer's list; the PAS owns the reported clock, its pauses and its stops (PA-15).

The built-in pathways are the seven conditions on this page, with the figures from the table above; they are examples
for development, and a deployment replaces the file with its own approved catalogue.

## The stored suspicion (RS-1)

`/api/patients/{id}/referrals/{referral_id}/suspicion`: `GET` (every version oldest first, and `current`; none is an
empty history), `POST {suspicion}` (version 1; 409 if the referral has one) and `PUT {suspicion, expected_version,
reason}`. A suspicion holds the condition in the referrer's words and an optional code (not looked up), the point of
suspicion (RFC 3339, not in the future), an optional `subtype` `{scheme, code}` from the pathway version's list (RS-5), `source` (`clinician`, `screening-programme`, `result`) and `by`, the grounds
(`criterion`, `result` with a `result_id` on this patient's record, or `judgement`, with words), an optional
instruction, `patient_told` (`told`, `not-told`), and the pathway, version, band, factors and agreement the pathway
asks for. Version 1 is as raised; every later version is an amendment with a reason, and one that changes nothing is
refused. Each version lists the fields it `changed` and carries its own plan, so moving the point of suspicion moves
the due days and the earlier version keeps its own.

It is authorised as `Condition` (a provisional diagnosis): role, care relationship, purpose and consent. A read with
the `patient-access` purpose is refused until RS-4 decides what a patient sees. The audit row holds ids, versions and
the names of the fields changed; never the pathway, whose id names the condition, nor the condition, grounds or
reason, because the trail feeds the patient's access log.

**It survives triage, rebooking and redirection (RS-2).** Accepting, booking, a missed appointment and rebooking are
versions of the same referral, so they never touch its suspicion. A redirect ends a referral; the referral that
replaces it names it in `replaces` (a redirected referral of the same patient, replaced once only, else 422), and in
the same transaction its current suspicion becomes the replacement's version 1: the same body, the point of suspicion
unchanged, `carried_from` naming the old referral. A pathway whose clock runs from `referral-sent` counts from the
first referral in the chain, not the replacement.

**It ends only by an outcome (RS-3).** `POST …/suspicion/outcome {outcome, expected_version}` records `confirmed`
(with the diagnosis, and a code if given), `ruled-out` (with who ruled it out, and on what) or `withdrawn` (with the
reason), against the current version, once. After it the suspicion cannot be amended, and each plan's `overdue` is
judged at the outcome rather than now, so a target reads as met or missed. A missed appointment, a decline, a
cancellation or a redirect does not end it. A referral that has ended (declined, completed, cancelled, redirected)
with its suspicion open and not carried to a replacement is on the referrer's worklist as `open-suspicion`, which
cannot be acknowledged and leaves only when the outcome is recorded. The audit row names the outcome's kind, never
the diagnosis or its grounds.

**Messages do not name an untold suspicion (RS-4).** Everything the service sends goes through the outbox
(`src/outbox.rs`), whose gate decides each event before any sink sees it. An event whose payload names a suspected
condition carries `names_suspicion_of` (the referral). To a patient or a proxy it is refused (`suspicion-not-told`)
unless that referral's current suspicion says `told`, and for a referral that is not that patient's. The record is
read when the event is sent, not when it was queued. The sender's other choice is a message that does not name the
condition ("an urgent appointment"). This rests on the sender setting the field; nothing reads a payload to find a
condition named without it. No letter, portal or text sender exists yet, and PA-13's opt-outs and quiet hours are
not built.

**Not done (RS-6):** no screen shows the plan; outcomes do not stop a
target (RS-3); clock pauses (a patient's choice to wait) are not
modelled; no clinician has approved any pathway.

## The PAS side (RS-5): a proposed mapping

The PAS in `~/git/patient-administration-system` already tracks suspicion (its spec 20, read 2026-11-09):
`POST /api/referrals/{id}/suspicions` with `condition`, `subtype`, `unique_pathway_identifier`, `suspected_on` (a date),
`suspected_by` and `recorded_by`; `POST /api/suspicions/{id}/stop` once, for `treatment_started`, `downgraded`, `died`
or `entered_in_error`; conditions, subtypes, targets, a held cancel and a booking priority as a profile (`generic`,
`wales`). Its clock always starts at the point of suspicion and never pauses. This is a proposal, sent to the PAS's
maintainers and not agreed; nothing is built.

| EPR | PAS | Gap |
|---|---|---|
| `pathway` `suspected-cancer-wales`, `suspected-cancer-england` | `condition` `cancer` | — |
| `type-1-diabetes-child` | `diabetes`, `subtype` type 1 | — |
| `heart-failure` | `cardiovascular_disease` | The PAS condition is broader |
| `parkinsons`, `dementia`, `obstructive-sleep-apnoea` | `parkinsons_disease`, `dementia`, `obstructive_sleep_apnoea` | — |
| `subtype` `{scheme, code}` (2026-11-10) | `subtype` and the proposed `subtype_scheme` | Built on the EPR side: `suspected-cancer-wales` v2 requires `wales-primary-cancer-site`, `dementia` v2 takes `generic-dementia-type`, both copied from the PAS's profiles; `type-1-diabetes-child` sends `generic-diabetes-type` `type_1` by mapping, not stored |
| `point_of_suspicion` (a time) | `suspected_on` (a date) | Settled: the day in the PAS profile's `time_zone` (the EPR's own due days are the site's days) |
| `by` | `suspected_by` | — |
| the version's author | `recorded_by` | — |
| the referral's `external_ids` | `unique_pathway_identifier` | Which external id system carries it is unnamed |
| the EPR referral id | the PAS referral id | **Nothing links the two referrals yet** |
| an amendment moving the point of suspicion, with a reason | none: a PAS suspicion is stopped `entered_in_error` and recorded again | Whether the PAS should correct in place |
| outcome `ruled-out` | stop `downgraded` | — |
| outcome `confirmed` (at diagnosis) | stop `treatment_started` (later) | **Different events**: a confirmed suspicion keeps the PAS clock running until treatment starts |
| outcome `withdrawn` | none (`entered_in_error` is for mistakes) | **No PAS equivalent** |
| none (BV-1 holds death assertions) | stop `died` | The EPR does not end a suspicion on death |
| England's `clock_from` `referral-sent` | the clock is fixed at the point of suspicion | **A different measure**: the PAS spec says such a standard must say so |
| Wales targets 28 days (diagnosis) and 62 | 62 only | The 28-day target is the EPR's alone |

**The PAS's answers (2026-11-09, its session's recommendations; agreed by the PAS's maintainer on 2026-11-10, below).**

1. **Referral identity:** the EPR referral id travels as the PAS referral's `external_ref`. A PAS referral is unique
   on (`source`, `external_ref`) and receiving is idempotent, so a retried send is safe. The PAS would add an `epr`
   source, because `internal` would mix EPR referrals with ones raised in the PAS. `unique_pathway_identifier` is not
   the join (optional, Welsh cancer data sets only, not unique).
2. **Subtype:** agreed as an optional `{scheme, code}` on the EPR suspicion, checked against the PAS's
   `GET /api/suspicion-profile`. The PAS would add an optional `subtype_scheme` to its request, refused unless it is
   the active profile's list for that condition, so a code from another list is never relabelled.
3. **Confirmed:** agreed. The EPR sends nothing on `confirmed`, and the PAS clock runs on to first definitive
   treatment, a downgrade or death.
4. **Withdrawn:** not `downgraded` (it would skew the ruled-out figures) and not `entered_in_error` (the suspicion was
   real). The PAS would add a `withdrawn` stop; whether it counts against the 62-day share is for the PAS's cancer
   services sign-off (its WPAS-1a).
5. **Corrections:** for now an amendment of the point of suspicion stops the PAS suspicion `entered_in_error` and
   records it again, each step audited. An amend route (If-Match, a reason, open suspicions only) follows if
   amendments prove common. England's referral-sent clock stays an EPR-side displayed target and is never sent as a
   PAS clock.

Also from the PAS: it takes neither the grounds nor whether the patient was told, which are clinical. `suspected_by`
and `recorded_by` must be named people (the clinician who raised it, the EPR user who recorded it), never a system.
`suspected_on` is a date in the PAS's own time zone (`PAS_TIMEZONE`). Its writes need `If-Match`, so the EPR reads
before it stops. The four PAS changes (an `epr` source, `subtype_scheme`, a `withdrawn` stop, perhaps an amend route)
were agreed on 2026-11-10.

**Agreed and built on the PAS side (2026-11-10, its WPAS-9).** The PAS's maintainer agreed all four changes, and its
commit is `b64b7e2` on the PAS's `origin/main` ("Link suspicions from an electronic patient record"), seen here in
its git log; its code and tests were not read from here. The shapes, as the PAS's session sent them (its `spec/openapi.json` holds the schemas `Suspicion`,
`NewSuspicion`, `Amendment` and `ActiveProfile`):

| Call | Body and rules |
|---|---|
| `POST /api/referrals` | `source` `epr`, and `external_ref` (the EPR referral id) is required for it; the same pair again returns the first referral with `duplicate: true` |
| `POST /api/referrals/{id}/suspicions` | `condition`, `subtype?`, `subtype_scheme?`, `unique_pathway_identifier?`, `suspected_on` (a date), `suspected_by`, `recorded_by`; a `subtype_scheme` that is not the active profile's list for that condition is 400 |
| `GET /api/suspicion-profile` | `name`, `title`, `notes`, `conditions[]`, and now `time_zone` (IANA, for example `Europe/London`): the zone `suspected_on` is a day in |
| `POST /api/suspicions/{id}/stop` | `reason` (`treatment_started`, `downgraded`, `died`, `withdrawn`, `entered_in_error`), `stopped_on`, `stopped_by`, with `If-Match`; `withdrawn` is counted in the report and kept out of the share within target and the median; a second stop is 409 |
| `POST /api/suspicions/{id}/amend` | `suspected_on?`, `suspected_by?`, `subtype?`, `subtype_scheme?`, `reason`, `amended_by`, with `If-Match`; open suspicions only, at least one field changed, never the condition (stop and record again); emits `SuspicionAmended` |
| Errors | RFC 9457 problems; no `If-Match` is 428, a stale one 412 |

So an EPR amendment of the point of suspicion or the subtype becomes a PAS amend, not a stop and re-record; an EPR
`withdrawn` becomes the PAS's `withdrawn` stop; `ruled-out` becomes `downgraded`; `confirmed` sends nothing. The EPR's
`suspected_on` is the point of suspicion's day in the profile's `time_zone`, read from the PAS, not a UTC day.

**The subtype is checked against the PAS's live profile when sent.** The EPR's lists are copies, copies drift, and
an operator may run another profile or their own file. So the sender reads `GET /api/suspicion-profile`, checks the
code against that condition's `subtypes`, and sends `subtype_scheme` with it; a mismatch is held for a person, not
sent. The PAS's 400 is the backstop only. Of the built-in profiles, both carry the diabetes and dementia types, and
only `wales` has the cancer site list: `generic` refuses any cancer subtype.

**The mapping, built (2026-11-10).** `src/pas_suspicion.rs` in the service is the mapping as pure code: the PAS
condition for each pathway (and the subtype a child's type 1 diabetes implies), the `NewSuspicion`, `Amendment` and
stop bodies, each checked against an `ActiveProfile` as the PAS answers it, or a hold with words naming ids and codes.
A suspicion raised by a screening programme or a result is held, because the PAS needs the clinician's name. An
amendment that changes nothing the PAS holds sends nothing; one that removes a subtype, or changes the PAS condition,
is held for a person. A held programme or result suspicion goes to the PAS only once a clinician takes it on:
that clinician is `suspected_by`, and the day they acted is the point of suspicion sent. The PAS's named-person check
refuses only a blank name, so the EPR never puts a programme, system or laboratory name there. Whether the PAS should
instead count from the earlier day the result was reported is for the PAS's cancer services sign-off (its WPAS-1a), not
sent now; that it was raised by screening stays in the EPR (PAS session, 2026-11-10). `tests/pas_contract.rs` checks every body against the PAS's request schemas, copied with the
two built-in profiles from PAS commit `b64b7e2` into `tests/fixtures/pas/`. **Not built:** the sender (an outbox
sink, the profile read, the `epr` referral with `external_ref`, `If-Match` from a read, the PAS ids kept), and the
names: `recorded_by`, `amended_by` and `stopped_by` are given by the caller, and the service holds token subjects,
not names.

## Who owns what

| Part | EPR | PAS |
|---|---|---|
| The suspicion: condition, point of suspicion, who, grounds, what the patient was told | records it; it is clinical | holds the id and the fields it needs to schedule |
| The clock and its target | shows it, read through the port (PA-15) | starts it from the point of suspicion, pauses and stops it by the pathway's rules |
| Triage, redirection, rebooking | the clinical request (CA-3) | administrative status; never changes the suspicion |
| The outcome: confirmed, ruled out, withdrawn | records it | stops the clock from it; never ends a suspicion itself |
| Open suspicions with no outcome | lists them for the referrer | lists them on its waiting list |

## Rules for any implementation

- A suspicion is append-only like the rest of the record: a change is a new version with who, when and why.
- A referral closed while its suspicion is open goes on the referrer's list, as a referral timeout does now.
- The suspected condition is coded where a licensed code system allows (SNOMED CT's findings and "suspected" situations;
  not checked) and always kept as the referrer's words too.
- In FHIR, a suspicion maps most nearly to a `Condition` with `verificationStatus` `provisional`, linked from the
  `ServiceRequest`'s `reasonReference`; the point of suspicion has no standard element and needs an extension or the
  `Condition.recordedDate`. This mapping is unchecked.

## Not done, not verified

Only the pathway catalogue (RS-6) and the stored suspicion with its outcome and the outbound check (RS-1 to RS-4) are built; no screen shows either. The NICE intervals were read through search summaries, not the NICE pages; each guideline has been
amended since publication. The Welsh target's current figure was not checked against the latest Welsh Government
statistics. The PAS in `~/git/patient-administration-system` was read (its spec 20 and its two profiles), not run; the subtype lists here are copies of its profiles, and nothing checks that the two stay the same. No clinician
has read this page.

## Sources

Read 2026-11-09 through web search results; pages not opened unless said.

- [GOVWNEW] Welsh Government, *New measure for cancer services: Wales a UK first*: https://www.gov.wales/new-measure-cancer-services-wales-uk-first
- [WCWT] Welsh Government, *NHS cancer waiting times quality report* (2019): https://www.gov.wales/sites/default/files/statistics-and-research/2019-10/nhs-cancer-waiting-times-quality-report.pdf ; *Suspected cancer pathway quality report* (2023): https://www.gov.wales/sites/default/files/pdf-versions/2023/8/5/1691749814/suspected-cancer-pathway-quality-report.pdf
- [WDDPCS] NHS Wales Data Dictionary, *Primary Cancer Site Description* (valid from 2021-01-01, last changed by DSCN 2023/45), as copied in the PAS's `wales` profile; not opened here: https://www.datadictionary.wales.nhs.uk/worddocuments/primarycancersitedescription.htm
- [WNOP] Welsh Government, *National optimal pathways for cancer* (2019): https://www.gov.wales/sites/default/files/publications/2019-10/national-optimal-pathways-for-cancer.pdf
- [IWA] Institute of Welsh Affairs: https://iwa.wales/?p=49889 (press)
- [NG12] NICE NG12, *Suspected cancer: recognition and referral*: https://www.nice.org.uk/guidance/ng12 ; NICE feature: https://www.nice.org.uk/news/feature/helping-gps-make-an-early-diagnosis-of-cancer
- [NG18] NICE NG18, *Diabetes (type 1 and type 2) in children and young people*, recommendation 1.1.2: https://www.nice.org.uk/guidance/ng18/chapter/recommendations ; QS125 statement 1
- [NG106] NICE NG106, *Chronic heart failure in adults*: https://www.nice.org.uk/guidance/ng106/chapter/recommendations ; QS9 statement 2: https://www.nice.org.uk/guidance/qs9/chapter/Quality-statement-2-Specialist-assessment
- [NG71] NICE NG71, *Parkinson's disease in adults*, recommendation 1.2.2: https://www.nice.org.uk/guidance/ng71/chapter/recommendations
- [NG97] NICE NG97, *Dementia*: https://www.nice.org.uk/guidance/ng97/chapter/recommendations ; QS184 statement 2: https://www.nice.org.uk/guidance/qs184/chapter/quality-statement-2-diagnosis
- [NG202] NICE NG202, *Obstructive sleep apnoea/hypopnoea syndrome and obesity hypoventilation syndrome in over 16s*: https://www.nice.org.uk/guidance/ng202/chapter/rationale-and-impact ; supporting documentation 3: https://www.nice.org.uk/guidance/ng202/documents/supporting-documentation-3
