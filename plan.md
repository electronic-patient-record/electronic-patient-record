# Plan: Electronic Patient Record

> **Status (2026-10-20).** Research and decision history, not a live queue.
> The work items derived from it live in [tasks.md](tasks.md); the canonical
> specification starts at [spec/index.md](spec/index.md). **No code exists
> yet.** Facts marked **[uncertain]** came from secondary sources or could not
> be confirmed during this research pass and must be checked against a primary
> source before anything depends on them.

Research date: 2026-10-20. Evidence base, stated plainly: web sources on
products, standards and regulation, plus the published papers cited in §5.
Not read: the source code of any product, any running instance, or any
standard's full normative text. Treat product feature lists as pointers to
verify, not as a spec.

## 1. Goal

Build open-source electronic patient record (EPR) software that:

1. keeps a **safe, longitudinal, append-only patient record** that is fully
   audited;
2. is **standards-native**: FHIR R4 is both the data model and the API, using
   SNOMED CT, LOINC and UCUM;
3. is **usable**: it targets the documented causes of clinician burnout
   (documentation burden, alert fatigue, inbox load, latency);
4. is **compliance-ready**, not claimed compliant: it supports a deployer's
   HIPAA, UK GDPR, NHS DSPT / DTAC, DCB0129 / DCB0160 and ONC (g)(10) work
   with evidence rather than slogans;
5. reuses the **project-portfolio-management tech stack and monorepo
   discipline** unchanged wherever it fits.

## 2. Tech stack and monorepo (copied from project-portfolio-management)

Source: `~/git/project-porfolio-management/project-porfolio-management`,
read 2026-10-20. Note that the user's path said "porforlio"; the directory on
disk is `project-porfolio-management`.

### 2.1 Layout

```
electronic-patient-record/
├── AGENTS.md  CLAUDE.md (@AGENTS.md)  README.md  index.md
├── plan.md  tasks.md  CHANGELOG.md  NEWS.md  RFC.md
├── CONTRIBUTING.md  GOVERNANCE.md  MAINTAINERS.md  SECURITY.md  CODEOWNERS
├── AI_STATEMENT.md  CITATION.cff  COMPARISONS.md  BENCHMARKS.md
├── LICENSE.md  LICENSE/{MIT,Apache-2.0,BSD-3-Clause,GPL-2.0-only,GPL-3.0-only}.txt
├── llms.txt  llms.json  lychee.toml  rust-toolchain.toml  compose.yaml
├── .github/{dependabot.yml, workflows/{ci,release,chaos}.yml}
├── agents/            shared agent guidance (spec-driven-development, testing, restful, models, matching, fhir)
├── spec/              entity-wide living spec, 01- … 18- plus cross-cutting topics
├── governance/        DPIA, DTAC checklist, record of processing, retention schedule,
│                      clinical-safety (hazard log, CRMP, safety case), accessibility statement
├── operations/        deployment, runbook, backup and restore, recovery objectives, downtime procedures
├── locales/           role-targeted docs per locale (en-gb-oxendict/for/chief-clinical-information-officer …)
├── scripts/           generate-status.sh, generate-llms.py, check-links.py, backup/restore, seed-demo.mjs, load-test.mjs, chaos-test.sh
├── electronic-patient-record-matcher-with-rust/    pure library: patient identity matching (MPI)
├── electronic-patient-record-service-with-rust/    loco.rs service: record, FHIR facade, audit, consent
├── electronic-patient-record-ui-with-svelte/       SvelteKit SPA with BFF
└── electronic-patient-record-skill/                agent skill for working in this repo
```

The matcher-to-MPI mapping is the natural reuse. In project-portfolio-management
the matcher is a pure, deterministic, explainable record matcher, and its
`Plan` type **is** the API DTO. Patient identity matching is the same problem,
with higher stakes: an overlay (two people merged into one record) is a patient
safety incident.

### 2.2 Versions (pin to what the reference repo pins at time of scaffold)

