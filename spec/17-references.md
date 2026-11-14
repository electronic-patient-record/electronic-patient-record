## 17. References

Full list with citations: [`../plan.md`](../plan.md) §13. Key standards: HL7 FHIR R4 (hl7.org/fhir), US Core, UK Core,
IPS, SMART App Launch 2.2 (hl7.org/fhir/smart-app-launch), CDS Hooks (cds-hooks.hl7.org), Bulk Data
(hl7.org/fhir/uv/bulkdata), SQL-on-FHIR (sql-on-fhir.org), IHE profiles (profiles.ihe.net), USCDI (healthit.gov/isa),
Inferno (inferno.healthit.gov). Evidence: Melnick 2020; Arndt 2017; Steinkamp 2022; ECRI Top 10 Hazards 2026; NISTIR
7804 / 7741 / 7865. Regulation: DCB0129/DCB0160, DTAC, HIPAA Security Rule, ONC HTI-1/HTI-5, SAFER Guides.

Open-source systems read for the [benchmark](benchmark.md) (2026-10-30, plan.md §22.6): OpenEMR, Medplum, HAPI FHIR
and its JPA starter, OpenMRS core, Blaze, EHRbase, Metriport, Haste Health and Helios (both Rust), Marley Health
(formerly Frappe Health), VistA, HospitalRun (archived); Bahmni, Ozone HIS and GNU Health for capabilities only.

A proof-of-concept FHIR guide read in part for the [closed-loop referral model](interoperability/closed-loop-referral.md) (2026-11-02, plan.md §24.8): *Care Coordination and 360 Referrals* v0.1.0, <https://kevinmayfield.github.io/PatientCareCoordination/en/index.html> (seven pages; one author; no licence stated; "must not be used for implementation or clinical purposes").

The sister project, read for what this record can learn (2026-11-03, plan.md §25.7): the patient administration system in `~/git/patient-administration-system/patient-administration-system` at commit `4f75348` (a local checkout, same author; task titles and first lines, `spec/hazard/index.md`, `governance/production-readiness.md`, file names and commit messages; its code was not read).

