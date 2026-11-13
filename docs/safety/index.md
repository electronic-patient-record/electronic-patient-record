# Safety: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

This note complements, and does not replace, [spec/clinical-safety/measurement.md](../../spec/clinical-safety/measurement.md), written separately. That file is the place for clinical safety measures; this note places safety among the other quality attributes, and where the two differ, that file and the hazard log govern.

## Definition

Safety is new as a top-level characteristic in ISO/IEC 25010:2023. A Japanese summary lists its sub-characteristics as operational constraint, risk identification, fail safe, hazard warning and safe integration, and tells readers to confirm the names in the standard ([search summary](https://www.issoh.co.jp/tech/details/5019/); snippet only, and I did not read the standard). My reading of each:

- Operational constraint: the product limits operation to safe parameters.
- Risk identification: the product helps identify risks that could harm life, health, property or environment.
- Fail safe: on failure it keeps or returns to a safe state.
- Hazard warning: it warns users of hazards in time to act.
- Safe integration: it stays safe when combined with other systems.

This is product safety in the 25010 sense. Clinical safety as a process (DCB0129, DCB0160, ISO 14971) is wider; the product characteristic is evidence for it, not a substitute.

## Why it matters for an EPR

- In England DCB0129 sets clinical risk management requirements for manufacturers of health IT and DCB0160 for the organisations that deploy and use them; both are published under section 250 of the Health and Social Care Act 2012 and were under review, with a consultation that opened on 29 June 2026 ([engagement page of England's national health body](https://www.engage.england.nhs.uk/patient-safety/national-review-of-clinical-risk-management-standa/); [national health standards](https://standards.nhs.uk/published-standards/clinical-risk-management-its-application-in-the-deployment-and-use-of-health-it-systems); snippets; current status not confirmed).
- ISO 14971 is the risk management standard behind medical device software; IEC 62304 gives software lifecycle processes and IEC 82304-1 product safety for health software ([overview, secondary](https://jamb.readthedocs.io/en/latest/_sources/iec-62304/overview.md.txt); [IEC 82304-1 listing](https://www.evs.ee/en/iec-82304-1-2016)). Whether any module is a device is open (OQ-6, AS-11).
- SAFER Guides are ONC-supported self-assessment tools for EHR safety, free of charge and disclaimed for legal compliance; the sources I saw date from 2014, and AS-3 in this repository refers to eight 2025 guides, which I did not check ([AJMC](https://www.ajmc.com/view/the-safer-guides-empowering-organizations-to-improve-the-safety-and-effectiveness-of-electronic-health-records); [ECRI](https://www.ecri.org/search-results/member-preview/ccrm/pages/iiccrm013114_onc/)).
- EHR-related events in reports: 557 of 1.735 million reported events used wording suggesting usability contributed ([Becker's](https://www.beckershospitalreview.com/healthcare-information-technology/ehrs/study-links-ehr-usability-issues-to-patient-harm/); secondary).
- Alert overriding is common (49 to 96 percent in a review), so a warning is only as safe as its tuning ([van der Sijs](https://pmc.ncbi.nlm.nih.gov/articles/PMC1447540)).

## What the project says today

- NFR-1: hazard log before shipping (DCB0129 shape); no automatic patient link on missing evidence. NFR-12: support only, no score may reduce what a patient is offered; a tripwire test reads column names. NFR-9: matching is pure and total with a score breakdown ([spec 07](../../spec/07-non-functional-requirements.md)).
- Built ([clinical safety](../../spec/clinical-safety/index.md)): a hazard log (44 rows by line count of `hazard-log.csv`, all the author's proposals, none reviewed), a draft risk management plan and safety case report template (unsigned), `scripts/check-hazard-log.py`, `scripts/check-clinical-impact.sh`, and a rule that only the Clinical Safety Officer may accept or close ([hazard removal](../../spec/hazard/index.md), [plan](../../governance/clinical-safety/risk-management-plan.md)). The register in the product (AS-17) landed in part.
- Fail safe, built in part: a write is refused when its audit row cannot be written (NFR-7, CQ-39, tested on 18 write routes by trigger, not a real storage fault); absent evidence is `null` with a reason, never zero; the clinical record is append-only; the allergy line says "Not recorded in this system. This does not mean there are none."
- Hazard warning: alerts are tiered in plan only; no CDS exists; override-rate tracking is NFR-10 (unbuilt).
- Safe integration: a feed refusal queue and ADT registration exist; the outbox sink logs only (OS-3, H-038). Request limits sit far above clinical use so as not to refuse a clinician (H-039).
- Not built: a named Clinical Safety Officer (OQ-2), any clinical review, a safety case report, SAFER mapping (AS-3), signed closure (AS-14), ISO 14971 readiness (AS-16), safety test cases traced to hazards. The GitHub workflow has never run there. This is a scaffold and not for clinical use.

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
|---|---|---|---|
| Hazards by status | Count by `status`, score band and age | None `accepted` or `closed` without a CSO; every open hazard has a control or reason | `hazard-log.csv`, `check-hazard-log.py --report` |
| Hazards naming a test | Share whose evidence is `path::test_name` that exists | Rising to all controlled hazards | `check-hazard-log.py` |
| Clinical-impact PRs without log change | Count of failures | Zero merges | `check-clinical-impact.sh` |
| Fail-safe coverage | Write routes with an audit-failure test over write routes | All, by census | CQ-39 census |
| Planted-failure checks | Controls whose test fails when the control is removed | Required before any closure (hazard removal rule) | Mutation runs, by hand |
| Safety test cases traced | Test cases mapped to hazard ids | Every control has one | Not built |
| Never-claim guard | Docs that say accepted, certified, reviewed or compliant without evidence | Zero | Doc census (partial) |
| Warning response | Alert shown, acknowledged, overridden, reason | Track; investigate high override | NFR-10 (no alerts yet) |

## Trade-offs with other attributes

- Reliability ([reliability](../reliability/index.md)): refusing a write when audit fails is safe but lowers availability; the project chose safe.
- Interaction capability ([interaction capability](../interaction-capability/index.md)): warnings need tuning; too many are overridden.
- Auditability ([auditability](../auditability/index.md)) supports learning from events but is not safety by itself.
- Interoperability ([interoperability](../interoperability/index.md)): accepting more feeds widens the integration hazard surface.
- Flexibility ([flexibility](../flexibility/index.md)): site configuration can alter a clinical rule; packs must not.
- Privacy ([privacy](../privacy/index.md)): consent denial can hide information a clinician needs; an emergency override needs a privacy-officer decision (SE-2).
- Maintainability ([maintainability](../maintainability/index.md)): every clinical-impact change costs hazard-log work, deliberately.

## Gaps and risks

- The ratings are the author's; no clinician has reviewed anything, so the log is a draft, not a safety case.
- The controls are tested by the author's own tests; the hazard rule ensures evidence exists, not that it is adequate.
- Standards under review in England may change shapes used here.
- Tripwire tests that read names are not proof (NFR-12).

## Ideas

1. Map each ISO 25010 safety sub-characteristic to hazard rows and controls (new task under AS-1).
2. Do AS-3, the SAFER Guides mapping, and mark every cell as author's reading.
3. Name a Clinical Safety Officer or record the absence in every release note (OQ-2).
4. Trace safety test cases to hazard ids (AS-1, testing strategy).
5. Run a real storage-fault test of the audit-refusal path, not a trigger (CQ-39 follow-up).
6. Write intended use and foreseeable misuse (AS-16).
7. Re-read DCB0129 once the England review concludes.

## Open questions

- Is any module a medical device (OQ-6, AS-11)?
- How is "hazard warning" counted when no alerts exist yet?
- Who counts as a competent reviewer outside England?

## Not verified

No standard text was read; ISO 25010 safety names are from one Japanese summary. I did not open the hazard log rows. The national review's status is unconfirmed. Nothing here is certified, compliant or reviewed.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://www.ecri.org/search-results/member-preview/ccrm/pages/iiccrm013114_onc/

## Sources

- https://www.issoh.co.jp/tech/details/5019/ - Japanese summary of 25010 safety (snippet)
- https://www.engage.england.nhs.uk/patient-safety/national-review-of-clinical-risk-management-standa/ - DCB review
- https://standards.nhs.uk/published-standards/clinical-risk-management-its-application-in-the-deployment-and-use-of-health-it-systems - DCB0160
- https://jamb.readthedocs.io/en/latest/_sources/iec-62304/overview.md.txt - IEC 62304 (secondary)
- https://www.evs.ee/en/iec-82304-1-2016 - IEC 82304-1 catalogue
- https://www.ajmc.com/view/the-safer-guides-empowering-organizations-to-improve-the-safety-and-effectiveness-of-electronic-health-records - SAFER Guides
- https://www.ecri.org/search-results/member-preview/ccrm/pages/iiccrm013114_onc/ - SAFER release notice
- https://www.beckershospitalreview.com/healthcare-information-technology/ehrs/study-links-ehr-usability-issues-to-patient-harm/ - EHR safety reports
- https://pmc.ncbi.nlm.nih.gov/articles/PMC1447540 - alert override review