| Layer | Choice |
|---|---|
| Rust | 1.99.1 pinned in `rust-toolchain.toml`, MSRV 1.96, edition 2024 |
| Service framework | loco-rs 1.2 (Axum 0.8), `default-features = false`, **no `auth` feature** (no JWT / `rsa`) |
| ORM / DB | SeaORM 2.0, PostgreSQL 18, migrations crate `electronic-patient-record-migration` |
| Auth | PASETO v4.public verified offline with `authentication-verifier` (Ed25519 key set URL); OIDC upstream; SMART on FHIR 2.2 authorisation layered on top (see §6.4) |
| Integrity | `integrity-mac` for keyed MACs (audit hash chain), `sha2` / `sha3` digests |
| Search | `tantivy` 0.26 embedded index (patient search, typeahead); PostgreSQL for FHIR search parameters |
| Events | transactional outbox, optional `fluvio` feature for a durable bus (same as BUS-3) |
| Observability | `tracing`, JSON logs, `prometheus`, OTLP export |
| Matcher deps | serde, strsim, unicode-normalization, thiserror only. No IO, no clocks, no RNG |
| UI | SvelteKit (kit 3.x), Svelte 5 runes, TypeScript 6 strict, Vite 8, adapter-node, `@lilydesignsystem/*` widgets, `@sveltia/i18n`, `@fontsource/opendyslexic` |
| UI tests | vitest + @testing-library/svelte + jsdom; Playwright + @axe-core/playwright |
| Node / pkg | Node 26, pnpm |
| Containers | Podman (`podman compose`) in docs and scripts; CI builds with Podman; images to GHCR on `v*.*.*` tag |
| CI jobs | api, api-db, restore, matcher, deny (cargo-deny), ui, docs, container, zap (OWASP ZAP), fresh-clone, ci-ok; weekly chaos |
| Licence | `MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only` (same as the reference; see OQ-1) |
| Locales | ten locales, `<language>-<region>` names: `ar-001 cy-001 de-001 de-de en-001 es-001 fr-001 hi-001 zh-001 zh-cn` (the UI's `LOCALES`, copied from the reference) |

### 2.3 Working rules carried over

- **Spec first.** `spec/` plus each subproject's `spec/index.md` are the
  source of truth. A behavioural change is spec + code + test in one commit.
- **One item, one commit, one push**, then check CI. Stage named files only.
- **Say plainly what is not built or not tested.** Each task entry and each
  CHANGELOG entry says what was "Not built".
- Absent evidence is `null` with a reason, never zero. Every write is audited.
  People stay references.
- Computed styles use Svelte's `style:` directive (the CSP forbids inline
  `style=""`). One `DataTable` wrapper; Lily widgets; no other grid library.
- Machine-written translations are listed for review and never claimed
  reviewed.
- **Added for this domain:** every change answers "does this alter clinical
  risk?" and, if yes, updates the hazard log (DCB0129) before it merges.

## 3. Landscape

### 3.1 Commercial products (what users will compare us with)

| Product | What to learn |
|---|---|
| Epic | Depth and integration: MyChart portal, Care Everywhere / TEFCA exchange, Cosmos research data. Generative-AI assistants for clinicians, patients and revenue cycle announced at UGM 2025. |
| Oracle Health (Cerner) | A voice-first, agentic ambulatory EHR launched in 2025, with acute care planned for 2026. AI assistance is becoming table stakes. |
| MEDITECH Expanse | Web-based and mobile-first, with in-chart search and summarisation. Strong in community hospitals, Canada and the UK. |
| athenahealth | Multi-tenant SaaS: one code line, with network-wide learned rules shared across all tenants. |
| eClinicalWorks, NextGen | Ambulatory and specialty templates, built-in scribes, patient apps. |

### 3.2 Open source (where we sit)

