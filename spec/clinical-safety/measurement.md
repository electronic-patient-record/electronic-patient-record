# Measuring clinical safety

Public-source research read 2026-11-04 plus this repository's own files. A proposal: nothing here is built, and no clinical safety officer has reviewed it.

Related: [Clinical safety](index.md), [Hazard removal](../hazard/index.md), the [hazard log](../../governance/clinical-safety/hazard-log.csv), the [risk management plan](../../governance/clinical-safety/risk-management-plan.md), [Usability](../usability/index.md), task PR-2 in [`tasks.md`](../../tasks.md). **NOT FOR CLINICAL USE.** This scaffold holds no clinical record, so no measure below has ever been run on real care. Nothing here says the project meets any standard.

## What 'safe' is taken to mean

1. **Safety is not the absence of reports.** A quiet incident log can mean a safe product or a product nobody reports on. "No incident reported" is already refused as removal evidence in [Hazard removal](../hazard/index.md).
2. **For a supplier, three questions.** Before deployment: are the known hazards controlled, and does a test show it? After deployment: are the controls still working in real use? Over time: is something new going wrong that no hazard names?
3. **Safety is a property of the product in its setting.** The Joint Commission's alert on health IT found the human-computer interface the largest single factor in the 120 events it reviewed (33 per cent; software or hardware 6 per cent) [JC54]. A supplier can measure its software and its audit trail; it cannot see the ward. Section "What software cannot measure" says what is left.
4. **A measure is evidence, not a verdict.** Only the Clinical Safety Officer (CSO, OQ-2, not yet named) can accept or close a hazard. Measures feed that person's judgement; they never replace it, and nothing here sets a status.

## Standards and the duty

Vendor and regulator text is separated from independent evidence below.

**Regulator and standard text (read as secondary summaries unless stated).**

- **DCB0129** (manufacturer) and **DCB0160** (deployer) are the English clinical risk management standards. NHS England's directory lists DCB0129 as "Clinical Risk Management: its Application in the Manufacture of Health IT Systems", amendment reference 24/2018, published 7 June 2018 [DCB0129]. NHS England's developer guidance (seen as a search snippet only) says the manufacturer must run a clinical risk management system, analyse and assess risk, give evidence it is managed, and present findings to the adopter [NHSDR]. Each side names its own CSO. A national review of both standards exists [NHSREV]; I saw only its title and so do not know what it changes. Check the current issue before relying on any detail here.
- **Hazard log and clinical safety case report** are the two artefacts the standards centre on. This repository holds a log of 44 hazards and an unsigned report template. Both are proposals, not a safety case.
- **ISO 14971** (risk management for medical devices) is the international parent method. NHS guidance compares it with the DCB standards and says the DCB standards add NHS-specific duties such as the CSO [NHSDR]. **IEC 62304** governs the software lifecycle; **IEC 82304-1** covers health software as a product and references 62304 and 14971 [IEC82304]. I saw catalogue and training pages only, not the standards. Whether this product is a medical device in any jurisdiction is not decided here (see Open questions).
- **No post-deployment duty is quoted here.** I did not read the post-market clauses of any of these standards. The surveillance plan below is a design, not a restatement of a requirement.

**Self-assessment and test tools.**

- **ONC SAFER Guides** (US) are self-assessment checklists of recommended practices for providers, in foundational, infrastructure and clinical-process groups, including Patient Identification, Test Results Review and Follow-up, Contingency Planning and CPOE with Decision Support [SAFER] [SAFERBLOG]. They are written for the deploying organisation, not the supplier; the repository's own mapping is task AS-3 and is not built [Clinical safety](index.md).
- **Leapfrog CPOE Evaluation Tool** (US) sends test orders through a hospital's live order system and records whether warnings fire. Its 2010 report covered 214 hospitals; the report and press summaries give 52 per cent of adult and 42.1 per cent of paediatric test orders not triggering an appropriate warning, and 94 per cent of repeat testers improving [LEAPFROG] [LFPRESS]. The figures are from the Leapfrog Group's own report; I did not check later results. The useful idea for us is the shape: a fixed, replayable set of unsafe test cases, scored.
- **Joint Commission Sentinel Event Alert 54** (2015) and **AHRQ PSNet** summaries give independent-ish evidence on categories of health IT harm [JC54] [PSNET54]. **ECRI** publishes annual health-technology hazard lists; I did not read them for this page and cite none.

## Kinds of measure

