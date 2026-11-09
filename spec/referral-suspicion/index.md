# Suspicion along a referral

**Requirement (2026-11-09, the maintainer).** A Patient Administration System (PAS) must track **suspicion** during a
referral: what condition the referrer suspects, from when, on what grounds, and what became of it. The EPR records the
clinical suspicion; the PAS carries it on the referral and the waiting list and runs any clock it starts. A design and a
requirement on the PAS boundary ([scope](../02-scope.md)). Built: the pathway catalogue below (RS-6); the rest is tasks
RS-1 to RS-5.

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

For one suspicion, the plan is the pathway's (or band's) priority raised by every factor that applies, and each
target's **due day**: the clock start's UTC day plus the target's days. A target is **overdue** once that day has
passed, so a same-day target is overdue from the next day. These are the record's view of the dates, for the screen and
the referrer's list; the PAS owns the reported clock, its pauses and its stops (PA-15).

The built-in pathways are the seven conditions on this page, with the figures from the table above; they are examples
for development, and a deployment replaces the file with its own approved catalogue.

**Not done (RS-6):** the plan is not yet attached to a stored suspicion (RS-1) or shown anywhere; outcomes do not stop a
target (RS-3); due days are UTC days, not the deployment's local days; clock pauses (a patient's choice to wait) are not
modelled; no clinician has approved any pathway.

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

Only the pathway catalogue (RS-6) is built. The NICE intervals were read through search summaries, not the NICE pages; each guideline has been
amended since publication. The Welsh target's current figure was not checked against the latest Welsh Government
statistics. The PAS in `~/git/patient-administration-system` was not read for whether it tracks suspicion. No clinician
has read this page.

## Sources

Read 2026-11-09 through web search results; pages not opened unless said.

- [GOVWNEW] Welsh Government, *New measure for cancer services: Wales a UK first*: https://www.gov.wales/new-measure-cancer-services-wales-uk-first
- [WCWT] Welsh Government, *NHS cancer waiting times quality report* (2019): https://www.gov.wales/sites/default/files/statistics-and-research/2019-10/nhs-cancer-waiting-times-quality-report.pdf ; *Suspected cancer pathway quality report* (2023): https://www.gov.wales/sites/default/files/pdf-versions/2023/8/5/1691749814/suspected-cancer-pathway-quality-report.pdf
- [WNOP] Welsh Government, *National optimal pathways for cancer* (2019): https://www.gov.wales/sites/default/files/publications/2019-10/national-optimal-pathways-for-cancer.pdf
- [IWA] Institute of Welsh Affairs: https://iwa.wales/?p=49889 (press)
- [NG12] NICE NG12, *Suspected cancer: recognition and referral*: https://www.nice.org.uk/guidance/ng12 ; NICE feature: https://www.nice.org.uk/news/feature/helping-gps-make-an-early-diagnosis-of-cancer
- [NG18] NICE NG18, *Diabetes (type 1 and type 2) in children and young people*, recommendation 1.1.2: https://www.nice.org.uk/guidance/ng18/chapter/recommendations ; QS125 statement 1
- [NG106] NICE NG106, *Chronic heart failure in adults*: https://www.nice.org.uk/guidance/ng106/chapter/recommendations ; QS9 statement 2: https://www.nice.org.uk/guidance/qs9/chapter/Quality-statement-2-Specialist-assessment
- [NG71] NICE NG71, *Parkinson's disease in adults*, recommendation 1.2.2: https://www.nice.org.uk/guidance/ng71/chapter/recommendations
- [NG97] NICE NG97, *Dementia*: https://www.nice.org.uk/guidance/ng97/chapter/recommendations ; QS184 statement 2: https://www.nice.org.uk/guidance/qs184/chapter/quality-statement-2-diagnosis
- [NG202] NICE NG202, *Obstructive sleep apnoea/hypopnoea syndrome and obesity hypoventilation syndrome in over 16s*: https://www.nice.org.uk/guidance/ng202/chapter/rationale-and-impact ; supporting documentation 3: https://www.nice.org.uk/guidance/ng202/documents/supporting-documentation-3
