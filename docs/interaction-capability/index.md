# Interaction capability: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

## Definition

ISO/IEC 25010:2023 renamed "usability" as "interaction capability". The sub-characteristics listed on the ISO 25000 site page are: appropriateness recognizability, learnability, operability, user error protection, user engagement, inclusivity, user assistance and self-descriptiveness ([ISO 25000 page](https://iso25000.com/index.php/en/iso-25000-standards/iso-25010); I saw a fetched summary, and the page did not state its edition year, so I cannot confirm it is the 2023 text). Changes from 2011, from two summaries: accessibility was split into inclusivity and user assistance; user interface aesthetics was replaced by user engagement; self-descriptiveness is new ([arc42 summary](https://quality.arc42.org/articles/iso-25010-update-2023)). The quality-in-use model moved out to ISO/IEC 25019 (search snippet, [VDE preview](https://www.vde-verlag.de/iec-normen/preview-pdf/info_isoiec25010%7Bed2.0%7Den.pdf)); so "satisfaction" and "effectiveness in use" are not part of this characteristic now.

Short reading for an EPR: can the intended clinician recognise that the system fits the task, learn it, operate it, avoid and recover from errors, and can people of varied abilities, languages and devices use it.

## Why it matters for an EPR

- Usability is a safety matter. A review of certification reports for 50 EHR vendors found that several certified products did not follow the required user-centred design testing, and 30 percent did not attest to a user-centred process ([Ratwani, JAMA 2015, via press report](https://www.dotmed.com/news/story/27044); secondary).
- A review of 1.735 million patient safety reports from 2013 to 2016 found 557 events (0.03 percent) whose wording suggested EHR usability contributed, for example an allergy alert that did not fire ([Becker's report of the study](https://www.beckershospitalreview.com/healthcare-information-technology/ehrs/study-links-ehr-usability-issues-to-patient-harm/); secondary, low rates reflect reporting not incidence).
- NISTIR 7804 defines an EHR usability protocol: expert review plus summative validation with representative users on realistic tasks, aimed at finding critical problems that could cause use errors ([NIST](https://nvlpubs.nist.gov/nistpubs/ir/2012/NIST.IR.7804.pdf); abstract only).
- Alert design: drug safety alerts are overridden in 49 to 96 percent of cases across studies, and many overrides are justified ([van der Sijs 2006 review](https://pmc.ncbi.nlm.nih.gov/articles/PMC1447540); the range comes from a snippet).
- UK public bodies must meet WCAG 2.2 AA and publish an accessibility statement ([service manual for UK public health services](https://service-manual.nhs.uk/accessibility/what-all-nhs-services-need-to-do); a legal reading needs the regulations themselves).

## What the project says today

- NFR-4 (WCAG 2.2 AA, axe on every page, keyboard operation of critical tasks, mobile viewability) and NFR-10 (SUS at least 75 in summative tests, click and time budgets, alert override rate tracked) in [spec 07](../../spec/07-non-functional-requirements.md); [usability](../../spec/usability/index.md) (planned); [mobile viewability](../../spec/mobile-viewability.md) (proposed 2026-11-04, nothing built).
- Built: Playwright with axe on the shell and many patient pages, desktop Chromium only (testing guide). Click budgets for register a patient (6) and record an allergy (5), failing above budget plus 10 percent (AS-5, partly landed). In-flight guards on write buttons with `role="status"` and `aria-busy` (CQ-33). Right-to-left check with axe for `ar-001`. An idempotency key on every create (OF-1). A patient banner with a plain allergy line: "Not recorded in this system. This does not mean there are none." (ID-7, RC-5, hazard H-016).
- Not built: phone-sized tests (XP-1), reflow and stacked tables (XP-2), small-screen safety rule (XP-3), a screen-reader audit or conformance report (AS-12), a usability programme and SUS study (AS-4), keyboard-only tests for critical tasks, time budgets, sign-order-acknowledge tasks that do not exist yet.
- Language: ten locales named; only English exists apart from one machine-written, unreviewed form; no translation is claimed reviewed (SF-4).
- The author's own record says nothing was tested with real clinicians; Playwright plus axe "does not prove usability with real clinicians" ([testing](../../agents/testing.md)).

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
|---|---|---|---|
| Automated accessibility violations | axe per page, per viewport and per locale | Zero on every page; add a page to the list when added | Playwright + axe (exists, desktop only) |
| Reflow at 320 CSS px | Playwright check for no horizontal scroll | Zero failing core screens | XP-1, XP-2 (not built) |
| Touch target size | Script over interactive elements | At least WCAG 2.5.8 minimum; choose a larger primary size after testing | XP-2 |
| Task clicks and time | Budget file per critical task | Budget plus 10 percent (as AS-5); add time once real tasks exist | `budgets.json` (exists for two tasks) |
| SUS | Questionnaire after summative test | At least 75 (NFR-10) with sample size stated; record the shortfall if missed | AS-4 (not run) |
| Critical-task error rate | NISTIR 7804 style summative test with clinicians | Set with the CSO; any use error that reaches the record is a hazard row | AS-4 |
| Alert override rate | Count overrides over alerts shown, by alert type | Track first; a rate is a question, not a verdict | NFR-10 (no alerts yet) |
| Wrong-patient recovery | Test that banner stays whole at 320 px and after tab restore | Zero truncation of name, date of birth, identifier | XP-3 |
| Translation review status | Share of strings reviewed by a named speaker | Report honestly; 0 today | Locale files |

## Trade-offs with other attributes

- Safety ([safety](../safety/index.md)): a tiered alert reduces fatigue but a suppressed alert is a hazard; NFR-12 forbids hiding or delaying care by score.
- Security ([security](../security/index.md)): idle lock and sign-in prompts cost clinician time; mobile adds device-loss risk.
- Performance ([performance-efficiency](../performance-efficiency/index.md)): chart open at p95 1 s is part of felt usability.
- Auditability ([auditability](../auditability/index.md)): refusing a write when the audit row fails is correct but looks like a fault to a clinician; the message must say so.
- Flexibility ([flexibility](../flexibility/index.md)): specialty views help specialists, but different layouts per site hurt rotating staff.

## Gaps and risks

- No user research with clinicians; targets such as SUS 75 come from literature, not from this product.
- Desktop-only automated tests while the direction is phone-first for community staff.
- Axe finds only a part of accessibility problems; I did not locate a source for a share, so none is given.
- Machine translation may mislead on clinical wording until reviewed.
- Banner, allergy and alert content on small screens is a wrong-patient risk (H-001).

## Ideas

1. Run XP-1 first and publish the failing-screens list before any redesign.
2. Add a Playwright project per phone class and a 200 percent and 400 percent zoom run (XP-1, XP-2).
3. Recruit a small panel of clinicians for a NISTIR 7804-shaped test of register, allergy and referral tasks; record the number actually tested (AS-4).
4. Add time budgets once the API is real (AS-5 follow-up).
5. Commission a manual screen-reader audit and publish an accessibility statement (AS-12).
6. Make "why was this refused" messages a tested part of the audit-failure path (CQ-39).
7. Track alert override with reason codes as soon as the first alert exists.

## Open questions

- Which tasks are the critical ones for the first release, and who decides?
- Who reviews translations, and how is that recorded (OQ not yet written)?
- Does offline entry matter (OQ-5)?

## Not verified

No ISO text read; the ISO 25000 page edition is unknown. Press reports of studies were not checked against the papers. No test of mine was run.

## Sources

- https://iso25000.com/index.php/en/iso-25000-standards/iso-25010 - sub-characteristic list (edition unclear)
- https://quality.arc42.org/articles/iso-25010-update-2023 - 2023 changes
- https://www.vde-verlag.de/iec-normen/preview-pdf/info_isoiec25010%7Bed2.0%7Den.pdf - VDE preview (listed, not read)
- https://www.dotmed.com/news/story/27044 - certification review press report
- https://www.beckershospitalreview.com/healthcare-information-technology/ehrs/study-links-ehr-usability-issues-to-patient-harm/ - safety-report analysis
- https://nvlpubs.nist.gov/nistpubs/ir/2012/NIST.IR.7804.pdf - NISTIR 7804
- https://pmc.ncbi.nlm.nih.gov/articles/PMC1447540 - alert override review
- https://service-manual.nhs.uk/accessibility/what-all-nhs-services-need-to-do - accessibility rules for UK public health services