| Kind | Idea | Use for us | Caution |
|---|---|---|---|
| Structure, process, outcome | Donabedian (1966): the setting, the activities, the results [DONAB] | Structure: a control exists. Process: a control ran. Outcome: harm did or did not happen | Links between the three are assumed more than shown; outcomes need risk adjustment and can be gamed [DONAB] |
| Leading and lagging | Leading signals precede harm (backlog age); lagging signals follow it (incident count) | Prefer leading, computed from our data | A leading signal predicts only if the link is believed; say which link |
| Near-miss and incident reports | Staff report events caught or missed | A report route inside the product; hazards read from free text | Under-reporting is the norm; counts rise when reporting improves |
| Trigger tools | A manual look through a random sample of records for clues of harm; the IHI Global Trigger Tool counts events per 1,000 patient days and per 100 admissions [IHI] | A clinician-led review of a sample of our own records, once there is care to review | It is a screen, not a count of all events [IHI]; needs clinical reviewers |
| Validated EHR measures | Wrong-Patient Retract-and-Reorder (WP-RAR): an order retracted and re-placed for another patient by the same clinician within minutes; NQF-endorsed, 76 per cent of flagged events confirmed in interviews [WPRAR] | An identity near-miss analogue once we have orders or any retractable entry | Validation comes largely from its authors' group; I did not open the primary paper [WPRAR] |
| Alert override and burden | Override rate by alert type. A systematic review of 23 studies found average override rates of 46.2 to 96.2 per cent and widely varying appropriateness [OVERRIDE]; one study of about 157,000 alerts found 52.6 per cent overridden [NANJI] | Count alerts shown, overridden, with a reason, per alert type | A high rate is not proof of a bad alert, nor a low rate of a good one |
| Copy-forward and note bloat | A UCSF study of 23,630 progress notes found 18 per cent typed, 46 per cent copied, 36 per cent imported [COPY] | Provenance of note text (typed, copied, imported) when notes exist | One centre; a news summary was all I read |
| Result acknowledgement, lost to follow-up | Electronic triggers on abnormal results not acted on; the SAFER TRACKS trial uses the share of missed results and the share notified within seven days [TRACKS] | Referral and result loop closure (below) | Needs a defined "actionable"; I saw a registry entry and a search summary only |
| Usability testing | Task-based tests with measured errors, in [Usability](../usability/index.md) | Pre-deployment structure and process evidence | A lab test with few participants is not field use |

## Measures proposed for this project

Every measure is computed from tables the service already has or will have (audit events, the match queue, referrals, amendments), so none needs a new collection of staff data. "Review" names the role, not a person; the CSO does not exist yet. Thresholds are never fixed here (see the rule after the table).

