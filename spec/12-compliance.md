## 12. Compliance

The software **supports** compliance; it cannot **be** compliant. No certification is claimed.

- **Data classification**: all patient data is special-category (GDPR Art. 9) and PHI (HIPAA).
- **UK**: UK GDPR and Data (Use and Access) Act 2025; NHS DSPT; DTAC (2026 form); DCB0129 (manufacturer) and DCB0160
  (deployer) with a named Clinical Safety Officer, hazard log and safety case (a review of both standards was open in
  2026: re-check); Records Management Code of Practice 2023; the [MHRA](https://www.gov.uk/government/organisations/medicines-and-healthcare-products-regulatory-agency)
  (Medicines and Healthcare products Regulatory Agency) Software as a Medical Device boundary.
- **US**: HIPAA Security Rule (build to the January 2025 NPRM baseline: encryption, MFA, asset inventory, 72-hour
  restoration, segmentation, scanning cadence; not final); ONC §170.315 (g)(10), (d)(2)/(d)(3), (b)(10), (g)(3); HTI-1
  (USCDI v3 from 1 January 2026); HTI-5 is proposed, not final, so do not design around its removals; information
  blocking (45 CFR 171); 42 CFR Part 2 via the `SUD` label.
- **Standards**: ISO/IEC 27001:2022, ISO 27799, ISO 14971 and IEC 62304 if any module is a device, SAFER Guides (2025).

Deliverables are the governance pack (task AS-2). Items still needing primary-source confirmation: [`../plan.md`](../plan.md) §12.
