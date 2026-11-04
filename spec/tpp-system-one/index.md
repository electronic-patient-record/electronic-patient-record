# TPP SystmOne: comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

## What it is

- SystmOne is the clinical system of The Phoenix Partnership (TPP), based in Horsforth near Leeds. Source: [Wikipedia, SystmOne](https://en.wikipedia.org/wiki/SystmOne). Who owns TPP now: I could not confirm from the sources read.
- It is modular: GP practice, community services, child health, palliative care, hospital, A&E, maternity, mental health, prisons and social services modules. Source: [Wikipedia, SystmOne](https://en.wikipedia.org/wiki/SystmOne) (a secondary source, undated in parts).
- It is centrally hosted: the data sits on remote servers, not in each practice. Source: same.
- Scale (independent trade press and sponsored sources disagree): about 2,600 GP practices cited in [IT Pro](https://www.itpro.com/it-infrastructure/32155/errors-with-third-party-nhs-it-system-led-to-significant-patient-risk); about 40 per cent of English practices is repeated in vendor-linked pages. I found no NHS England count by supplier. English GP software is described as a duopoly with EMIS ([Wikipedia](https://en.wikipedia.org/wiki/SystmOne)).
- Markets: England mainly; also all prisons in England (133 prisons and young offender institutions, per Wikipedia).

## Capabilities worth noting

- One shared record across services, with a two-way sharing model: "sharing out" (what a service makes available) and "sharing in" (what a service may view), set per organisation. Source: [Derbyshire Healthcare FT guidance](https://focus.derbyshirehealthcareft.nhs.uk/clinical/electronic-patient-record-systmone/data-consent-and-sharing/faqs) and [TPP Enhanced Sharing training document v12.0](https://tpp-uk.com/wp-content/uploads/2024/12/EnhancedSharingTrainingDocumentv12.0.pdf).
- The Enhanced Sharing model records the basis for sharing at each organisation, and share-out settings also govern data flowing to GP Connect (same TPP document, as summarised by search; the document itself was not opened in full).
- A mental-health module with a dedicated screen for Mental Health Act information, section applications and treatment consent, per the vendor ([TPP mental health page](https://tpp-uk.com/products/mental-health/), seen only through a search summary).
- Mobile working: first used at Plymouth Community Healthcare mainly for remote access to records for community mental-health staff ([Digital Health, 2014](https://www.digitalhealth.net/2014/07/plymouth-first-for-tpp-mental-health/)); a touch-optimised mobile version described as usable on and off line (Digital Health listing, undated, [tag page](https://www.digitalhealth.net/tag/systmone/page/5/)).
- Patient-facing and third-party APIs (Client Integration, Patient Facing Services, Generic HTML, Telephony) with an integration request process: [TPP integration request](https://tpp-uk.com/integration-request/).

## Architecture and standards

- Single central hosted system; clinical coding historically Read codes ([Wikipedia](https://en.wikipedia.org/wiki/SystmOne)). Current SNOMED CT use: not verified here.
- Third parties reach the GP module through NHS England's IM1 pairing process and GP Connect; GP Connect is FHIR-based and TPP lists structured medications and allergies at full roll-out ([TPP interoperability status](https://tpp-uk.com/products/system-wide-functionality/interoperability/interoperability-status); [NHS England IM1 standards](https://digital.nhs.uk/developer/api-catalogue/interface-mechanism-1-standards)).
- TPP's own interface specifications are released after assessment of an application; public pages do not show endpoints (NHS England page above). It is a closed, proprietary product; no source code is public.
- Does it expose a native FHIR server for clinicians' own use: I could not confirm.

## Fit across care settings

- Primary care: the core market (see What it is).
- Community: a stated module and shared care with district nursing and GP services ([TPP mental health page](https://tpp-uk.com/products/mental-health/)).
- Mental health: modules deployed at, for example, Cheshire and Wirral Partnership ([TPP news](https://tpp-uk.com/news/cheshire-and-wirral-partnership-nhs-foundation-trust-go-live-with-systmone-mental-health/)) and Plymouth. Whether detention features work fully on the mobile app or offline: I could not confirm.
- Acute and A&E: modules exist; secondary-care modules were rolled out from 2013 and Torbay and South Devon began using maternity in 2022 ([Wikipedia](https://en.wikipedia.org/wiki/SystmOne)). Scale of acute adoption: not found.

## What independent sources report

Positive or neutral:
- Trust guidance shows the sharing model is understood and widely applied at the point of care (Derbyshire page above).

Problems reported by independent sources (these are incidents, not a verdict on the product today):
- 2009 to 2010: a review of West Yorkshire out-of-hours use found serious risk and called the system "unfit for the purpose for which it is being used"; TPP disputed parts and an action plan followed ([Digital Health, West Yorks report](https://www.digitalhealth.net/2010/02/west-yorks-report-identifies-it-concerns/); [action plan](https://www.digitalhealth.net/2010/06/west-yorks-action-plan-almost-complete)).
- 2018: opt-outs set between March 2015 and June 2018 for about 150,000 patients were not passed on, and cancelled prescriptions were not cancelled at pharmacies ([IT Pro](https://www.itpro.com/it-infrastructure/32155/errors-with-third-party-nhs-it-system-led-to-significant-patient-risk); [Wikipedia](https://en.wikipedia.org/wiki/SystmOne)).
- 2017: the Information Commissioner's Office issued a statement on the enhanced sharing model (Wikipedia, detail not read).
- 2022 to 2023: a coroner's prevention-of-future-deaths process led TPP to change SystmOne so an abnormal result could not be missed in the way shown ([TPP response](https://www.judiciary.uk/wp-content/uploads/2023/03/2023-0101-Response-from-TPP.pdf)).
- 2024: crashes tied to pathology printing and an electronic referral tool led to its being switched off for SystmOne practices in Essex ([Pulse](https://www.pulsetoday.co.uk/news/technology/electronic-referral-system-switched-off-after-continuing-it-issues/)).

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
|---|---|---|---|
| Consent and sharing | Per-organisation share out and share in, with a recorded basis | Consent and care-relationship access control with break-glass | Learn: sharing as two explicit directions with a recorded basis. Avoid: defaults that make a missing setting look like agreement |
| Central record | One hosted record across services | One append-only record, FHIR-shaped | Both are single-record designs; ours has no delete path |
| Mental Health Act | Dedicated MHA screen (vendor) | Not built; a pack is planned ([care-settings](../care-settings.md)) | Keep legal status as a record, not a flag |
| Mobile | Touch-optimised mobile, on and off line (undated) | No mobile layout ([mobile-viewability](../mobile-viewability.md)) | Offline writes need safe retries; our idempotency keys help |
| Opt-out and sharing defects | Documented 2018 errors | Audit trail is hash-chained | Test that a recorded preference is honoured end to end |
| Identity | Not found | Matcher with explainable scores, no auto-link on missing evidence | Our strength; keep |
| Openness | Closed, API by application | Open spec, open code | Publish interface contracts |
| Scale and acute depth | Roughly 40 per cent of English GP practices (vendor-linked) | M0 scaffold | Not comparable |

## Ideas for this project

These are ideas, not decisions.
1. Model sharing as two independent directions (offer, accept) per organisation, each with a recorded basis, on top of the existing consent and care-relationship checks (spec/access-control).
2. When a preference changes, prove by test that every outbound path (FHIR facade, HL7, exports) honours it; the 2018 opt-out error is the cautionary case.
3. For the care-settings profile, write the Mental Health Act status as a dated record with authority and restrictions (XP-7), and test it on a 320 px viewport.
4. For mobile, treat on and off line as a design question early: queue writes with the existing idempotency key and show unsent items visibly.
5. Add a "recorded the result, but could a clinician miss it" review to the clinician pathway design (spec/pathway-design.md), as the coroner-driven SystmOne change suggests.

## Not verified

- Current owner of TPP, current practice count, and any official NHS England share figure.
- Whether MHA features are complete on mobile or offline; whether SNOMED CT is native.
- TPP mental health page and Wikipedia were read via search summaries or a single fetch; the training PDF was not opened in full.
- The incidents above are dated (2009 to 2024); no 2025 to 2026 independent evaluation was found.
- Vendor claims (mobile, sharing) are marketing.

## Sources

- https://en.wikipedia.org/wiki/SystmOne : history, modules, 2017 and 2018 incidents (secondary)
- https://www.itpro.com/it-infrastructure/32155/errors-with-third-party-nhs-it-system-led-to-significant-patient-risk : 2018 errors, 2,600 practices
- https://www.digitalhealth.net/2010/02/west-yorks-report-identifies-it-concerns/ : West Yorkshire review
- https://www.digitalhealth.net/2010/06/west-yorks-action-plan-almost-complete : action plan
- https://www.pulsetoday.co.uk/news/technology/electronic-referral-system-switched-off-after-continuing-it-issues/ : Essex referral tool
- https://www.judiciary.uk/wp-content/uploads/2023/03/2023-0101-Response-from-TPP.pdf : TPP response to coroner
- https://tpp-uk.com/wp-content/uploads/2024/12/EnhancedSharingTrainingDocumentv12.0.pdf : vendor sharing model
- https://focus.derbyshirehealthcareft.nhs.uk/clinical/electronic-patient-record-systmone/data-consent-and-sharing/faqs : trust guidance on sharing
- https://tpp-uk.com/products/mental-health/ : vendor mental health product page
- https://www.digitalhealth.net/2014/07/plymouth-first-for-tpp-mental-health/ : first mental-health mobile use
- https://tpp-uk.com/news/cheshire-and-wirral-partnership-nhs-foundation-trust-go-live-with-systmone-mental-health/ : vendor news of a go-live
- https://tpp-uk.com/products/system-wide-functionality/interoperability/interoperability-status : vendor GP Connect status
- https://tpp-uk.com/integration-request/ : vendor API request process
- https://digital.nhs.uk/developer/api-catalogue/interface-mechanism-1-standards : NHS England IM1
- https://www.digitalhealth.net/tag/systmone/page/5/ : trade-press listing, mobile version
