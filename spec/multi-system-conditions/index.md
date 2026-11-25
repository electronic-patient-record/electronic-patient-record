# One cause, several organs: multi-system genetic conditions (CF-4)

A draft from the customer's request of 2026-11-24 ("capabilities for complex genetic medical conditions that affect multiple organs and body systems, such as Brain-Lung-Thyroid syndrome"), search summaries of public sources and this repository's own files. Two building blocks exist in the service; the rest is a proposal. **It holds no clinical content on purpose**: what such a condition affects, and what to watch for, must come from a clinical geneticist and not from the author's reading. No clinician, geneticist or patient group has reviewed it. NOT FOR CLINICAL USE. "My reasoning" marks the author's.

## Related links

[Problem list](../problem-list/index.md) (causes), [Relative history](../relative-history/index.md), [Care settings](../care-settings.md) (specialty views, section 3; requests, section 4), [Pathway design](../pathway-design.md), [Support groups](../support-groups/index.md), [Protected information](../access-control/protected-information.md), [Open questions](../16-open-questions.md) (OQ-GC1 to OQ-GC6).

## What was asked, and what is not known

A customer wants the record to serve a person whose one condition is treated by several teams. The request names an example and no setting, country, age group or team. My reasoning on the need: each team sees its own organ; nobody sees that the findings have one cause; the person, or their parent, carries the story from clinic to clinic.

## What exists (service only, unreviewed)

| Piece | What it does | What it does not do |
|---|---|---|
| A problem names its cause (EH-1) | Findings in several organs point at one problem | Say how the cause was established |
| **Problems grouped by cause (2026-11-25)** | The list's summary groups every problem under the top cause it comes from, directly or through others, and says when a cause is not on the list (after an unmerge) or has been refuted | Work anything out: it is only what people recorded. A problem in no group has **no recorded link**, which is not the same as no link |
| Relative history (EH-3) | Findings in relatives, with the genetic link asked and never guessed, and no relative named | Use a relative's own record (OQ-EH4) |

A refuted cause is **said and not refused**: the finding in the organ may be real whatever its cause, so nothing is removed and the people who look after it are told that the reason recorded for it may not hold. No notice is sent; it is there when the list is read.

## What is missing (my reasoning)

1. **How the cause was established.** "Confirmed" on the problem list is a clinician's word. A genetic diagnosis may be clinical, by a test, or by a relative's test, and these differ when a test is read again. A problem could name the result that supports it (RC-12 and the result inbox); nothing does today.
2. **A test's meaning changes while the test does not.** A laboratory may later read the same variant differently. The earlier reading stays a version, the new one is raised again as a changed result is today (AG-1), and every problem that names it is listed as possibly affected. **No diagnosis changes by itself.**
3. **"No variant found" is not "no genetic condition"**, and "uncertain" is neither. The repository's rule applies: absent evidence is null with a reason. What was tested for, and by which method, is kept with the answer.
4. **One plan, several owners.** The organising unit is the pathway (CA-2): one pathway for the condition, a step per team, each step with **one** named owner, and a named coordinator for the whole. A step with no owner is shown as having none. My reasoning: the coordinator is a role a person holds and agrees to, never a default.
5. **Surveillance is a pathway template** (XP-12) that a clinician reviewed, with a source, a version and a date. The core ships none. A check that is due and has no owner is a hazard; so is one that nobody told the person about.
6. **A specialty view adds and never hides** ([care settings](../care-settings.md) section 3). Each team keeps its own view; the group by cause is shown in all of them, so that a lung clinic sees that a thyroid problem has the same cause.
7. **A change in one organ may matter to another team.** Telling every team everything makes noise that hides what matters. Which change reaches whom is a clinician's rule in the pathway, and not a default of the core.
8. **The person may know before the record does**, or not wish to know. A test before symptoms, a child's test, and the right not to know are consent questions that differ by country (a pack). A patient and a proxy read none of the problem list or relative history yet (OQ-EH1).
9. **A relative is a different patient.** A finding here may matter to them. The service holds no link between two people's records for this, makes no list of relatives, and recalls nobody ([relative history](../relative-history/index.md); the refusals of SG-2 stand). A clinician tells the patient, who tells the relative, or the country's law says otherwise.
10. **From child to adult care**: several teams change within a year or two and not together. Each hand-over is a step with both owners named, and the pathway is not closed until the last is taken up.
11. **A name that is rare identifies.** With a date and a place, a rare diagnosis can be one person. Counts, exports and research data sets suppress it ([research data set](../research-dataset/index.md)), and a group offer discloses it (H-065).
12. **A rare diagnosis may have no code, or several.** The title in words is always kept; a code is added beside it and never replaces it.

## Coding and exchange (search summaries only; read before design)

| Source | What a search summary says | Use |
|---|---|---|
| HL7 FHIR [`condition-dueTo`](https://hl7.org/fhir/extensions/StructureDefinition-condition-dueTo.html) | An extension on `Condition` that points at what caused it; trial use | The natural mapping of a problem's cause (EH-15) |
| SNOMED International, [map to Orphanet](https://conf.spaces.snomed.org/wiki/spaces/RMT/pages/1138229305), July 2026 | 6545 active mapped entries, from SNOMED CT to Orphanet, one to one; a user needs both licences | A code may be added beside the title. The licences are to be checked ([licenses](../licenses/index.md)) |
| HL7 FHIR [Genomics Reporting](https://hl7.org/fhir/uv/genomics-reporting/) | Version 3.0.0 on FHIR R4. How it carries a later reading of a variant was **not found** | RC-12 |
| CONCORD study, [UCL](https://www.ucl.ac.uk/epidemiology-health-care/research/applied-health-research/research/health-care-organisation-and-management-group/concord) | Many people with rare conditions have no coordinator, and information may not be shared between services | The need in points 4 and 6, in the United Kingdom |

## The worked example: not written

The acceptance of CF-4 asks for an example written **with a clinician**. It is left empty: the findings, their order, the surveillance, the test and its meaning, the teams and the transition are theirs to give. A test of the data shape without clinical meaning exists: a problem called "Syndrome" with two effects (`tests/entries_store.rs`).

## Not decided

| Id | Question | Who |
|---|---|---|
| OQ-GC1 | Which setting, country and ages; which teams; who coordinates today? | customer |
| OQ-GC2 | When a cause is refuted or entered in error, should someone be told, and who? Today it is only said when the list is read. | clinical lead, Clinical Safety Officer |
| OQ-GC3 | May a problem name the result that supports it, and what happens to it when the result is read again? | clinical geneticist |
| OQ-GC4 | Who may read a genetic result or a genetic cause: every treating team, or fewer? | customer, Clinical Safety Officer |
| OQ-GC5 | What is kept for a person who does not wish to know? | clinical geneticist, the country pack |
| OQ-GC6 | Which code systems, under which licences? | customer |

## Not done, not verified

Every source above was seen in search summaries only; none was read. No clinician, geneticist, patient or parent was asked. No screen shows a group, and FHIR carries no cause. A cause on a duplicate that was never merged is not found. Two people's records are never compared.
