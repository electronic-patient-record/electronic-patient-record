# Measuring before-and-after productivity when phasing in this EPR

Public-source research read 2026-11-04 plus this repository's own files. A proposal: nothing here is built, and no clinician or trainer has reviewed it.

The question: how would a site that adopts this EPR know whether it made clinicians' work better or worse, and when is it safe to move the next ward, team or setting on? The answer has three parts: a baseline taken before go-live, a small set of measures that are chosen in advance and that include safety, and a roll-out designed so that the phasing itself is a comparison. "Productivity" is used carefully below: measures of time and effort are not measures of the quality of care, and [NFR-12](../07-non-functional-requirements.md) (support only) applies to anything computed from them.

## What to measure

Group the measures so that no group can be improved at the expense of another.

| Group | Examples | Typical source |
|---|---|---|
| EHR time | Total EHR time, note time, inbox time, order time, per 8 hours of scheduled patient time | Audit and usage logs |
| After-hours work ("pyjama time") | EHR time outside scheduled hours, per 8 scheduled hours | Logs, plus a schedule feed |
| Critical tasks | Clicks, seconds and errors for the top tasks (find a patient, record an allergy, prescribe, act on a result) | Instrumented test sessions |
| Workload and usability | SUS, NASA-TLX | Short surveys |
| Flow | Waiting times, length of stay, door-to-clinician time, discharge delay, appointments per session | Operational data |
| Wellbeing | Burnout item, intention to leave, sickness absence | Survey, HR (aggregated) |
| Safety (balancing) | Incident reports, near misses, wrong-patient events, overridden alerts, result-acknowledgement delay, downtime incidents | Incident system, audit |

Sinsky and colleagues proposed seven core measures of EHR use (total EHR time, work outside work, documentation time, prescription time, inbox time, teamwork for orders, undivided attention), each normalised per 8 hours of scheduled patient time so that physicians with different clinic loads can be compared (snippet of the table only; see Sources). Epic's Signal reports time in notes and "pajama time"; one registry entry defines pajama time as minutes per scheduled day charting outside 07:00 to 17:30 on weekdays plus unscheduled time on weekends and holidays. A national study uses a 30-minute buffer around scheduled appointments instead. These definitions differ, so a number from one source cannot be compared with another without checking the definition.

## Methods

- **Vendor metrics.** Epic Signal and Cerner (now Oracle Health) Lights On Network turn log data into EHR-use metrics. Both exist; they are products of those suppliers, not open standards. An Annals of Family Medicine paper notes that Epic's pajama time uses clock time while a time-outside-scheduled-hours measure refers to the patient schedule, and that vendor metrics likely understate after-hours work.
- **Audit-log metrics.** This project writes an audit row for every read and write (see below). The same events, aggregated, could give time-on-chart and task counts without a vendor product. They give activity, not effort or reason.
- **Time-motion (direct observation).** The most direct measure of what a person does, but costly and affected by being watched (see pitfalls).
- **Questionnaires.** SUS (benchmark mean 68, above average about 80 in one analysis of vendor-reported EHR scores). NASA-TLX has six dimensions (mental, physical and temporal demand, performance, effort, frustration). The Mini Z single burnout item has been compared with the full Maslach inventory and tends to give somewhat lower prevalence.
- **Interrupted time series (ITS).** Many measurements before and after go-live, with a control series where one exists. A published phased roll-out used ITS with control sites over about four and a half years of monthly data.
- **Stepped-wedge or phased roll-out.** Each ward or setting goes live in a planned order, so the order is the comparison: sites not yet live are the controls. It suits a phased roll-out and is more convincing if the order is randomised or at least not chosen by readiness alone.
- **Qualitative work.** In the US Department of Veterans Affairs Cerner transition, EHR-use metrics improved over 12 months while interviews reported persistent usability problems. Metrics and voices must be read together.

## The dip after go-live

Public evidence is thin and mixed.

- A 2025 systematic review of emergency departments (ten studies; a student project in a repository, not a journal paper) reports that big-bang replacements commonly cause a short dip, with length of stay and door-to-provider times back near baseline within about six months, and confounding limits confidence.
- One ITS evaluation of a phased roll-out reported emergency-department length of stay rising immediately and returning to baseline within 12 months (secondhand summary; productivity findings were mixed across regions).
- In the Veterans Affairs Cerner study, total EHR time, documentation time and order time fell sharply within four months and kept improving over 12 months.
- A US Veterans Affairs Inspector General report says facility leaders planned for a 30 percent productivity fall at a pilot site.
- Practitioner advice ranges from weeks to many months; that is opinion, not data.

Plan for a dip of months, measure its depth and length for this EPR, and do not set a target that assumes none.