| Project | Takeaway |
|---|---|
| OpenEMR (PHP, GPL-3) | Most-deployed open-source ambulatory EHR; ONC-certified (v8, 2026). A legacy stack. |
| OpenMRS (Java, MPL-2.0 + HD) | Proves a module/plugin model and a concept dictionary in low-resource settings. O3 micro-frontend UI. |
| Bahmni | A "distribution" built from best-of-breed open-source parts (EMR + ERP + LIS + PACS). |
| GNU Health | Hospital information system plus public health, with a federated design. |
| VistA | Decades of clinical depth (barcode medication administration). Also a warning about technical debt. |
| Medplum (TS, Apache-2.0) | Headless FHIR-native platform with AuditEvent, SMART and subscriptions, but **no complete EHR UI**. |
| HAPI FHIR (Java, Apache-2.0) | The reference FHIR server, validator and terminology toolkit. Use it as a **test oracle**, not a dependency. |
| EHRbase (Java, Apache-2.0) | openEHR data repository: the versioned-object and contribution model is a mature reference design. |
| Ottehr | Reference EHR UI on a proprietary FHIR platform. **[uncertain]** licence and whether it can run standalone. |

**The gap we fill.** Open source today offers either complete applications on
legacy stacks or modern headless FHIR platforms with no clinical UI. No widely
adopted project combines a FHIR-native core with a complete, safety-engineered
and usability-tested clinical UI. That combination, in a memory-safe language
with a small dependency tree, is the product.

### 3.3 UK NHS context

- **National services:** PDS (NHS number, FHIR R4), SCR, EPS (FHIR R4),
  e-Referral Service, National Record Locator, CIS2 authentication. GP
  Connect is a FHIR (STU3) API set.
- **Policy:** the 10 Year Health Plan (July 2025) commits to a "Single
  Patient Record" visible in the NHS App from 2028.
- **Assurance:** DTAC (refreshed March 2026); DCB0129 v4.2 for manufacturers
  and DCB0160 v3.2 for deployers, which require a Clinical Safety Officer,
  CRMP, hazard log and safety case. A review consultation closed in
  September 2026, so watch for a new version.
- **Medical device boundary:** CDS calculators and AI may count as Software as
  a Medical Device under UK MDR. Keep any such feature in a separate,
  separately-classified module.

## 4. Standards decisions

| Area | Decision | Why |
|---|---|---|
| FHIR version | **R4 (4.0.1)** internally and on the wire; design so R6 can be added as a second facade | R4 is the regulatory baseline almost everywhere. R5 adoption is thin. R6 is in ballot. |
| Profiles | **UK Core** and **US Core 6.1.0** (USCDI v3, required from 1 January 2026), tracking US Core 8.x (USCDI v5); **IPS** for patient summaries | These are the two first markets. IPS is the cross-border summary format. **[uncertain]** current UK Core and IPS release numbers. |
| App platform | **SMART App Launch 2.2** (EHR launch, standalone launch, PKCE, granular scopes, backend services) | ONC (g)(10). It also makes ambient scribes and specialty tools pluggable. |
| Decision support | **CDS Hooks 2.0** (`patient-view`, `order-select`, `order-sign`) plus an internal rule engine; CQL later | External CDS services are the norm. Do not build a drug knowledge base ourselves. |
| Population export | **Bulk Data 2.0** `$export` (NDJSON); **SQL-on-FHIR v2** ViewDefinitions for analytics | Keeps analytics off the clinical database. |
| Legacy feeds | **HL7 v2.5.1** inbound ADT / ORU / SIU through an adapter; outbound VXU later | Labs, ADT and devices still use v2. |
| Documents | IPS document `$summary`; **C-CDA R2.1** export only when a US network requires it | HTI-5 (proposed, not final) would drop C-CDA criteria. Do not invest early. |
| Terminology | SNOMED CT, LOINC, UCUM, ICD-10 / ICD-11, RxNorm (US), dm+d (UK), CVX. External terminology server (Snowstorm or Ontoserver) behind a `TerminologyService` trait, with an embedded subset for tests | Licensing and size make embedding the full content wrong. SNOMED is free in member countries. CPT needs an AMA licence. |
| Identity matching | Fellegi-Sunter style weighted matching in the matcher crate, with national identifiers (NHS number via PDS) as deterministic keys; FHIR `Patient/$match`; IHE PDQm | Reuses the project-portfolio-management matcher pattern. |
| Audit | FHIR **AuditEvent** using IHE **BALP** patterns; ATNA syslog export optional | ONC (d)(2) / (d)(3), NHS, HIPAA. |
| openEHR | **Not** the persistence model in 1.0; study its versioning (VERSIONED_OBJECT, contribution) and archetypes for form content | One canonical model (FHIR) keeps the team small. Revisit at M6. |

