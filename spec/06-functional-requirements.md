## 6. Functional Requirements

Requirement IDs `FR-n.m` map to tasks in [`../tasks.md`](../tasks.md). Status is in [14](14-implementation-status.md);
at M0 none is met except FR-1.1 in part.

### 6.1 Identity (ID-, M1)
FR-1.1 Compare two patients with an explainable score (partly built). FR-1.2 Warn of likely duplicates at
registration; override needs a reason. FR-1.3 Merge and unmerge with provenance. FR-1.4 Search by name, birth date,
identifier, phonetic name. FR-1.5 FHIR `Patient/$match`.

### 6.2 Audit (AU-, M1)
FR-2.1 One AuditEvent per access, hash-chained. FR-2.2 Auditor-only read API; denial audited.

### 6.3 Record core (RC-, M2)
FR-3.1 Append-only store with history. FR-3.2 Problems, allergies, medications, immunisations, vitals. FR-3.3
Entered-in-error and amendment. FR-3.4 Terminology service. FR-3.5 Patient summary with persistent banner.

### 6.4 Security (SE-, M3)
FR-4.1 RBAC + ABAC + consent on every read. FR-4.2 Break-the-glass. FR-4.3 Label masking. FR-4.4 Multi-tenancy with
row-level security. FR-4.5 Patient access log.

### 6.5 Documentation (DO-, M4)
FR-5.1 Notes with addenda and co-sign. FR-5.2 Copy-forward provenance. FR-5.3 Structured forms. FR-5.4 Flowsheets and
NEWS2. FR-5.5 AI-drafted content is a draft with provenance and needs review.

### 6.6 Orders, results, CDS (OR-, M5)
FR-6.1 CPOE with identity re-confirmation. FR-6.2 Order sets. FR-6.3 HL7 v2 inbound. FR-6.4 Results inbox with closed
loop. FR-6.5 CDS Hooks with tiered alerts and captured overrides. FR-6.6 Alert analytics.

### 6.7 Interoperability (IO-, M6)
FR-7.1 CapabilityStatement. FR-7.2 SMART App Launch 2.2. FR-7.3 US Core / UK Core conformance. FR-7.4 Bulk `$export`.
FR-7.5 IPS `$summary`. FR-7.6 Subscriptions. FR-7.7 SQL-on-FHIR views. FR-7.8 EHI export.

### 6.8 Care coordination (CA-, M7)
FR-8.1 Encounters and ADT. FR-8.2 Care plans and tasks. FR-8.3 Referrals with closed loop. FR-8.4 Scheduling.
FR-8.5 Patient portal with proxy access. FR-8.6 Team inbox. FR-8.7 Downtime snapshot.

### 6.9 Assurance (AS-)
FR-9.1 Hazard log. FR-9.2 Governance pack. FR-9.3 SAFER mapping. FR-9.4 Usability programme. FR-9.5 Budgets.
FR-9.6 Load and chaos tests. FR-9.7 Backup and restore.
