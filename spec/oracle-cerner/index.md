# Oracle Health (Cerner Millennium): comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

## What it is

- Oracle Health is the former Cerner Corporation. Cerner was founded in 1979 in Kansas City. Oracle announced the purchase on 20 December 2021 for about $28.3 billion; it closed in June 2022 and the company was renamed Oracle Health ([Wikipedia](https://en.wikipedia.org/wiki/Cerner)).
- Main product: Millennium, launched in 1997 as a unified architecture and the primary EHR platform ([Wikipedia](https://en.wikipedia.org/wiki/Cerner)). In October 2024 Oracle announced a "next-generation EHR" built for Oracle Cloud Infrastructure with an embedded Clinical AI Agent; an early-adopter programme was to start in 2025 and the release carries a timing disclaimer ([Oracle press release](https://www.oracle.com/news/announcement/oracle-unveils-next-generation-ehr-2024-10-29/); [Fierce Healthcare](https://www.fiercehealthcare.com/health-tech/oracle-health-will-offer-new-ehr-2025-embedded-ai-and-analytics-tools)).
- Markets: US hospitals and the US government (the Department of Veterans Affairs and the Department of Defense), the NHS, Canada, Australia, Sweden and others ([Wikipedia](https://en.wikipedia.org/wiki/Cerner)).
- Scale: KLAS (via trade press) put Oracle Health at 21.9% of US acute hospitals and 20.4% of beds in 2026, having lost 56 hospitals in 2025 ([HIT Consultant](https://hitconsultant.net/2026/05/14/klas-2026-ehr-market-share-report/); read through a search summary).

## Capabilities worth noting

- A single longitudinal architecture that served as a base for national-scale government deployments (Wikipedia, above; the DoD contract is described there as a 10-year, $4.3 billion award to Cerner, Accenture and Leidos, outcome not stated).
- Vendor claims for the new EHR: AI chart summaries, conversational search, voice navigation, and a data layer called Health Data Intelligence ([Fierce Healthcare](https://www.fiercehealthcare.com/health-tech/oracle-health-will-offer-new-ehr-2025-embedded-ai-and-analytics-tools), reporting vendor claims).
- Public developer route: a free sandbox through Code Console, with open, provider and patient sandbox tenants ([Oracle docs](https://docs.oracle.com/en/industries/health/millennium-platform-apis/mcfap/intro.html); sandbox details from third-party guide [Cleverence](https://www.cleverence.com/articles/oracle-documentation/fhir-r4-apis-oracle-health-millennium-platform-5831/)).

## Architecture and standards

- FHIR R4 APIs and non-FHIR APIs documented on Oracle's help centre ([Oracle docs](https://docs.oracle.com/en/industries/health/millennium-platform-apis/mcfap/intro.html), vendor). A third-party guide says each customer must enable an application before it works in production ([Cleverence](https://www.cleverence.com/articles/oracle-documentation/fhir-r4-apis-oracle-health-millennium-platform-5831/); not confirmed on Oracle's pages).
- The new EHR is stated by Oracle to be built "from the ground up" on Oracle Cloud Infrastructure (vendor).
- Internal data model and hosting of Millennium itself: I did not find a primary public statement.
- Openness: closed source; interface access via customer enablement.

## Fit across care settings

- Acute inpatient: the core market (KLAS summary above).
- UK: Millennium was an early NHS electronic patient record; trusts in recent coverage include Sheffield (go-live date reset) and Croydon ([MediFormatica tag page](https://www.mediformatica.com/tags/cerner-millennium-epr), secondary).
- Maternity: Croydon's planned maternity system will be built outside Millennium, per Digital Health in 2025 as summarised by the tag page above.
- ED, community, mental health and primary care: I did not find a primary source stating fit. Unknown.

## What independent sources report

Positive or neutral:
- The Department of Veterans Affairs resumed deployments in 2026 after a pause since 2023: Michigan sites on 11 April 2026, a Ohio/Kentucky batch in June, and VA reported completing its 2026 go-lives in October ([Nextgov, April 2026](https://nextgov.com/modernization/2026/04/va-resumes-ehr-rollouts-four-michigan-medical-sites/412807/); [Nextgov, October 2026](https://www.nextgov.com/modernization/2026/10/va-completes-2026-ehr-system-deployments-final-two-site-go-lives/416823/)). Counts differ between outlets.
- The October 2022 national NHS outage was traced by NHS Digital and Cerner to a faulty NHS-side server, not Millennium ([Digital Health](https://digitalhealth.net/2022/10/cerner-millennium-epr-resolved-national-outage)).

Problems:
- VA Office of Inspector General, 2022: from go-live in October 2020 to June 2021 the new record failed to deliver more than 11,000 orders for requested clinical services, held in an "unknown queue" that did not alert the ordering clinician; reviewers assessed 1,286 facility events and classified 149 patient adverse events (2 major, 52 moderate, 95 minor harm). The OIG found no evidence that Oracle Cerner gave VA actionable information about the queue before go-live; Oracle has disputed how the OIG characterised the data (figures seen in Healthcare IT News, MeriTalk and Fierce Healthcare coverage and a search summary; the OIG report itself was not read) ([Healthcare IT News](https://healthcareitnews.com/node/551455); [Fierce Healthcare](https://fiercehealthcare.com/health-tech/va-watchdog-finds-software-flaw-oracle-cerner-ehr-led-patient-harm)). Draft figures were "at least 148"; outlets round differently.
- September 2024 OIG reports counted 826 outages, degradations and functionality gaps meeting the contract's "major performance incident" definition ([Nextgov](https://nextgov.com/modernization/2024/09/vas-ehr-clocks-more-800-major-performance-incidents-go-live-watchdog-finds/399790)).
- 2024 reports of pharmacy errors in how prescriptions were logged ([Military Times](https://www.militarytimes.com/veterans/2024/02/15/more-pharmacy-problems-endanger-future-of-vas-health-records-system/)).
- Wikipedia lists a halted Västra Götaland rollout in Sweden (2024), a long Queensland implementation and a 2014 coroner finding that a missing migrated appointment list contributed to a child's death delay at Bristol; these are summaries, primary sources not read ([Wikipedia](https://en.wikipedia.org/wiki/Cerner)).
- Older NHS trust reports (2007 to 2013) describe finance and reporting strain at Royal Berkshire and limited benefits elsewhere; they may not reflect today's product ([Digital Health](https://www.digitalhealth.net/2013/02/royal-berks-finances-hit-by-millennium/)).

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
| --- | --- | --- | --- |
| Order routing | A routing flaw let orders sit in an unmonitored queue (OIG 2022) | Referral replies that cannot be matched go to a visible unmatched-reply queue; timeouts | Our design already targets this failure; test it with unseen destinations and publish the result |
| Medication lists | Import and logging errors reported | Medications not built | Plan migration checks before any import (spec: medications) |
| Availability | Hundreds of performance incidents reported at VA | Not measured at scale | Define downtime behaviour early, even for a scaffold |
| Interfaces | FHIR R4 plus gated enablement | FHIR read facade; HL7 v2 ADT inbound | Same lesson as other vendors: show access scope |
| Safety information | Vendor did not tell customer of a known flaw, per OIG | CHANGELOG states what is not done | Keep stating gaps in writing |
| Cloud-native rebuild | Announced 2024, status unclear | Single service | Do not announce what is not built |

## Ideas for this project

These are ideas, not decisions.
1. Add a "no silent destination" property test: any order or referral to a service with no owner must appear in a queue a named role reads (spec: closed-loop referral).
2. Write a downtime and degraded-mode section for the clinician UI, with the questions the OIG raised about outages (spec: clinician pathway design, mobile).
3. Record in the hazard log the failure patterns from public inspector-general reports as candidate hazards, for the Clinical Safety Officer to judge, never as accepted (hazard-log).
4. Require a pre-go-live disclosure list of known defects for any pilot, in the spirit of the OIG finding about unshared information.
5. When ADT/beds are built, keep legacy-appointment migration as an explicit acceptance test (Bristol finding).

## Not verified

- KLAS numbers and the VA rollout counts come from search summaries; outlets disagree (13 vs 14 facilities).
- I found no GAO report on the VA rollout in my search.
- The next-generation EHR's availability and adoption in 2025 and 2026 was not found.
- Wikipedia items (Sweden, Queensland, Bristol) were not traced to primary sources.
- Care-setting fit for ED, community, mental health and primary care was not found.
- The Cleverence guide is third-party; Oracle's own sandbox and enablement pages were not read in full.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://www.cleverence.com/articles/oracle-documentation/fhir-r4-apis-oracle-health-millennium-platform-5831/

## Sources

- https://en.wikipedia.org/wiki/Cerner : history, acquisition, controversies
- https://www.oracle.com/news/announcement/oracle-unveils-next-generation-ehr-2024-10-29/ : next-generation EHR announcement (vendor)
- https://www.fiercehealthcare.com/health-tech/oracle-health-will-offer-new-ehr-2025-embedded-ai-and-analytics-tools : trade press on the announcement
- https://hitconsultant.net/2026/05/14/klas-2026-ehr-market-share-report/ : KLAS 2026 share
- https://docs.oracle.com/en/industries/health/millennium-platform-apis/mcfap/intro.html : API documentation (vendor)
- https://www.cleverence.com/articles/oracle-documentation/fhir-r4-apis-oracle-health-millennium-platform-5831/ : third-party API guide
- https://healthcareitnews.com/node/551455 : OIG patient-harm report coverage
- https://fiercehealthcare.com/health-tech/va-watchdog-finds-software-flaw-oracle-cerner-ehr-led-patient-harm : OIG coverage
- https://nextgov.com/modernization/2024/09/vas-ehr-clocks-more-800-major-performance-incidents-go-live-watchdog-finds/399790 : performance incidents
- https://www.militarytimes.com/veterans/2024/02/15/more-pharmacy-problems-endanger-future-of-vas-health-records-system/ : pharmacy issues
- https://nextgov.com/modernization/2026/04/va-resumes-ehr-rollouts-four-michigan-medical-sites/412807/ : VA restart
- https://www.nextgov.com/modernization/2026/10/va-completes-2026-ehr-system-deployments-final-two-site-go-lives/416823/ : VA 2026 completion
- https://digitalhealth.net/2022/10/cerner-millennium-epr-resolved-national-outage : NHS outage cause
- https://www.digitalhealth.net/2013/02/royal-berks-finances-hit-by-millennium/ : older NHS trust experience
- https://www.mediformatica.com/tags/cerner-millennium-epr : recent NHS items, secondary