## 5. Usability and safety evidence (why the UI is designed the way it is)

| Finding | Source | Design consequence |
|---|---|---|
| Physicians rate their EHRs at a mean SUS of 45.9 ("F"). Each point gained is associated with about 3% lower odds of burnout. | Melnick et al., Mayo Clin Proc 2020;95(3):476 | Target SUS ≥ 75 in summative testing. Measure it every release. |
| Primary care physicians spend 5.9 hours per weekday in the EHR. Inbox work is 23.7% of that, and clerical work 44.2%. | Arndt et al., Ann Fam Med 2017;15(5):419 | Team inbox routing, delegation, bulk acknowledge of normal results, keyboard-first flows. |
| 50.1% of note text is duplicated (Penn, 104M notes, 2015–2020). | Steinkamp et al., JAMA Netw Open 2022;5(9):e2233348 | Copy-forward is visibly marked with its source. Notes default to concise problem-oriented structure. |
| The pooled drug-interaction alert override rate is 79.8%. | JMIR Med Inform 2020;8(7):e15653 | Tiered alerts (interruptive only when high severity and actionable), override reasons captured, override-rate dashboard, CDS governance. Bates "Ten Commandments" (JAMIA 2003). |
| ECRI 2026 hazards: #1 AI chatbot misuse, #2 "digital darkness" outages, #9 unsafe workflow design | ECRI Top 10, 2026 | AI output is provenance-tagged and review-required. Downtime snapshot and printable summaries. Safety review of workflows. |
| Wrong-patient errors | NISTIR 7865; Adelman et al., JAMA 2019 | Persistent patient banner (photo, two identifiers, allergies). Configurable limit on open charts. Identity re-confirmation on orders. |
| Ambient AI scribes give modest time savings | NEJM AI RCT (Lukac et al., 2025); UChicago 2025 | Integrate as a SMART app. Draft-only, clinician signs. FHIR Provenance records the AI agent. |
| Summative usability testing on critical tasks | NISTIR 7804 / 7804-1, ONC §170.315(g)(3) | At least 10 representative users per critical task (CPOE, medication list, allergies, problems, CDS, demographics). Report in NISTIR 7742 format. |
| SAFER Guides (2025 revision, 8 guides) | healthit.gov/safer | Ship a SAFER self-assessment mapped to features in `governance/`. |
| AMIA 25×5 initiative: cut documentation burden to 25% of current levels | dbmi.columbia.edu/25x5 | Track clicks and time per task as product metrics. |

## 6. Architecture

### 6.1 Trio composition

```
electronic-patient-record-ui-with-svelte  (SvelteKit SPA + BFF; browser holds no token)
   │  clinician: /patients, /patients/[id]/{summary,problems,meds,allergies,results,notes,orders,encounters}
   │  inbox, worklists, scheduling, admin; patient portal under /portal
   ▼  REST + FHIR (PUBLIC_API_BASE_URL, default :5150)
electronic-patient-record-service-with-rust  (loco.rs / Axum)
   │  /api/...       native REST for the UI (task-shaped, paginated)
   │  /fhir/R4/...   FHIR R4 facade: read, vread, search, history, create, update, $match, $everything, $export
   │  /.well-known/smart-configuration, /metadata (CapabilityStatement)
   │  policy engine (RBAC + ABAC + consent + break-the-glass) → every read and write
   │  audit writer (AuditEvent, hash-chained, keyed MAC) → every access
   │  outbox → subscriptions / webhooks / CDS Hooks / HL7 v2 adapter
   ├──► electronic-patient-record-matcher (path/crate dep; pure; Patient = API identity DTO)
   ├──► PostgreSQL 18 (resource + resource_version JSONB, search index tables, audit, outbox)
   ├──► tantivy (patient search / typeahead)
   └──► TerminologyService (Snowstorm / Ontoserver; embedded fixture subset in tests)
```

