# Alcidion: comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

## What it is

- Alcidion Group Limited is an Australian-listed (ASX: ALC) health informatics company that says it was founded in 2000. Its main product is Miya Precision, which Alcidion describes as a FHIR-events platform ([alcidion.com](https://www.alcidion.com)). The user wrote "Alcidian"; the company is spelled Alcidion.
- Product names: Miya Precision (flow, tasking, noting, modular EPR and patient administration), Miya Observations and Assessments (the former Patientrack), Smartpage (clinical communications), Miya Results Tracking ([alcidion.com](https://www.alcidion.com); [Digital Health, Miya Results Tracking](https://www.digitalhealth.net/2024/03/alcidion-launches-miya-results-tracking-to-improve-patient-safety)).
- Patientrack came with the purchase of MKM Health and Patientrack, announced to the ASX on 24 April 2018 and completed 3 July 2018 (search summary of ASX filings, e.g. [ASX 2018-07-05](https://www.asx.com.au/asxpdf/20180705/pdf/43wb76j8lf9r28.pdf)).
- Markets: Australia, New Zealand and the UK. Company-reported 2023 figures: 95 healthcare organisations served, 2 billion observations recorded ([alcidion.com](https://www.alcidion.com), vendor).
- Financials (company ASX release, vendor-reported): FY25 revenue A$40.8M, ARR A$28.5M at June 2025 ([ASX 2025-08-28](https://announcements.asx.com.au/asxpdf/20250828/pdf/06ng58pgg8mfy9.pdf)). H1 FY26 results were released 26 February 2026 ([ASX 2026-02-26](https://announcements.asx.com.au/asxpdf/20260226/pdf/06wrxgdf6gvjpx.pdf)); I did not read them in full.

## Capabilities worth noting

- Electronic observations and early-warning scores with alerts (Patientrack, now Miya Observations), including a 1,250-bed NHS Lanarkshire deployment reported in earlier press ([Pulse IT tag page](https://pulseit.news/tag/patientrack)).
- Patient flow: digital journey boards replacing whiteboards, bed management, used in community hospitals in Herefordshire and Worcestershire ([Clinical Services Journal](https://clinicalservicesjournal.com/story/44600/herefordshire-and-worcestershire-health-and-care-nhs-trust-enhances-bed-manageme)).
- Results tracking and a modular EPR: North Cumbria signed a ten-year EPR contract on Miya Precision in February 2025 ([Digital Health](https://www.digitalhealth.net/2025/02/north-cumbria-signs-10-year-epr-contract-with-alcidion)).
- A Class 1 medical-device registration for an AI feature in Australia and the UK, reported in a search summary ([Pulse.bot blog](https://www.pulse.bot/healthtech/blogs/alcidion-delivering-software-as-a-medical-device-ai-capabilities-in-both-australia-and-the-uk-1193fafd-5a4b-45db-a4cc-602be6e0bd6a)); this is classification, not clinical evaluation.

## Architecture and standards

- Described by the vendor as an event-driven platform on FHIR, with real-time integration and FHIR interoperability ([alcidion.com](https://www.alcidion.com)). Independent confirmation of the architecture: none found.
- "Modular cloud-native EPR": vendor wording. Hosting details, data model, API documentation and licence: not found. Assume closed commercial software.

## Fit across care settings

- Acute inpatient and ED: flow and observations are the evidenced strengths (Royal Darwin ED-to-ward flow is a vendor case study, [casestudies.com](https://www.casestudies.com/company/alcidion/case-study/redesigning-access-and-discharge-at-royal-darwin-hospital)).
- Community: community hospital bed management in Herefordshire and Worcestershire (above). Community nursing records: unknown.
- Mental health: NHS Fife is cited by the vendor for risk assessments; wider mental-health fit unknown. Primary care: not a primary-care system; unknown beyond that.
- Operations centres for integrated care systems and remote patient management are listed by the vendor; no independent evidence found.

## What independent sources report

- Monash University evaluation of Miya Precision journey boards at Alfred Health (funded by the Digital Health CRC, released May 2024; Alfred Health, Monash and Alcidion collaborated): before deployment 25 to 40% mismatch between EMR data and ward whiteboards; workflow variations reduced from 30 to 9; bed-allocation call duration down 67% (the last two from a podcast summary) ([Hospital + Healthcare](https://hospitalhealth.com.au/content/technology/article/report-shows-benefits-of-electronic-journey-patient-boards-at-alfred-health-1666906116)). I did not reach the full report, and Alcidion was a collaborator, so this is not fully independent.
- Safety or adoption evaluations of Patientrack or Miya Observations by NHS England, HSSIB or a peer-reviewed journal: none found. The Manchester "reduced cardiac arrests" item is Alcidion-published ([casestudies.com](https://www.casestudies.com/company/alcidion/case-study/reducing-cardiac-arrests-and-critical-care-length-of-stay-at-manchester-university-nhs-foundation-trust)).
- Incidents, go-live problems or regulator findings: none found for Alcidion in this pass. Absence of evidence is not evidence of absence.

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
| --- | --- | --- | --- |
| Observations and early-warning scores | Core (Patientrack lineage) | Observations stored; no scoring | Scoring plus alert is the clinical value ([spec/pathway-design.md](../pathway-design.md)) |
| Patient flow, beds | Journey boards, bed management | Not built; ADT inbound only | Replace the whiteboard: measure data mismatch as a requirement |
| Event model | FHIR events platform (vendor) | Append-only FHIR-shaped record, audit chain | Our events are already append-only; expose them as subscriptions later |
| Results tracking | Miya Results Tracking | Not built; closed-loop referrals only | Closed-loop thinking applies to results too |
| Identity | Not found | Matcher, explainable scores | Differentiator |
| Consent, break-glass, audit | Not found publicly | Built | Not comparable |
| Modular adoption | Modules bought separately, EPR assembled | Monorepo, single product | Modular entry (flow first) is how a small supplier wins |
| Evidence | Vendor case studies, one academic evaluation | None clinical | Plan evaluation design early |

## Ideas for this project

These are ideas, not decisions.

1. Specialty and ward views: a journey-board view derived from the record, with a "mismatch with source" check (idea from the Alfred evaluation) ([spec/care-settings.md](../care-settings.md)).
2. In pathway design, model observation, score, alert and task as one chain with every step in the audit trail ([spec/pathway-design.md](../pathway-design.md)).
3. Extend the closed-loop referral pattern (sent, acknowledged, overdue, escalated) to results tracking as a candidate for the orders and results milestone.
4. Write an evaluation plan that measures workflow variation and data mismatch before and after, as the Monash design did, and state who is independent of the builder.
5. Consider a modular-entry note in the roadmap: which single module could be adopted alone by a ward ([spec/15-roadmap.md](../15-roadmap.md)).

## Not verified

- Every outcome figure on alcidion.com (42% fewer medical errors, 60 minutes less admin, and others) is vendor-stated.
- The Monash report was not read; figures come from trade press and a podcast summary.
- The Patientrack acquisition detail is from search summaries of ASX filings; a purchase price was reported but not confirmed.
- Press on Lanarkshire and ACT Health is from 2017 to 2020 and may be dated.
- Financial figures are company releases; the FY26 annual report was not found.
- Architecture claims (FHIR events) are unconfirmed by any independent source.
- Wikipedia has no company article under this name (the page is about a beetle genus).

## Sources

- https://www.alcidion.com (vendor, products and claims)
- https://announcements.asx.com.au/asxpdf/20250828/pdf/06ng58pgg8mfy9.pdf (FY25 results, company)
- https://announcements.asx.com.au/asxpdf/20260226/pdf/06wrxgdf6gvjpx.pdf (H1 FY26, company, not read in full)
- https://www.asx.com.au/asxpdf/20180705/pdf/43wb76j8lf9r28.pdf (2018 acquisition filing)
- https://www.digitalhealth.net/2025/02/north-cumbria-signs-10-year-epr-contract-with-alcidion (trade press)
- https://www.digitalhealth.net/2024/03/alcidion-launches-miya-results-tracking-to-improve-patient-safety (trade press)
- https://hospitalhealth.com.au/content/technology/article/report-shows-benefits-of-electronic-journey-patient-boards-at-alfred-health-1666906116 (Alfred Health evaluation)
- https://clinicalservicesjournal.com/story/44600/herefordshire-and-worcestershire-health-and-care-nhs-trust-enhances-bed-manageme (trust bed management)
- https://pulseit.news/tag/patientrack (trade press, older)
- https://www.casestudies.com/company/alcidion/case-study/reducing-cardiac-arrests-and-critical-care-length-of-stay-at-manchester-university-nhs-foundation-trust (vendor case study)
- https://www.casestudies.com/company/alcidion/case-study/redesigning-access-and-discharge-at-royal-darwin-hospital (vendor case study)
- https://www.pulse.bot/healthtech/blogs/alcidion-delivering-software-as-a-medical-device-ai-capabilities-in-both-australia-and-the-uk-1193fafd-5a4b-45db-a4cc-602be6e0bd6a (device classification, weak source)