| Hazard class | Measure | Definition | Source in our data | Review | Limits |
|---|---|---|---|---|---|
| Identity, wrong patient (H-001, H-002, H-020, H-021, H-033, H-041) | Unmatched-queue age and count | Count of items waiting in the unmatched queue; median and oldest age; number past the local target | Match queue timestamps | Maintainer weekly; CSO on any breach | A short queue can mean items auto-linked wrongly; pair with the next two |
| Identity | Merge and unmerge rate, and overlay rate | Merges (and unmerges) per 1,000 patient records per month; unmerges as a share of merges; an "overlay" is a record linked to a person later found to be another | Merge and unmerge events in the audit trail | CSO for each unmerge; maintainer for the trend | Low-numerator rates swing wildly; an unmerge shows a caught error, not all errors |
| Identity | Duplicate rate | Pairs the matcher scores above a review score, per 1,000 records, found after creation | Matcher scores and creation events | Maintainer | Depends on the matcher's own threshold; blind to duplicates it cannot see |
| Identity | Matcher explainability coverage | Share of links and rejections whose stored explanation names every compared field; share where absent evidence is `null` with a reason | Matcher output | Maintainer | Tests a property, not accuracy |
| Results and referrals lost (H-044, H-023, H-035) | Referral loop closure time | For referrals sent in a period: time from send to a matched outcome or decline; share open past the timeout | Referral and outcome records | Referring team, then CSO if rising | Closed is not clinically done; needs a defined timeout per urgency |
| Results and referrals | Orphaned outcomes | Outcomes or declines with no matched referral, count and age; referrals with a timeout and no alert on the referrer's worklist | Unmatched outcome list; worklist | Maintainer weekly | A zero can mean the matching key is too loose; sample by hand |
| Results and referrals | Rejected HL7 messages unseen | Refused messages not viewed by anyone within a set time | Ingest log, view events | Integration lead | Needs a "viewed" event to exist |
| Medication and allergy (H-018, H-019, H-022) | Allergy status "unknown" share, and ended-allergy events | Patients shown allergy "not recorded" as a share of those seen; allergy retractions per 1,000 allergy entries, and time to a second review | Allergy table, retraction events | Clinical lead; CSO | Cannot show an allergy is true; retraction may be right |
| Medication and allergy | Alert override and burden (when alerts exist) | Alerts shown per clinician-day; overridden share by alert type; overrides with no reason | Alert and override events | Clinical lead | See [OVERRIDE] [NANJI]; never target a rate in isolation |
| Display truncation (H-016, H-030, H-034) | List-cut-short events | Lists or timelines that hit a limit and showed the "cut short" notice, per 1,000 views; views where the notice was suppressed | Server list responses and UI event | Maintainer | Counts the notice, not whether the clinician saw it |
| Display truncation | Banner and unit checks | Share of vitals, ages and units displayed with the unit and reference range; failures of the banner contract test in CI | CI tests; render logs | Maintainer | Structure and process only |
| Downtime (H-025, H-039) | Snapshot staleness and downtime exercises | Age of the downtime snapshot at each check; exercises run, exercises failed; rate-limit refusals of a clinical request | Snapshot job, exercise record, rate-limit log | Site lead; CSO on any failure | An exercise is not a real outage |
| Data quality (H-003, H-004, H-040, H-043) | Rejected and corrected inputs | Invalid identifier rejections; edit conflicts caught; retried creates deduplicated; null-with-reason share vs. blank | Request log, constraint errors | Maintainer | Rejections can fall because input improved or because checks are bypassed |
| Data quality | Amendment turnaround | Patient correction requests: time to a decision, share accepted, denied, and shown to readers; denials with no stated reason | Amendment records | Privacy officer; CSO for a pattern | Accept and deny counts say nothing about whether the decision was right |
| Access and privacy (H-005 to H-011, H-024, H-042) | Break-glass use and review overdue | Break-glass opens per 1,000 accesses; reviews overdue past a set time; share opened with no later clinical event | Break-glass and review events | Privacy officer; CSO if overdue | Staff-level; see "Risks of measuring" |
| Access and privacy | Audit write failures and chain checks | Requests that succeeded with no audit row (should be zero); audit write errors; chain verification failures; archived ranges not verifiable | Audit trail, verifier job | Maintainer at once; CSO | A clean chain proves integrity, not that the right things were logged |

**Threshold-setting rule.** No threshold is set by the supplier alone or in advance of data. (1) Record a baseline over a stated period at a named site before any target. (2) Choose a signal that makes a person look (a run, a step, a drift), not a pass mark. (3) The CSO, the deployer and a clinical lead agree it and date it. (4) Zero-tolerance items (an access with no audit row, a chain break) need no baseline: any one is an event. (5) Review each threshold on a schedule; a threshold nobody has reviewed is not a control.

**Rates need denominators.** Each row above states both parts or is a count whose denominator the reviewer must name. Counts from one small site are shown with their numerator and denominator, never as a bare percentage, and never compared across sites without risk adjustment. Small numbers are read as a list of cases, not a trend.

## Linking measures to the hazard log

1. **Each hazard names at least one measure and one test.** PR-2 already checks that a control naming `path::test_name` points at a real test; only H-044 names tests so far [`tasks.md`](../../tasks.md). A proposed extension: a `measure` column with a measure id, and a check that every active hazard has one (a report first, as PR-2 did).
2. **Test pre-deployment, measure post-deployment.** The test shows the control works on a planted case (the removal-evidence rule in [Hazard removal](../hazard/index.md)). The measure shows it keeps working on real traffic. A hazard with a test and no measure is blind after go-live; a measure and no test is unproved.
3. **A measure breach reopens, never closes.** A breach sets a hazard to `open` through the existing "reopening is easy" rule and informs the CSO. No measure ever raises a status.
4. **Measures have owners and versions.** A measure's definition is a spec line, so a change to it follows the spec-first rule, and old values are not silently recomputed.
5. **Safety dashboard (idea).** A read-only page per site: each hazard class with its measures as numerator, denominator, trend, last review date and the threshold agreed. It shows "no data" in grey, never green, and shows cut-short lists and failed jobs on itself. It carries a banner stating it is not a safety assurance. No staff-level default view.

## After go-live: surveillance

Proposal only; the risk management plan states that no post-market surveillance process exists yet.