The dependency direction is strictly downward. The matcher depends on nothing
in the workspace.

### 6.2 Data model

- **Thin identity record**: the matcher's `Patient` (names, birth date,
  administrative sex, identifiers, addresses, telecom, links). This is the API
  DTO and is persisted as JSONB, the same partition idea as `Plan` versus
  operational sub-resources.
- **Clinical resources** are stored as FHIR R4 JSON in `resource_version`
  rows: (`resource_type`, `id`, `version_id`, `tenant_id`, `patient_id`,
  `recorded_at`, `author_ref`, `provenance_id`, `status`, `data JSONB`,
  `security_labels`). The current version is a materialised `resource_current`.
  Search parameters are extracted to typed index tables (token, string,
  date, reference, quantity), Medplum/HAPI style.
- **Append-only.** Updates write a new version. "Delete" sets
  `entered-in-error`. Signed notes accept only addenda. `_history` and "as
  seen at time T" queries come for free.
- **Provenance** on every write: who, on behalf of whom, when, why, and with
  what agent (human, device, AI).
- **Multi-tenancy** via `tenant_id` plus PostgreSQL row-level security,
  with per-tenant data-encryption keys (crypto-shredding for lawful erasure
  where retention permits).
- **Sensitivity**: FHIR security labels (`R`, `V`, `PSY`, `HIV`, `SUD`, `ETH`)
  drive segmentation and masking.

### 6.3 Access control

- Roles: clinician, nurse, pharmacist, allied health, receptionist,
  clinical admin, patient, proxy, auditor, system integrator.
- ABAC on top of roles: care-team or legitimate relationship, encounter,
  location, purpose of use and security label. Consent (FHIR `Consent`) is
  evaluated on every read.
- **Break-the-glass**: a mandatory reason, a time-boxed grant, real-time
  notification to the privacy officer, and a retrospective review queue.
- Patients can see their own access log ("who viewed my record").

### 6.4 Authentication

- Staff: OIDC (Entra ID / Keycloak / NHS CIS2) to the authentication service,
  which issues PASETO v4.public tokens. The service verifies them offline. MFA
  is required. Passkeys are supported. Fast user switching is a UI concern.
- SMART on FHIR: an authorisation server maps SMART scopes to the same policy
  engine. It could be the auth service or a module here (OQ-3).
- EPCS (US) and EPS (UK) prescribing need higher-assurance identity. These are
  deferred and recorded as out of scope for 1.0.

### 6.5 Downtime and offline