## Pitfalls

- **Hawthorne effect.** A systematic review found most studies report some research-participation effect, with biases hard to bound. In a hand-hygiene study overt observation gave 78% compliance against 55% covert. Prefer log data for volume and use observation for explanation.
- **Goodhart's law.** A measure that becomes a target stops measuring well: minutes saved on notes can be bought with shorter, riskier notes. Pair every speed measure with a safety balancing measure and publish both.
- **Speed against safety.** A faster task that skips a check is worse. Treat a rise in override rate, wrong-patient events or late result acknowledgement as a stop signal even if times improve.
- **Weak link to satisfaction.** A KLAS analysis found Signal data predicted only about 4% of the variation in EHR satisfaction; another study found local work culture explained far more burnout variance than EHR metrics. Metrics are not a verdict.
- **Privacy and workforce relations.** Per-person monitoring invites distrust and, in the UK, a need for lawful basis, a data protection impact assessment and staff-side agreement. Report at team level; small cells need suppression.

## Phased roll-out design

1. **Baseline** at least several months before go-live, using the same definitions after as before.
2. **Pilot ward or clinic** with a named clinical lead, hypercare (extra on-site support) and a daily review.
3. **Per-setting phasing.** Acute, community and mental-health settings differ ([care settings](../care-settings.md)); do not assume one setting's result carries to another.
4. **Go/no-go criteria agreed before the pilot**, for example: no unresolved high-severity safety issue; incident rate within an agreed band; training completion and sign-off at the agreed level; downtime procedure tested; a rollback path. Numerical thresholds are for each site and its Clinical Safety Officer; none is proposed here.
5. **Hold points** between phases long enough to see the dip's trend.

## Benefits realisation frameworks

- NHS England's Global Digital Exemplar programme was evaluated independently by the University of Edinburgh as a longitudinal qualitative multi-case study; it stressed a clear vision, signed-off business cases, and tying benefits delivery to funding. It warns that programme impact is hard to separate from other change, and the programme-reported savings figures are not audited.
- HIMSS EMRAM is an eight-stage (0 to 7) adoption model created in 2005. It measures what is adopted, not benefit; its own validation differs by stage (a site visit for stage 7). Treat it as a maturity yardstick, not a productivity measure. Current criteria were not checked (see Not verified).

## Compared with this project

| Need | This project today | Gap |
|---|---|---|
| Baseline of clinician time before go-live | Nothing. [Usability](../usability/index.md) states targets (SUS >= 75, fewer clicks, less inbox time), NFR-10 asks for click and time budgets | No baseline protocol, no definitions, no instrument |
| Usage metrics from logs | Audit row per read and write, append-only; FHIR AuditEvent shape ([audit](../audit/index.md)) | Audit records access, not session time or task boundaries; no scheduled-time feed; no aggregation |
| Click and time budgets per critical task | Named in NFR-10; the UI spec lists no per-task budget | No budgets set, no timed test |
| Surveys (SUS, NASA-TLX, burnout) | SUS target only | No questionnaire, no collection tool |
| Phased roll-out support | Care settings and pathway design described as directions; nothing built | No per-setting feature flags, no pilot-ward mode, no go/no-go record |
| Safety balancing measures | Hazard log of 15 hazards for what exists; override-rate tracking named in NFR-10 | No incident-to-metric link; override rate not built |
| Privacy-safe reporting | [Privacy-preserving analysis](../privacy-analytics.md) is post-1.0; NFR-12 | No small-cell suppression rule for staff metrics |
| Control groups | None | No design, no comparison site |

## Candidate hazards

For the Clinical Safety Officer to assess; none is accepted, and none is yet in the hazard log.

- **PM-H1** A speed target reduces documentation completeness or checking. Controls to consider: paired balancing measures, no individual targets.
- **PM-H2** Staff-level monitoring leads to workarounds (shared logins, notes written elsewhere) that weaken the audit trail.
- **PM-H3** A metric on override rates pushes people to override less or to avoid alerts, hiding genuine risk.
- **PM-H4** A go-live dip is read as a failure, or as acceptable, and a safety signal is missed in it.
- **PM-H5** Metrics from one setting are used to justify moving another setting on.
- **PM-H6** Aggregate staff data re-identifies a person in a small team.

## Ideas

