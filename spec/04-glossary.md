## 4. Glossary

- **EPR / EHR / EMR**: electronic patient record / health record / medical record. Used loosely; this project says EPR.
- **Care pathway**: the planned, goal-directed route a patient takes through care for a condition or need, across teams, settings, organisations and time; the organising unit of this record ([5.2.1](05-domain-model.md)). Held as a `CarePlan`, from a `PlanDefinition` template where one is used.
- **Encounter / episode of care**: one contact / a period in which an organisation takes responsibility, kept for tracking, reporting or billing. Both happen along a care pathway; neither is the unit the record is organised by, and no clinical rule depends on an episode existing.
- **Care setting / setting profile**: acute, community or mental-health care; a profile chooses the work-unit view, forms and legal-status records on one record ([care settings](care-settings.md)).
- **Specialty view**: how one specialty (for example renal) sees and works the same record, delivered as a pack; it adds and never hides safety information.
- **Homomorphic encryption (HE)**: computing on encrypted data; slow and limited, considered only for secondary-use analysis, never the clinical path ([privacy-preserving analysis](privacy-analytics.md)).
- **Problem list**: the clinician-maintained list of active and resolved conditions.
- **CPOE**: computerised provider order entry. **CDS**: clinical decision support.
- **MPI**: master patient index. **Overlay**: two people merged into one record (a safety incident). **Duplicate**: one
  person with two records.
- **Legitimate relationship**: the clinician is on the patient's care team (the basis for access).
- **Break-the-glass**: emergency override of access control with a mandatory reason, alert and review.
- **Entered-in-error**: a record withdrawn from default views but kept in history; the only form of "delete".
- **Closed loop**: an order or referral tracked until its result or outcome is received and acknowledged.
- **FHIR**: HL7 Fast Healthcare Interoperability Resources. **SMART**: app launch and OAuth profile for FHIR.
- **USCDI / US Core / UK Core / IPS**: US data set / US FHIR profiles / UK FHIR profiles / International Patient Summary.
- **DCB0129 / DCB0160**: NHS clinical risk management standards for manufacturers / deployers.
- **DTAC / DSPT**: NHS digital technology assessment criteria / data security and protection toolkit.