- A read-only **downtime snapshot**: per-location encrypted export of active
  patients' summaries, medications, allergies and MAR, refreshed on a
  schedule (ECRI #2).
- Printable patient summary (IPS) and medication administration record.
- Offline-first clinical entry is **not** in 1.0 (OQ-5). The design keeps
  writes append-only so a later sync queue can merge them.

### 6.6 Non-functional targets

| Target | Value |
|---|---|
| Chart open (summary) | p95 ≤ 1 s |
| Patient search / typeahead | p95 ≤ 200 ms |
| Order sign | p95 ≤ 2 s |
| FHIR read | p50 ≤ 10 ms |
| Availability | 99.9%, stateless app tier, PostgreSQL replication |
| RPO / RTO | ≤ 5 min / ≤ 4 h. Restoration capability ≤ 72 h is required by the proposed HIPAA rule |
| Accessibility | WCAG 2.2 AA, axe on every Playwright page |
| Usability | SUS ≥ 75 on summative tests; alert override rate tracked |

## 7. Scope

### 7.1 In scope for 1.0 (ambulatory first, inpatient core next)

1. Patient identity and registration (MPI, duplicate prevention, merge and unmerge)
2. Patient summary and banner
3. Problems, allergies, medications (list and reconciliation), immunisations,
   histories, vitals and flowsheets (NEWS2)
4. Encounters and episodes of care. Inpatient ADT core (admit, transfer,
   discharge, bed list)
5. Clinical notes (templates, co-sign, addenda, copy-forward provenance),
   structured forms (FHIR Questionnaire)
6. Orders (CPOE) for labs, imaging, referrals and medications, with order sets.
   Results inbox with acknowledgement and closed-loop follow-up
7. Prescribing with interaction checks through a CDS Hooks service (drug
   knowledge from a licensed third party)
8. Care plans, goals, care teams, tasks, referrals
9. Scheduling (schedules, slots, appointments, waitlist)
10. Patient portal (results, letters, messaging, booking, proxy access,
    access log, record download)
11. Audit, consent, break-the-glass, reporting basics, Bulk export
12. FHIR R4 API, SMART App Launch, CDS Hooks client, HL7 v2 inbound adapter

### 7.2 Out of scope for 1.0 (named so nobody assumes otherwise)

Billing and claims (X12, NHS PbR), a licensed drug knowledge base (we
integrate one), e-prescribing network certification (Surescripts, EPS),
EPCS, PACS (we link via ImagingStudy and DICOMweb), LIS/RIS internals, eMAR
with barcode scanning (planned for M7), ONC certification itself, eCQM / QRDA
reporting, public-health eCR, and offline-first data entry.

## 8. Compliance approach

The software **supports** compliance. It cannot **be** compliant on its own,
because compliance belongs to a deployment. Deliverables in `governance/`:

- DCB0129 clinical risk management plan, hazard log and clinical safety case
  template, with a named Clinical Safety Officer role (OQ-2).
- DTAC (2026 form) checklist, DPIA, record of processing, personal data
  inventory, retention schedule (UK Records Management Code of Practice 2023
  and US state ranges), accessibility statement.
- HIPAA Security Rule mapping, built to the January 2025 NPRM baseline
  (encryption at rest and in transit, MFA, asset inventory, 72-hour
  restoration, segmentation, scanning cadence). The rule is **not final**;
  the projection is July 2027.
- ONC §170.315(g)(10) readiness: run the Inferno test kit in CI. Note that
  HTI-5 is proposed, not final, so do not design around its removals.
- SAFER Guides 2025 self-assessment mapping.
- 42 CFR Part 2 (US substance-use records) segmentation via security labels.

## 9. Milestones

| M | Name | Exit criterion |
|---|---|---|
| M0 | Scaffold | Monorepo exists with the reference layout. All CI jobs are green on empty crates and UI shell. Spec chapters 01–18 drafted. |
| M1 | Identity | Matcher `Patient` with explainable scores. Patient CRUD, search, `$match`, merge and unmerge with provenance. Audit on every access. Registration UI with duplicate warning. |
| M2 | Record core | Resource store (append-only, versions, history, search params). Problems, allergies, medications, vitals, immunisations. Patient summary and banner UI. FHIR read/search/history. |
| M3 | Security | Policy engine (RBAC + ABAC + consent). Break-the-glass. Security labels. Patient access log. RLS multi-tenancy. ZAP clean. |
| M4 | Documentation | Notes, templates, addenda, co-sign, copy-forward provenance, Questionnaire forms, flowsheets and NEWS2. |
| M5 | Orders and results | CPOE, order sets, ServiceRequest/Task lifecycle, HL7 v2 ORU inbound, results inbox, acknowledgement and closed-loop tracking. CDS Hooks client with tiered alerts and override capture. |
| M6 | Interop and apps | SMART App Launch 2.2, CapabilityStatement, Bulk `$export`, IPS `$summary`, Inferno (g)(10) run in CI, US Core / UK Core validation. |
| M7 | Care and access | Encounters and ADT, care plans, referrals, scheduling, patient portal with proxy access, downtime snapshot. |
| M8 | Assurance | Hazard log complete for shipped features. NISTIR 7804 summative test (SUS ≥ 75). DTAC and DPIA drafts. Load test meets §6.6. Release 1.0 candidate. |

## 10. Risks

| Risk | Mitigation |
|---|---|
| Patient overlay (wrong merge) | Conservative thresholds, a human review queue, reversible merge (unmerge), matcher fuzz tests and a labelled test corpus |
| Drug knowledge licensing cost | CDS Hooks boundary. Ship with a demo-only fixture and say so. |
| Terminology licensing | External server. Document per-country licence duties (SNOMED affiliate, CPT/AMA). |
| Scope creep toward "everything Epic does" | §7.2 out-of-scope list. A new module needs a plan.md section and an owner. |
| Medical device classification | Calculators and AI stay in a separate module behind a flag, with a regulatory assessment before enabling them (OQ-6). |
| Regulation moving (HTI-5, HIPAA NPRM, DCB review) | §8 tracks state. Re-check dates at each milestone. |
| Usability regresses as features grow | Click and time budgets per critical task in Playwright. SUS every milestone from M4. |

## 11. Open questions (moved to spec/16-open-questions.md once written)

- **OQ-1** Licence: keep the five-way permissive/copyleft choice from the
  reference, or pick Apache-2.0 alone (the norm for HAPI, Medplum and
  EHRbase)?
- **OQ-2** Who is the named Clinical Safety Officer (a registered clinician),
  and is there a clinical advisory group?
- **OQ-3** Is the SMART authorisation server part of this service or the
  shared authentication service?
- **OQ-4** First market: UK (UK Core, PDS, DCB0129) or US (US Core, ONC)? The
  plan builds both profile sets; the order matters for M6.
- **OQ-5** Is offline-first data entry needed for target sites (low-resource,
  community, home visits)?
- **OQ-6** Which features would be SaMD, and is any wanted for 1.0?
- **OQ-7** Which terminology server: Snowstorm (open source, SNOMED only) or
  Ontoserver (commercial, broader)?

## 12. Items needing primary-source checks

HTI-3 and HTI-4 contents and dates; the HTI-5 final outcome; the NCPDP SCRIPT
2023011 deadline; HIPAA final rule timing; current UK Core and IPS release
versions; FHIR R6 publication date; OpenEMR's USCDI v5 claim; Ottehr's licence
and backend dependency; the Mirth Connect licence change; the exact TEFCA QHIN
count.

## 13. Sources

- Products: open-emr.org, openmrs.org, bahmni.org, gnuhealth.org,
  medplum.com/open-source, hapifhir.io, ehrbase.org; healthcareitnews.com and
  fiercehealthcare.com coverage of Epic UGM 2025 and Oracle Health 2025.
- Standards: hl7.org/fhir (R4, R6 ballot), build.fhir.org/ig/HL7/US-Core,
  hl7.org/fhir/smart-app-launch, cds-hooks.hl7.org,
  hl7.org/fhir/uv/bulkdata, sql-on-fhir.org, profiles.ihe.net,
  healthit.gov/isa (USCDI), inferno.healthit.gov.
- UK: digital.nhs.uk/developer, digital.nhs.uk/services/gp-connect,
  innovation.nhs.uk (DTAC 2026), england.nhs.uk (DCB0129/0160 review).
- US regulation: drummondgroup.com and reedsmith.com on HTI-5;
  fiercehealthcare.com on HIPAA Security Rule timing; healthit.gov TEFCA
  update (Feb 2026); healthit.gov/safer.
- Evidence: Melnick 2020 (Mayo Clin Proc); Arndt 2017 (Ann Fam Med);
  Steinkamp 2022 (JAMA Netw Open); JMIR Med Inform 2020;8(7):e15653; Bates
  2003 (JAMIA); ECRI Top 10 Hazards 2026; NISTIR 7804 / 7804-1 / 7741 / 7742 /
  7865; Lukac et al. 2025 (NEJM AI); AMIA 25×5.