1. A measurement protocol document: definitions copied from Sinsky et al., normalised per 8 scheduled hours, with the definitions pinned so sites compare like with like.
2. Derive EHR-time and after-hours metrics from the audit stream at team level, with a minimum cell size, and say plainly what they cannot show.
3. A schedule-feed field (or a documented join) so "outside scheduled hours" can be computed.
4. Timed critical-task scripts run on the synthetic-patient environment, repeated every release, giving a regression series for clicks and seconds.
5. A short instrument pack: SUS, NASA-TLX, Mini Z items, in the supported languages (machine-written until a speaker reviews them).
6. A roll-out record per site: order of phases, hold points, go/no-go answers, decision-maker, kept in the clinical safety file.
7. A balancing-measure dashboard that cannot show a speed measure without its paired safety measure.
8. A publish-by-default approach: each site's before-and-after results shared in aggregate, so the project learns from open deployments.

## Open questions

- OQ-PM1 Who owns the measurement: the site, a regional body, or this project? What must the software provide, and what is local?
- OQ-PM2 Which staff groups are measured first (physicians, nurses, allied health, administrators)? Most published metrics are physician-centred.
- OQ-PM3 What staff-side agreements and data-protection basis does UK-first practice need for log-derived metrics?
- OQ-PM4 How should the equity rule ([scope](../02-scope.md)) shape measures for low-resource sites with no baseline staff or schedule data?
- OQ-PM5 Should control sites be randomised, and what is the ethical position when a safety benefit is expected?

## Not verified

- Only search results were seen for most sources; the Sinsky definitions came from a table snippet, not the paper.
- The ED systematic review is a repository thesis; its numbers were not checked against the studies.
- The ITS roll-out evaluation was seen as a search result; its exact recovery times were not read.
- Goodhart's law is cited from general knowledge; no source was read.
- Stepped-wedge EHR studies were seen as snippets; none is cited by name.
- HIMSS EMRAM current criteria, and any UK equivalent, were not found.
- No oral or written confirmation from Epic or Oracle of metric definitions was obtained; definitions come from academic papers.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://ahisp.ahima.org/Portals/0/Physician%20EHR_Table%201.pdf; https://www.ed.ac.uk/node/130610

## Sources

- Annals of Family Medicine, EHR use metrics definitions: https://www.annfammed.org/content/21/3/264.full.pdf
- KLAS, what Epic Signal data tell us about satisfaction and burnout: https://klasresearch.com/archcollaborative/report/what-can-epics-signal-data-tell-us-about-ehr-satisfaction-and-burnout/324
- Trial registry definition of pajama time: https://clinicaltrials.gov/study/NCT06812234
- Sinsky et al., JAMIA 2020 (table): https://pmc.ncbi.nlm.nih.gov/articles/PMC7075531/table/ocz223-T1
- National ambulatory EHR use study: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC11534958/
- Lights On, VA transition, JGIM: https://link.springer.com/10.1007/s11606-023-08318-w
- Standard metrics needed (Lights On context): https://ehrintelligence.com/news/standard-metrics-needed-to-measure-after-hours-clinician-ehr-work
- ED systematic review thesis: https://drepo.sdl.edu.sa/items/cb01f4cc-5843-4134-9e77-b2992b71dfcb
- Longitudinal ITS methodology, phased roll-out: https://nursing.jhu.edu/wp-content/uploads/2025/07/Comprehensive-methodology-to-monitor-longitudinal-change-patterns-during-EHR-implementations.pdf
- VA Inspector General, Spokane: https://www.oversight.gov/reports/review-access-care-and-capabilities-during-vas-transition-new-electronic-health-record
- SUS benchmarks for EHRs: https://ahisp.ahima.org/Portals/0/Physician%20EHR_Table%201.pdf
- NASA-TLX dimensions (secondary page): https://www.koji.so/docs/nasa-tlx
- Mini Z and the Maslach inventory: https://read.qxmd.com/read/30467949/cross-sectional-survey-of-workplace-stressors-associated-with-physician-burnout-measured-by-the-mini-z-and-the-maslach-burnout-inventory
- Mini Z 3.0 psychometrics: https://pmc.ncbi.nlm.nih.gov/articles/pmid/35048290
- Hawthorne effect review: https://pubmed.ncbi.nlm.nih.gov/24275499/
- Overt against covert hand-hygiene observation: https://bmcinfectdis.biomedcentral.com/articles/10.1186/s12879-018-3292-5
- GDE evaluation, Edinburgh: https://www.ed.ac.uk/node/130610
- GDE evaluation protocol: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC7545630/
- NHS Transformation Directorate, GDE shared learning: https://transform.england.nhs.uk/blogs/the-art-of-the-possible-shared-learning-from-the-gde-programme
- EMRAM overview: https://healthexec.com/topics/health-it/medical-records/himss-analytics-emram-could-travel-across-pond and https://healthcareitnews.com/node/434591
- This repository: [usability](../usability/index.md), [audit](../audit/index.md), [care settings](../care-settings.md), [NFR](../07-non-functional-requirements.md), [clinical safety](../clinical-safety/index.md)
