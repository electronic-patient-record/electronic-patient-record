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