1. **Inputs.** Incident and near-miss reports (in-product form, email, support), measure breaches, audit and integrity job failures, support tickets mentioning a patient, and any outside notice (a dependency advisory, a terminology change).
2. **Triage.** Within a stated time, a maintainer classifies each input: no safety link, possible link, or clinical. Possible and clinical go to the CSO. When unsure, treat it as clinical (the repository's existing rule).
3. **Escalation.** Any of: a wrong-patient link or overlay, a lost referral or result, an access with no audit row, a chain break, harm reported. These go to the CSO at once and to the deployer's CSO; the deployer decides on its own local response (a DCB0160 duty, secondary source [NHSDR]).
4. **Learning.** Each case yields a hazard-log change (new hazard, changed cause or residual), a test, and a measure change; a case with none of these says why.
5. **Periodic review.** A dated review by the CSO of all measures, breaches and thresholds. Until a CSO exists, the review is not done and the dashboard says so.
6. **Reporting outward.** Whether and to whom a supplier must report (a regulator, the MHRA in the UK, other authorities) depends on whether the product is regulated as a medical device. I have not read those rules; this is an open question.

## What software cannot measure

- Harm that happens outside the system: a clinician acting on a memory, a paper note, a phone call.
- Whether a clinician read, understood and acted on what was shown. Display events are not comprehension.
- Missed diagnoses, delays that were never recorded, and patients who never reached the system.
- Whether clinical content, translations (machine-written until reviewed) or ratings are correct.
- Under-reporting, a safety culture, and whether staff feel able to report.
- Equity of harm: a measure can only be split by a group the data holds. Groups missing from the data are invisible, and the [equity rule](../02-scope.md) means the absence of a field must not read as agreement.
- Anything for a deployment that never turns the measures on or does not send us its numbers.

## Risks of measuring

- **Goodhart.** A target becomes the aim. Closing a referral to meet a time target is not a clinical close. Defence: measure outcomes with samples checked by hand, and never put a target on a count alone.
- **Gaming and hiding.** Staff stop reporting if reports are counted against them; items are closed or merged to clear a queue. Defence: report the unmatched-queue age, not the count alone; watch for a drop in near-miss reports as a signal.
- **Alarm fatigue.** Too many breaches teach people to ignore all of them, which is the very hazard the product aims to reduce in alerts [OVERRIDE]. Defence: few measures, each tied to a named hazard; retire a measure nobody acts on.
- **Staff-level privacy and fairness.** Break-glass, override and copy-forward data can single out a person. Defence: aggregate by default, individual views only to a named privacy role with an audited reason, and no use of these measures in discipline without a separate human process. The data is itself patient-adjacent and covered by the same audit trail.
- **False reassurance.** A green dashboard read as "safe". Defence: the banner above, "no data" in grey, and the limits column.
- **Cost and drag.** Measures that need a clinician's time cost care; sampling and a small set keep it affordable.

## Ideas

1. Add a `measure` column to the hazard log and a report of hazards without one (the PR-2 route).
2. A measures registry under `spec/clinical-safety/`, one spec entry per measure with definition, numerator, denominator, owner, review cadence and the hazards it serves.
3. Compute the identity measures first (queue age, merge and unmerge, duplicates): all exist in the scaffold.
4. A replayable "unsafe case pack" in the Leapfrog style [LEAPFROG]: fixed wrong-patient, allergy and lost-referral scenarios run in CI and at each deployment, with a stored score.
5. A WP-RAR-like measure for any retractable entry once such entries exist [WPRAR].
6. A planted-failure test for each measure: break the control and show the measure moves.
7. An in-product "report a concern" button that attaches the screen's record id, not patient data in free text.
8. A trigger-tool sample review by a clinician once real records exist, using a locally agreed trigger list [IHI].
9. A safety dashboard with "no data" shown as grey and limits shown beside each tile.
10. An annual "hazard class" review that checks the measures against new incidents.
11. Publish the aggregate, de-identified measures for open-source adopters so sites can compare against a baseline without sharing records.
12. A SAFER Guides mapping that cross-refers each practice to a measure, where one exists (AS-3).

## Open questions

1. Who is the CSO (OQ-2), and who is the deployer's? Until then nothing here can be reviewed.
2. Is the product a medical device, or in scope of DCB0129 or equivalents, in each target country? That changes whether reporting duties apply.
3. Where do measures run: in each deployment, centrally, or both? Central collection of safety data raises its own privacy question.
4. What is the minimum viable set: the hazard classes above give about sixteen rows; is that too many?
5. How are thresholds agreed in small sites, where one event is a large percentage?
6. Who may see staff-level data, and under which audited purpose?
7. Which equity splits (language, age, ethnicity, disability) can we hold lawfully, and how do we show the unrecorded?
8. How should a measure behave when the audit trail itself is the failing component?

## Not verified

- I read no standard in full. DCB0129 details come from the NHS England directory snippet and secondary summaries; IEC 82304-1, 62304 and 14971 from catalogue and training pages.
- I did not read the current DCB0129 or DCB0160 text, their post-market wording, or the national review [NHSREV].
- SAFER Guides: I saw a blog and news summaries, not the 2025 revision or any individual guide's text. A claim in one vendor page that CMS requires attestation is unverified and omitted.
- Leapfrog: the 2010 figures are from search summaries of the Group's own report; I did not open the PDF or look for later results.
- WP-RAR, override, copy-forward and SAFER TRACKS figures are from search summaries, not the papers. WP-RAR validation is largely the authors' own.
- ECRI and AHRQ material beyond a PSNet summary was not read.
- Nothing in this file has had clinical review. All measures are untested against real data, and no thresholds are proposed.

## Sources

Regulator, standard and vendor or organisation text:

- [DCB0129] NHS England standards directory, DCB0129: https://standards.nhs.uk/information-standards-notice/clinical-risk-management-its-application-in-the-manufacture-of-health-it-systems
- [NHSDR] NHS England digital regulations, complying with the clinical risk management standards, and DCB versus ISO 14971 (snippets): https://digitalregulations.innovation.nhs.uk/regulations-and-guidance-for-developers/all-developers-guidance/complying-with-nhs-digital-clinical-risk-management-standards/ and https://digitalregulations.innovation.nhs.uk/blog/dcb-standards-vs-iso-14971-what-healthcare-it-deployers-in-england-need-to-know/
- [NHSREV] NHS England, national review of DCB0129 and DCB0160 (title seen only): https://england.nhs.uk/long-read/national-review-of-clinical-risk-management-standardsdcb0129-and-dcb0160-supporting-information/
- [IEC82304] IEC 82304-1 catalogue entry (secondary): https://standards.iteh.ai/catalog/standards/iec/364734f0-08c7-44ef-a30f-8e3b116a166d/iec-82304-1-2016
- [SAFER] ONC SAFER Guides: https://www.healthit.gov/topic/safety/safer-guides ; [SAFERBLOG] https://healthit.gov/blog/electronic-health-and-medical-records/safer-guides-optimize-safety/
- [LEAPFROG] Leapfrog Group, CPOE Evaluation Tool results report: https://archive.leapfroggroup.org/sites/default/files/Files/NewCPOEEvaluationToolResultsReport.pdf ; [LFPRESS] https://www.hcinnovationgroup.com/clinical-it/clinical-documentation/article/13013747/leapfrog-group-releases-new-cpoe-study
- [IHI] IHI Global Trigger Tool: https://www.ihi.org/resources/pages/tools/ihiglobaltriggertoolformeasuringaes.aspx
- [JC54] Joint Commission Sentinel Event Alert 54: https://jointcommission.org/sea_issue_54 ; [PSNET54] AHRQ PSNet: https://psnet.ahrq.gov/issue/safe-use-health-information-technology
- [WPRAR] AHRQ Digital Healthcare Research, automated retract-and-reorder measures (institutional summary): https://digital.ahrq.gov/program-overview/research-stories/automated-retract-and-reorder-measures-improve-medication-safety

Independent or peer-reviewed material (read as summaries):

- [DONAB] Donabedian model overview, citing Donabedian 1966: https://pmc.ncbi.nlm.nih.gov/articles/PMC4196461
- [OVERRIDE] Poly et al., 2020, JMIR Med Inform, systematic review: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC7400042/
- [NANJI] Nanji et al., 2014, JAMIA: https://www.pubmed.ncbi.nlm.nih.gov/24166725/
- [COPY] UCSF progress-note provenance study, news summary: https://www.healthcareitnews.com/news/ehrs-are-overflowing-copy-and-paste-records-jama-study-shows
- [TRACKS] SAFER TRACKS trial entry: https://www.syfrah.com/trial/NCT04166240 ; AHRQ PSNet on lost follow-up: https://psnet.ahrq.gov/issue/why-test-results-are-still-getting-lost-follow-qualitative-study-implementation-gaps
