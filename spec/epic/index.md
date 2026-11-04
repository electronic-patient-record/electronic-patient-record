# Epic: comparison with this project

Public-source research read 2026-11-04. Vendor material is marketing; nothing here was verified by using the product; no private documentation was read. Direction only: nothing in this project was built from it.

## What it is

- Epic Systems is a privately held US company founded in 1979 by Judith Faulkner, based in Verona, Wisconsin. Source: [Wikipedia](https://en.wikipedia.org/wiki/Epic_Systems).
- Product names (per the same page): the Hyperspace client, the Chronicles real-time database, Clarity (relational) and Caboodle (warehouse), Cogito analytics, the MyChart patient portal, and the Cosmos de-identified data set.
- Epic's own software index lists Patient Experience, Standalone Specialties, Health Systems and Clinics, Post-Acute, Care in the Home, Social Safety Net, Interoperability and Artificial Intelligence ([epic.com/software](https://www.epic.com/software/)). The page did not describe individual modules.
- Market: KLAS reported (as summarised by trade press) that Epic held 43.7% of US acute care hospitals and 56.9% of beds in its 2026 report, with Oracle Health second and MEDITECH third ([HIT Consultant, 2026-05-14](https://hitconsultant.net/2026/05/14/klas-2026-ehr-market-share-report/)). I read this through a search summary, not the full report.
- Outside the US, Epic is used by several NHS trusts and by Norway's Helseplattformen (see below).

## Capabilities worth noting

- Patient portal at scale: Wikipedia says MyChart is used by 150 million patients in the US (figure from the article; its own source not checked) ([Wikipedia](https://en.wikipedia.org/wiki/Epic_Systems)).
- Cross-organisation record sharing through Care Everywhere and Share Everywhere (patient-authorised outside access) ([Wikipedia](https://en.wikipedia.org/wiki/Epic_Systems)). The same page reports press claims that Care Everywhere limited sharing with competitors' software, and that customers paid one-time costs to link each non-Epic system.
- A de-identified research warehouse, Cosmos, claimed by Epic-sourced figures at 296 million patients ([Wikipedia](https://en.wikipedia.org/wiki/Epic_Systems)).
- One integrated record across inpatient, ambulatory and community: the Royal Devon and Exeter trust went live in October 2021 and was reported as the first NHS site to use Epic across acute and community ([HSJ](https://www.hsj.co.uk/expert-briefings/the-download-inside-an-epic-implementation/7029845.article), trade press; paywalled, read through a search summary).
- Predictive models are sold inside the record (the sepsis model, evaluated below).

## Architecture and standards

- Epic says it strongly supports HL7 FHIR for REST interoperability and publishes a developer portal with a sandbox, client registration and endpoint lists including R4 ([fhir.epic.com](https://fhir.epic.com/Home), vendor).
- Third-party directories describe R4, STU3 and DSTU2 endpoints, SMART on FHIR, CDS Hooks and Bulk Data, and say production access needs a sponsoring health system to approve the client and its scopes; write access and subscriptions vary by site ([supergood.ai](https://supergood.ai/docs/epic-api), secondary and commercial).
- Hosting and the internal data model: Wikipedia names Chronicles as the core database. I could not find a primary, current statement of the hosting model or the database technology, so none is given here.
- Openness: the source is closed. Access to interfaces is gated by customer approval (see above).

## Fit across care settings

- Acute and ambulatory: the core market (KLAS figures above).
- Community: Royal Devon and Exeter, acute plus community, per HSJ above.
- Emergency, mental health, primary care: Epic's software index page did not list them by name, and I did not find a primary Epic page for each. Unknown from the sources I read.
- Home: "Care in the Home" is a listed category ([epic.com/software](https://www.epic.com/software/)).

## What independent sources report

Positive or neutral:
- Cambridge University Hospitals, the first big-bang NHS Epic go-live, was later reported by Digital Health as delivered on budget and on time, with staff expecting improvement ([Digital Health, 2017](https://www.digitalhealth.net/2017/08/three-years-on-cambridge-epic)). The earlier article reports staff who understood the case for a single record ([Digital Health, 2016](https://www.digitalhealth.net/2016/02/cambridge-big-bang-reality/)).
- Norway: a parliamentary committee majority opposed stopping Helseplattformen, calling that dramatic and harmful, and some municipalities reported benefits ([Altinget](https://www.altinget.no/digital/artikkel/ikke-flertall-for-aa-stanse-videre-innfoering-av-helseplattformen), read through a search summary).

Problems:
- Sepsis model: an external validation at Michigan Medicine (27,697 patients, 38,455 hospitalisations) found an AUC of 0.63, missed 67% of sepsis cases and alerted on 18% of hospitalised patients. Epic disputed the definitions and maintained the model still helped ([JAMA Internal Medicine, 2021](https://jamanetwork.com/journals/jamainternalmedicine/fullarticle/2781313); [Michigan IHPI](https://ihpi.umich.edu/news/popular-sepsis-prediction-tool-less-accurate-claimed)). Single centre.
- Norway: the national audit office (Riksrevisjonen, 2024) judged that weaknesses threatened patient safety, and the health supervisory authority received 43 notifications of serious incidents where the system may be a cause ([Medical Republic](https://www.medicalrepublic.com.au/lessons-from-norways-epic-rollout/126516), secondary; primary audit not read).
- In-basket load: a study of 934 physicians found about 47% of weekly in-basket messages were system-generated, and more of them went with more burnout ([Becker's](https://www.beckershospitalreview.com/ehrs/in-basket-messages-sent-from-the-ehr-may-drive-physician-burnout-study-finds)).
- Competition: antitrust suits by Particle Health, CureIS and the Texas Attorney General are allegations, not findings ([Wikipedia](https://en.wikipedia.org/wiki/Epic_Systems)).

## Compared with this project

| Area | This product | This project today | What we could learn or should avoid |
| --- | --- | --- | --- |
| Scope | Whole-hospital and ambulatory suite plus portal and warehouse | Scaffold: allergies, vitals, amendments, referrals | Breadth is years of work; do not chase it. Keep the order in spec |
| Interoperability | FHIR R4 with SMART, gated by customer approval (sources above) | FHIR R4/R5 read facade for three resources; HL7 v2 ADT inbound | Publish what is gated and why; make scopes visible to the clinician |
| Patient access | MyChart at scale | Amendment requests; no portal | Corrections are a small, safe first portal step |
| Decision support | Vendor-supplied models | None built | A model shipped to many sites needs local validation; see the sepsis finding |
| Audit and access | Not found in public sources I read | Hash-chained audit, break-glass, consent | Our audit is a strength; say what it does not cover |
| Messages | System-generated in-basket volume linked to burnout | Referral worklist with timeouts | Count what each automatic message costs the reader |
| Openness | Closed | Open spec and code | Keep every claim traceable |

## Ideas for this project

These are ideas, not decisions.
1. Test any future decision-support output against local data before showing it, and record the result next to the output (spec: CDS). Prompted by the sepsis validation.
2. Budget automatic messages: each generated worklist item must name its reader and an owner, to avoid the in-basket pattern (spec: closed-loop referral).
3. Treat community and acute as one record with different views, as the Exeter report suggests (spec: care-settings).
4. Plan a staged go-live note for any pilot site, since both Cambridge and Norway reports centre on rollout preparation (spec: clinician pathway design).
5. Publish an access-gating table for our FHIR facade (who may read what, and why) as Epic's gated model is a common integrator complaint in secondary sources.

## Not verified

- All market figures come from search summaries of trade press, not the KLAS report.
- Customer counts, MyChart reach and Cosmos size are Wikipedia figures sourced to Epic claims.
- Hosting, data model and mobile apps (Haiku, Canto, Rover): not confirmed from a primary page.
- HSJ and Altinget pages were read through search summaries.
- The Norwegian audit was not read directly.
- Nothing was tried in a product; usability is as reported by others.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://supergood.ai/docs/epic-api

## Sources

- https://en.wikipedia.org/wiki/Epic_Systems : history, products, criticisms, lawsuits
- https://www.epic.com/software/ : Epic's software categories (vendor)
- https://hitconsultant.net/2026/05/14/klas-2026-ehr-market-share-report/ : KLAS 2026 share summary
- https://fhir.epic.com/Home : Epic FHIR portal (vendor)
- https://supergood.ai/docs/epic-api : third-party API access notes
- https://jamanetwork.com/journals/jamainternalmedicine/fullarticle/2781313 : sepsis model validation
- https://ihpi.umich.edu/news/popular-sepsis-prediction-tool-less-accurate-claimed : university summary of the same
- https://www.digitalhealth.net/2017/08/three-years-on-cambridge-epic : Cambridge three years on
- https://www.digitalhealth.net/2016/02/cambridge-big-bang-reality/ : Cambridge go-live
- https://www.hsj.co.uk/expert-briefings/the-download-inside-an-epic-implementation/7029845.article : Exeter implementation
- https://www.medicalrepublic.com.au/lessons-from-norways-epic-rollout/126516 : Norway rollout
- https://www.altinget.no/digital/artikkel/ikke-flertall-for-aa-stanse-videre-innfoering-av-helseplattformen : Norwegian parliament vote
- https://www.beckershospitalreview.com/ehrs/in-basket-messages-sent-from-the-ehr-may-drive-physician-burnout-study-finds : in-basket study