Local CI/CD, read 2026-11-04 (plan.md §26.9): GitHub, *2026 pricing changes for GitHub Actions* (<https://github.com/resources/insights/2026-pricing-changes-for-github-actions>) and *Availability report: July 2026* (<https://github.blog/news-insights/company-news/github-availability-report-july-2026/>); GitHub Docs *Secure use* (<https://docs.github.com/en/actions/reference/security/secure-use>); Forgejo Actions (<https://forgejo.org/docs/latest/admin/actions/>, <https://forgejo.org/docs/next/user/actions/github-actions/>); nektos/act (<https://github.com/nektos/act>); Woodpecker CI (<https://woodpecker-ci.org/docs/intro>); Dagger integrations (<https://docs.dagger.io/integrations>); Depot's Rust CI guide, a vendor source (<https://depot.dev/blog/guide-to-faster-rust-builds-in-ci>). Third-party incident counts (IncidentHub, StatusGator) disagree and are indicative only.

Care pathway and episode of care, read 2026-11-04 (spec/05 §5.2.1): HL7 FHIR R4 `EpisodeOfCare` (<https://hl7.org/fhir/R4/episodeofcare.html>), `CarePlan` (<https://hl7.org/fhir/R4/careplan.html>) and `PlanDefinition` (<https://hl7.org/fhir/R4/plandefinition.html>). The page text was read in part (CarePlan and PlanDefinition pages are long; the first 100,000 characters only). The quotations in 5.2.1 come from these pages; the mapping in rule 5 is the author's reading and is to be confirmed.

Mobile, care settings, specialty views, pathway design and privacy-preserving analysis, read 2026-11-04 (plan.md §27.9):
W3C WCAG 2.2 criteria 2.5.8 Target Size (Minimum) and 1.4.10 Reflow (via secondary summaries: <https://kb.daisy.org/publishing/docs/wcag/target-size-minimum.html>; confirm on w3.org);
Digital Health and supplier case studies of community and mental-health EPR choices (<https://www.digitalhealth.net/2016/06/special-report-mental-health-systems/>, <https://www.theaccessgroup.com/en-gb/health-social-care/health-and-support-case-studies/health-related-case-studies/somerset-partnership-nhs-foundation-trust/>, <https://buildingbetterhealthcare.com/lincolnshire-nhs-trust-goes-live-with-rio-electronic-patient-record--148402>);
UK Renal Registry annual report and data-flow pages (<https://ukkidney.org/sites/renal.org/files/23rd_UKRR_ANNUAL_REPORT.pdf>, <https://renalregistry.atlassian.net/wiki/spaces/UD/pages/2359853069/Questions+and+Answers+-+UKRR>) and a primary-care renal-data study (<https://eprints.soton.ac.uk/398654/1/AcceptedNDT20160625.pdf>);
HL7 Clinical Practice Guidelines IG, documentation methodology and pathway definition profile (<https://hl7.org/fhir/uv/cpg/documentation-methodology.html>, <https://hl7.org/fhir/uv/cpg/StructureDefinition-cpg-pathwaydefinition.html>);
HomomorphicEncryption.org standard (<https://homomorphicencryption.org/standard/>), ISO/IEC 18033-6 (<https://www.iso.org/standard/67740.html>), a Chalmers thesis on FHE with differential privacy for healthcare analytics (<https://odr.chalmers.se/items/5d2cf46b-ce60-459c-95df-3c9bd6d53987>).
Most of these were seen as search results or in part; none was read in full.

Other EPRs, read 2026-11-04 (plan.md §28): each `spec/<vendor>/index.md` lists its own sources; most were seen as search results, and many are trade press or third-party guides.

Clinical safety measures, maternity, manual steps, data exchange, data quality, productivity and training, read 2026-11-04 (plan.md §29): each page lists its own sources; most were seen as search results.

System quality attributes, read 2026-11-04 (plan.md §30): each `docs/<attribute>/index.md` lists its own sources; ISO/IEC 25010:2023 was read only through secondary summaries.

Roles, protected information, care homes, virtual wards and step-down, read 2026-11-04 (plan.md §31): each page lists its own sources and how each was read. HL7 sensitivity codes: <https://www.hl7.org/fhir/R4/v3/InformationSensitivityPolicy/vs.html>. Gender Recognition Act 2004 s.22: <https://www.legislation.gov.uk/ukpga/2004/7/section/22>.

Bereavement, survivor grief, suicide watch and trauma treatment, read 2026-11-04 (plan.md §32): each page lists its own sources and how each was read. NICE NG225 and NG116 were read from third-party copies.

Welsh national health systems, read 2026-11-09 (plan.md §34): the Welsh Clinical Portal, Welsh GP Record, Welsh Immunisation System, Welsh Intensive Care Information System, Welsh Nursing Care Record and Welsh Community Care Information System (now Connecting Care). [`wales/index.md`](wales/index.md) lists 32 sources with how each was read; the Welsh national digital health body's pages are its own account, and Audit Wales (2018, 2020, 2022), the Senedd Public Accounts Committee (2018), Public Health Wales (2026) and Welsh Government circulars are the independent ones.

Suspicion along a referral, read 2026-11-09 (RS): Welsh Government on the Single Cancer Pathway and its waiting-times reports, and NICE NG12, NG18, NG71, NG97, NG106 and NG202, through search summaries. [`referral-suspicion/index.md`](referral-suspicion/index.md) lists them.

Clinical modules, patient portal, analytics, population health and research data, read 2026-11-14 (plan.md §35): each page lists its own keyed sources and how each was read; most study figures are from abstracts or search summaries, as each page's "Not verified" section says. Key sources:

- Inpatient and ambulatory workflows, and scheduling (IW, SC):
  - FHIR R4 Encounter: https://hl7.org/fhir/R4/encounter.html
  - FHIR R4 Appointment: https://hl7.org/fhir/R4/appointment.html
  - IHE ITI Scheduling: https://profiles.ihe.net/ITI/Scheduling/volume-1.html
  - HSSIB, outpatient appointments intended but not booked (2021): https://www.hssib.org.uk/patient-safety-investigations/outpatient-appointments-intended-but-not-booked-after-inpatient-stays/investigation-report/
  - Joint Commission Sentinel Event Alert 58: https://digitalassets.jointcommission.org/api/public/content/a05e74ef89484e2084b6511189b73a99
  - Forster et al. 2003: https://pubmed.ncbi.nlm.nih.gov/12558354/
  - Cochrane CD007458 (text reminders): https://www.cochrane.org/evidence/CD007458_mobile-phone-messaging-reminders-attendance-healthcare-appointments
  - McQueenie et al. 2019: https://doaj.org/article/8726f50dfd414d0e89238a59bd35dfdd
  - Samorani et al. 2022: https://papers.ssrn.com/abstract=3467047
- Clinical documentation and order entry (CN, OE):
  - Radley 2013: https://pmc.ncbi.nlm.nih.gov/articles/PMC3628057
  - Koppel 2005: https://pubmed.ncbi.nlm.nih.gov/15755942/
  - Adelman 2013: https://journalclub.wustl.edu/2013/08/13/understanding-and-preventing-wrong-patient-electronic-orders-a-randomized-controlled-trial/
  - van der Sijs 2006: https://pmc.ncbi.nlm.nih.gov/articles/PMC1447540
  - Paterno 2009: https://pmc.ncbi.nlm.nih.gov/articles/PMC2605599
  - Wang 2017: https://jamanetwork.com/journals/jamainternalmedicine/article-abstract/2629493
  - Rule 2021: https://pmc.ncbi.nlm.nih.gov/articles/PMC8290305/
  - Joint Commission Quick Safety 10: https://www.jointcommission.org/-/media/tjc/documents/newsletters/quick_safety_issue_10pdf.pdf
  - PRSB eDischarge summary: https://theprsb.org/standards/edischargesummary/
  - CDS Hooks 2.0: https://cds-hooks.hl7.org/2.0/
  - FHIR R4 ServiceRequest: https://hl7.org/fhir/R4/servicerequest.html
  - SAFER guides: https://healthit.gov/topic/safety/safer-guides
- Nursing flowsheets and medication administration (NF, MA):
  - RCP NEWS2: https://www.rcp.ac.uk/improving-care/resources/national-early-warning-score-news-2/
  - RCP NEWS2 additional guidance 2020: https://www.rcp.ac.uk/media/umzn4ntq/news2_additional-guidance-002-_0.pdf
  - FHIR R4 MedicationAdministration: https://hl7.org/fhir/R4/medicationadministration.html
  - Poon 2010, NEJM: https://pubmed.ncbi.nlm.nih.gov/20445181/
  - Koppel 2008, JAMIA: https://pmc.ncbi.nlm.nih.gov/articles/PMC2442264
  - Sjoding 2020, NEJM: https://pmc.ncbi.nlm.nih.gov/articles/PMC7808260
  - Collins 2018: https://pubmed.ncbi.nlm.nih.gov/30815074/
  - NICE CG50: https://www.nice.org.uk/guidance/cg50/chapter/1-Guidance
- Patient portal: results, messages, booking, bills and sharing (PP):
  - Steitz et al. 2023, JAMA Netw Open: https://doi.org/10.1001/jamanetworkopen.2023.3572
  - ASTP/ONC Preventing Harm FAQ: https://healthit.gov/faq/do-preventing-harm-exception-requirements-type-harm-align-hipaa-rules/
  - Holmgren et al. 2023, JAMA: https://pmc.ncbi.nlm.nih.gov/articles/PMC10408262
  - North et al. 2013, JAMIA: https://pmc.ncbi.nlm.nih.gov/articles/PMC3822104
  - Ip et al. 2021, JAMA Netw Open: https://pmc.ncbi.nlm.nih.gov/articles/PMC8446820
  - SMART App Launch 2.2 scopes: https://hl7.org/fhir/smart-app-launch/scopes-and-launch-context.html
  - IPS IG: https://hl7.org/fhir/uv/ips/
  - FHIR R4 Invoice: https://hl7.org/fhir/R4/invoice.html
  - HINTS Brief 52: https://hints.cancer.gov/docs/Briefs/HINTS_Brief_52.pdf
- Analytics model: operational reports and ad hoc exploration (AN):
  - SQL on FHIR 2.0.0: https://sql-on-fhir.org/ig/2.0.0/
  - FHIR Bulk Data export: https://hl7.org/fhir/uv/bulkdata/STU2/export.html
  - PostgreSQL 18 replication settings: https://www.postgresql.org/docs/18/runtime-config-replication.html
  - PostgreSQL 18 row filters: https://www.postgresql.org/docs/18/logical-replication-row-filter.html
  - PostgreSQL 18 row security: https://www.postgresql.org/docs/18/ddl-rowsecurity.html
  - OMOP CDM: https://ohdsi.github.io/CommonDataModel/
  - OpenSAFELY: https://docs.opensafely.org/how-opensafely-works
- Population health: care management, risk, outreach and quality measures (PH):
  - HL7 DEQM: https://hl7.org/fhir/us/davinci-deqm/
  - HL7 QI-Core: https://hl7.org/fhir/us/qicore/
  - HL7 Quality Measure IG: https://hl7.org/fhir/us/cqfmeasures/
  - HL7 Da Vinci ATR: https://hl7.org/fhir/us/davinci-atr/
  - NCQA, using HEDIS measures: https://ncqa.org/hedis/using-hedis-measures
  - Obermeyer et al. 2019: https://doi.org/10.1126/science.aax2342
  - TRIPOD+AI, BMJ 2024: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC11025451/
  - ONC HTI-1 decision support fact sheet: https://www.healthit.gov/sites/default/files/page/2023-12/HTI-1_DSI_fact%20sheet_508.pdf
- De-identified research dataset (RD):
  - 45 CFR 164.514: https://www.law.cornell.edu/cfr/text/45/164.514
  - HHS de-identification guidance: https://www.hhs.gov/hipaa/for-professionals/special-topics/de-identification/index.html
  - ICO anonymisation: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/data-sharing/anonymisation/how-do-we-ensure-anonymisation-is-effective/
  - National data opt-out operational policy: https://digital.nhs.uk/services/national-data-opt-out/operational-policy-guidance-document/what-are-national-data-opt-outs
  - SATRE: https://satre-specification.readthedocs.io/en/stable/
  - OMOP CDM: https://ohdsi.github.io/CommonDataModel/
  - Tarabichi et al. 2021 (Cosmos): https://pmc.ncbi.nlm.nih.gov/articles/PMC8775787
  - Sweeney 2002: https://www.cs.ubc.ca/~laks/cpsc504/kanonymity.pdf
  - Rocher et al. 2019: https://pmc.ncbi.nlm.nih.gov/articles/PMC6650473
  - OpenSAFELY: https://www.bennett.ox.ac.uk/blog/2025/01/how-opensafely-works/
