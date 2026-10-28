# Plan: Electronic Patient Record

> **Status (2026-10-20).** Research and decision history, not a live queue.
> The work items derived from it live in [tasks.md](tasks.md); the canonical
> specification starts at [spec/index.md](spec/index.md). **Written when no
> code existed; code now exists** (see [spec/14](spec/14-implementation-status.md)). Facts marked **[uncertain]** came from secondary sources or could not
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
| Rust | 1.99.1 pinned in `rust-toolchain.toml`, MSRV 1.97, edition 2024 |
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

- Staff: OIDC (Entra ID / Keycloak / NHS CIS2). **Keycloak is built** (2026-10-20): the API verifies its access tokens directly (`operations/keycloak/README.md`); the rest goes through the authentication service,
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
reporting, public-health eCR, offline-first data entry, and emergency department triage (decided post-1.0 on
2026-10-23; see §15 and OQ-T2).

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

## 14. Research pass 2026-10-21: code quality and new capabilities

> **Status (2026-10-21).** §1–13 above were written before any code existed;
> their "no code exists" note is out of date (identity, audit, consent,
> break-the-glass, allergies, HL7 v2 ADT and the patient pages now exist). This
> section records a second pass: a read-only review of the code as it stands
> at `c361a8e`, and web research on capabilities that no task covers. The
> work items are in [tasks.md](tasks.md) under "Research 2026-10-21" (`CQ-`
> for code quality; new capabilities under their area prefix).

Evidence base, stated plainly: the source tree was read, not run, except
that `pnpm run check` passed and the matcher was exercised by a throwaway
probe crate outside the repository. No database test, no `EXPLAIN`, no CI run
(there is no remote, so CI has never run). Web facts marked **[uncertain]**
come from secondary sources only.

### 14.1 Patient-safety defects found (fix first)

These alter what a clinician sees or what the matcher decides, so each fix
needs a hazard-log row (AS-1) in the same commit.

| # | Defect | Evidence |
|---|---|---|
| 1 | `compare(a, b)` is not symmetric: with given-name lists of equal length the per-token average runs only from `a`, so `["John","J"]` vs `["John","Smith"]` is AutoLink one way (0.975) and Review the other (0.875). The only symmetry test uses an invalid date. | matcher `src/engine.rs:309`, `tests/matching.rs:60` |
| 2 | Placeholder NHS numbers (`0000000000`, `1111111111`, `9999999999` …) pass modulus 11 and count as a shared verified identifier, so two people carrying the same dummy number and birth date AutoLink with no name check. | matcher `src/nhs.rs:9` |
| 3 | A shared NHS number with two different full birth dates AutoLinks unless date similarity is below 0.5; spec §4 says any difference goes to Review. | matcher `src/engine.rs:180` |
| 4 | Birth-date "typo" credit (0.6) is given for any day, month or year difference (1950 vs 2010), and impossible dates (`2020-02-31`) parse. | matcher `src/engine.rs:350`, `src/normalize.rs:55` |
| 5 | A given name that folds to nothing scores `Some(0.0)` instead of `None`, breaking "absent is not zero". | matcher `src/engine.rs:302` |
| 6 | Wrong-patient display: moving from patient A to B leaves A's banner and allergies under B's URL until B loads, and a slow A response can overwrite B (no stale-response guard). `PatientBanner` already guards its photo correctly. | UI `src/routes/patients/[id]/+page.svelte:41` |
| 7 | Duplicate-creation race: the duplicate check runs outside the transaction and nothing serialises creation, so two concurrent registrations or ADT A04s (MLLP serves 8 connections) both create a record. | service `src/controllers/patients.rs:473`, `src/adt_feed.rs:272` |
| 8 | The once-a-minute background poll refreshes the BFF session, so an unattended workstation never reaches the 15-minute idle timeout. | UI `+page.svelte:100`, `src/lib/server/session.ts:72` |

### 14.2 Code-quality findings

**Service.** Non-test code has no `unwrap`/`expect`/`panic`, and all SQL is
parameterised. The main debts are:

- **Indexes defeated by casts.** Eleven lookups compare `patient_id::text = lower($1)`, which defeats the primary-key and lookup indexes. `*_current` views run `DISTINCT ON` over whole version tables.
- **N+1 consent checks.** These run per search result: about 100 queries for 50 results.
- **Misplaced shared plumbing.** Shared controller helpers live in `controllers/patients.rs`, and errors are hand-built `json!` bodies inside `Box<Response>`.
- **Copied store boilerplate.** It is copied six times, and version conflicts are detected by matching `"_pkey"` in the error text.
- **FHIR re-parses JSON responses.** The FHIR layer calls the JSON handlers and parses their HTTP responses back, with `unwrap_or_default` turning failures into `null`.
- **Malformed FHIR query.** A malformed `AuditEvent` query becomes an unfiltered search.
- **Audit export gaps.** The NDJSON export drops rows that fail to serialise, leaving a gap in a chain meant to be re-verified.
- **Inconsistent error bodies.** The route gate returns `text/plain` even on `/fhir/R4`.
- **Down migrations are a delete path.** `down()` drops clinical tables, and triggers do not fire on `DROP`.
- **No handler-level tests.** Controller wiring is checked only by hand.
- **No observability.** There are no request ids, spans or metrics.
- **Audit lock is a throughput ceiling.** One global advisory lock serialises every audited request.

**Matcher.** Beyond §14.1:

- **Name matching.** Jaro-Winkler prefix bias (`Smith-Jones` vs `Smithson` passes the family guard). `John` vs `John Paul` AutoLinks, a twins risk. There is no transliteration, no nickname or phonetic matching, and no given/family swap.
- **Name folding.** `fold_name` splits Indic and Persian names on combining marks.
- **Phone numbers.** They are not E.164-normalised, and a shared household phone or postcode counts as strong evidence.
- **Free-text output.** Conflicts and notes are free English strings.
- **Configuration.** `MatchConfig` is unvalidated, and `min_evidence_weight` cannot bind with the default weights.
- **Tests.** There are 13 example tests and no property, mutation or fuzz tests.

**UI.**

- **Proxy errors.** The upstream call in the proxy has no timeout, and a transport error throws a bare 500.
- **Refresh failures.** Any refresh failure, including a network blip, deletes the session.
- **Unchecked response shapes.** API responses are cast (`as T`), not decoded.
- **Repeated status handling.** Each page repeats the status-code chain.
- **In-flight buttons.** Buttons are not disabled while a request is in flight.
- **Layout and accessibility.** Physical CSS `left` properties are used under `dir="rtl"`, and the loading text has no `role="status"`.
- **Localisation.** One English `STRINGS` object ignores the locale, and times are shown as UTC text, a misread risk in BST.
- **Versioning header.** `Accepts-version` is neither sent nor forwarded.
- **Forwarded headers.** `isHttps` trusts a client-supplied `X-Forwarded-Proto`.
- **Test gaps.** There are no component tests, and the proxy handler and hooks are untested.

**Infrastructure and documents.**

- **CI has never run.** Some action versions may not exist.
- **Hazard-log job.** It ignores a `clinical-impact` label added after the PR opens, because there is no `labeled` trigger. `reviewed_by` is free text with no Clinical Safety Officer allow-list. CODEOWNERS has no clinical-safety entry.
- **Release.** It never builds the UI image.
- **Pinning.** Actions and base images are pinned by tag, not by SHA or digest, and Dependabot has no `docker` ecosystem.
- **Missing gates.**
  - an MSRV check;
  - a ZAP scan;
  - image scanning;
  - signing and SBOM attestation;
  - `pnpm audit`;
  - coverage;
  - cargo-mutants;
  - CodeQL.
- **Playwright.** It never meets the real API or Keycloak.
- **Compose.** There is no `read_only`, `cap_drop` or `no-new-privileges`. The test Postgres binds `0.0.0.0` with a known password.
- **Test selection.** CI hard-codes test files, so a new non-ignored test file runs nowhere.
- **Stale documents.**
  - The hand-written table in `spec/14` still says "Missing: patient storage".
  - The UI `AGENTS.md` still says patient pages are not built.
  - The `compose.yaml` comment still says break-the-glass is not built.
  - Per-crate changelogs, which the release notes read, stop at 0.1.0.
- **Long task entries.** Several `tasks.md` entries are over 2,000 characters of inline status log.

### 14.3 New capabilities (no task yet)

Ranked by value to this project and by how firm the evidence is.

| Rank | Capability | Why | Depends on |
|---|---|---|---|
| 1 | Medication reconciliation (admit, transfer, discharge) | In 1.0 scope (§7.1) but no task; NHS DAPB4013 medicine and allergy transfer standard | RC-5, CA-1, RC-3 |
| 2 | FHIR `AuditEvent` with IHE BALP, and IHE PDQm/PIXm | Decided in §4 but no task; audit stays required in the UK and under HIPAA whatever HTI-5 does | AU-1, AU-2, ID-6 |
| 3 | Passkeys / phishing-resistant MFA, fast user switching | §6.4 promises passkeys; NIST SP 800-63-4 asks AAL2 to offer a phishing-resistant option **[uncertain wording]** | SE-9, SE-7 |
| 4 | SDC adaptive forms with `$populate` / `$extract` | SDC STU4 published 2025; makes DO-4 concrete and is a prerequisite for DTR | DO-4 |
| 5 | Ambient AI scribe governance (capture consent, transcript-beside-draft review, agent Provenance, DCB0160 template) | NHS England guidance on AI ambient scribing; MHRA treats a diagnosis-suggesting scribe as a device **[uncertain: version]** | DO-6, SE-2, AS-1 |
| 6 | AI agent / MCP access boundary (agents as principals, per-tool scopes, audited) | Medplum ships an MCP server; Epic agent roadmap (HIMSS26); ECRI 2026 hazard #1 is AI | SE-1, IO-2, AU-1 |
| 7 | NHS national APIs: PDS FHIR, GP Connect Access Record Structured (STU3 adapter), NRL, BaRS | Named in §3.3, no tasks | ID-2, ID-6, CA-3 |
| 8 | SMART Health Links / Cards for patient-shared summaries at check-in | HL7 SHC&L IG STU1; CMS "kill the clipboard" pledges | IO-5, CA-5, ID-4 |
| 9 | EHDS / European Patient Summary on IPS 2.0 | Regulation (EU) 2025/327; cross-border summary due 2029 **[uncertain: dates]** | IO-5, IO-3 |
| 10 | Post-quantum readiness: crypto inventory and algorithm agility for audit MAC, PASETO, TLS | NCSC timeline: plan by 2028, migrate 2028–2031, done by 2035; long-retention records | AU-1, SE-7 |
| 11 | Patient-generated and device observations, quarantined until reviewed | Home BP and glucose via the portal | RC-6, DO-5, CA-5 |
| 12 | Accessibility conformance evidence for the portal (screen-reader audit, ACR) | HHS Section 504 rule requires WCAG 2.1 AA **[uncertain: deadline moved to 2027]** | CA-5, AS-4 |
| 13 | Da Vinci CRD/DTR/PAS (electronic prior authorisation) | HTI-4 final (g)(31)–(g)(33); CMS-0057-F API by 2027 **[uncertain]**. US market only (OQ-4) | OR-5, DO-4 |
| 14 | Single Patient Record (England) outbound feed | NHS Modernisation Bill (May 2026) **[uncertain: status; no spec yet]**. Watch, then build | IO-4, IO-6 |
| 15 | TEFCA participation via Facilitated FHIR | SOP 2.0 effective March 2026 **[uncertain: final text]**. US only | IO-2, ID-6 |
| 16 | UK EPS FHIR Prescribing API | EPS split into prescribing and dispensing FHIR APIs; §7.2 excludes it from 1.0, so post-1.0 | OR-1, OR-6 |
| 17 | Digital quality measures (CQL on QI-Core) outside the clinical database | CMS moving eCQMs to FHIR dQMs **[uncertain: 2030]**; §7.2 excludes eCQM reporting from 1.0 | IO-7, IO-4 |
| 18 | Offline-first entry spike (client outbox, Provenance-based merge) | OpenMRS O3 ships offline mode; resolves OQ-5 | CA-7, RC-1 |
| 19 | Genomics reporting (FHIR Genomics Reporting STU3) | Post-1.0, low priority | RC-6, OR-4 |
| 20 | FHIR R6 | Still in ballot (ballot5, July 2026). Spec note only: keep storage version-neutral | IO-1, RC-1 |

Already covered, nothing to add: topic-based Subscriptions (IO-6). openEHR:
no new evidence to change the M6 "revisit" decision. HTI-5 (proposed) would
remove 34 of 60 criteria, including CDS, MFA and audit reports; do not drop
those features on its account.

### 14.4 Order of work

1. **Safety defects (§14.1).** CQ-1 to CQ-8, each with its hazard row.
2. **Make the gates real.** Push to a remote and get CI green (CQ-20), fix the hazard-log trigger (CQ-21), and fix test selection (CQ-24).
3. **Service structure.** Typed ids and indexes (CQ-9), `ApiError` and the shared module (CQ-11, CQ-12), then handler-level tests (CQ-16), which the later refactors need as a net.
4. **Capabilities in the order of §14.3.** Fit them into the milestones they depend on.

### 14.5 Sources (this pass)

- data.standards.nhs.uk/published-standards/medicine-and-allergyintolerance-data-transfer
- england.nhs.uk/long-read/guidance-on-the-use-of-ai-enabled-ambient-scribing-products-in-health-and-care-settings
- healthit.gov/resources/hti-4-final-rule-electronic-prescribing-real-time-prescription-benefit-and-electronic-prior-authorization/
- healthit.gov/resources/hti-5-proposed-rule-chart/
- hl7.org/fhir/uv/sdc/en/changes.html
- hl7.org/fhir/uv/smart-health-cards-and-links/STU1/history.html
- build.fhir.org/ig/HL7/fhir-ips/en/changes.html
- hl7.org/fhir/6.0.0-ballot5/
- hl7.org/fhir/uv/genomics-reporting/STU3/
- ncsc.gov.uk/guidance/pqc-migration-timelines
- medplum.com/docs/ai/mcp
- fiercehealthcare.com (HIMSS26 Epic AI roadmap)
- ecqi.healthit.gov/fhir/about
- openmrs.atlassian.net (Offline and Internet Connection FAQs)
- rce.sequoiaproject.org (Facilitated FHIR SOP 2.0 draft)
- Secondary sources only, so treat as **[uncertain]**:
  - corbado.com (NIST 800-63-4 passkeys)
  - accessibilitychecker.org (HHS Section 504 dates)
  - freevacy.com (NHS Modernisation Bill)
  - capminds.com (CMS-0057-F)
  - Wikipedia (EHDS)

## 15. Research pass 2026-10-23: Manchester Triage System (MTS)

Evidence base, stated plainly: one standard web search and one extended search, giving abstracts and secondary
summaries. Not read: the MTS book (Emergency Triage: Manchester Triage Group, 3rd edition), any flowchart, the
licence terms, or any vendor's implementation. Everything here is a pointer to verify (AS-11), and facts that
rest on secondary sources are marked **[uncertain]**.

### 15.1 What MTS is

- A nurse-led emergency department triage method. The nurse picks one of about 50 presenting-complaint
  **flowcharts** (52 or 53 depending on the source **[uncertain]**; 49 are suitable for children), then works down
  its **discriminators** from most to least urgent. The first discriminator that applies sets the category.
- Five categories, with maximum times to first clinical assessment: red *immediate* (0 min), orange *very
  urgent* (10), yellow *urgent* (60), green *standard* (120), blue *non-urgent* (240).
- Six general discriminators recur across flowcharts: life threat, pain (0 to 10), haemorrhage, acuteness of
  onset, level of consciousness and temperature. Each flowchart adds its own.
- The 3rd edition (2014; a 2023 revision is listed as version 3.8 **[uncertain]**) added neonate and baby
  charts, changed how fever in children is prioritised, and clarified terms (for example "new abnormal pulse").
  The paediatric validation papers I found used the 1996 first edition, so their results may not transfer.
- It is a **method with published content**, owned by the Manchester Triage Group and published through the
  Advanced Life Support Group. **I did not find the licence text.** Whether we may encode its flowcharts in an
  open-source repository is therefore unknown, and we must not copy them in until it is settled (OQ-T1).

#### 15.1.1 Licensing findings (TR-1, 2026-10-23)

From search summaries only; the ALSG help-centre page and the ALSG introduction PDF were not opened.

- Electronic use is licensed through **Wiley** on behalf of the Advanced Life Support Group and the Manchester Triage
  Group. A software developer can license the framework and the content, or the framework only, leaving each
  end-user organisation to integrate the content later. A hospital can also license MTS directly and buy software
  already populated with content.
- The group gives licensed vendors a detailed map of each flowchart, so discriminator order is preserved. An
  accreditation scheme covers software design, licensing, audit and timely updates.
- Price signals: a 2025 NHS trust contract with Wiley, £17,500 excl. VAT over five years; an ambulance trust paid
  £12,000 for three years (2019 FOI). Prices are per organisation and **[uncertain]** for us.
- The Royal College of Emergency Medicine reports that the group no longer supports earlier editions, so a
  deployer must run the current version and our packs need an update path.
- A German implementation guideline says two rights holders (Wiley and the German translation publisher) must both
  confirm IT rights. Translation rights may differ by country **[uncertain]**.
- **Not found:** the licence terms, and whether an open-source project may publish the flowcharts at all.

**Interim policy (until Wiley or the group answers):** no MTS flowchart, discriminator or wording in the repository,
in tests or in demo data. The project ships the framework only: the pack interface (TR-6) and a synthetic pack. A
deployer obtains the MTS licence and content from Wiley, and the project does not claim MTS accreditation or
conformance. **Next step for a human:** write to the group (manchestertriage@alsg.org, from the community hub
**[uncertain: confirm the address]**) and to Wiley's licensing team, asking whether the framework-only model suits
an open-source, Apache-2.0 or MIT-licensed project.

### 15.2 Evidence on safety

- Validity studies report that MTS under-triages some groups. The ones I found concern children with congenital
  heart disease, and children whose vital signs are not part of the discriminators. Updating MTS with paediatric
  vital signs improved prioritisation in the cited work **[uncertain: abstracts only]**.
- The consequence for us is that triage is a **clinical decision aid with known error modes**, not a calculation.
  A wrong category changes who is seen first, so every triage item is clinical (AGENTS.md rule 4).

### 15.3 How it fits this project

- Today the scope is ambulatory first, inpatient core next (§7.1). Emergency department triage is a new setting.
  It needs a decision on scope (OQ-T2) before any build.
- It reuses what exists: vital signs as Observations (RC-6) and the early-warning score from the NEWS2 work
  (OR-3), Encounter (RC-1), the work queue (CA-1), audit (AU-1), Provenance, and consent.
- Absent evidence is `null` with a reason. A discriminator that was not assessed is "not assessed", never
  "absent". A missing assessment must not lower the category.
- The nurse stays the decision maker. We record the flowchart, the discriminator that fired, the category and
  who chose it. An override by the nurse is a first-class event with a reason, and the system never changes a
  category silently.
- Time targets are a clock from arrival or triage. Breaches show on the tracking board and are audited. Alert
  fatigue (§5) argues for one breach signal per patient, not a stream.
- FHIR has no single agreed home for a triage category. Candidates are `Encounter.priority`, an `Observation`
  with a coded value, or an extension. **[uncertain]** The mapping needs checking against the UK Core and
  US Core ED profiles before the spec commits (OQ-T3).

### 15.4 Risks specific to MTS

| Risk | Mitigation |
|---|---|
| Copyright on flowcharts | Licence first (TR-1); configuration loaded from a licensed pack, not stored in the repo |
| Under-triage of groups MTS handles badly (children, congenital heart disease) | Show vitals and NEWS2 or PEWS beside the category; a prompt, never an automatic change; hazard rows |
| Stale or mixed flowchart versions | Version every pack, stamp each triage with its pack version |
| Silent re-categorisation | Category changes are append-only events with reason and author |
| A tool that looks like it triages by itself | Wording and UI say "suggested"; the nurse confirms; see MHRA device risk in AS-1 |
| Fixed clocks cause alert fatigue | One breach signal per patient, tested against a click and alert budget |
| Locale | Flowchart text is clinical content; translations stay machine-written until a speaker reviews them |

### 15.5 Open questions (to move to spec/16-open-questions.md)

- **OQ-T1.** May the project encode MTS flowcharts and discriminators, and under what licence?
- **OQ-T2.** Is emergency triage in scope for 1.0, or post-1.0 (§7.2)? Recommendation: post-1.0, with the data
  model kept compatible now.
- **OQ-T3.** Which FHIR element carries the triage category, and which code system?
- **OQ-T4.** Do we support other triage systems (ESI, CTAS, ATS, NEWS2-led pathways), through the same
  pluggable pack interface? Recommendation: design the interface for more than one, build only MTS.

### 15.6 Order of work

1. Settle the licence and scope (TR-1, TR-2). Nothing else is safe to build before these.
2. Model and hazard rows (TR-3, TR-4), then the triage record (TR-5).
3. Pack loader (TR-6), then the UI (TR-7), tracking board and clock (TR-8).
4. Paediatric and vitals prompts (TR-9), audit of triage accuracy (TR-10), interoperability (TR-11), tests (TR-12).

### 15.7 Sources (this pass)

All are abstracts or summaries, so treat the details as **[uncertain]**.

- wiley.com: Emergency Triage: Manchester Triage Group, 3rd edition (version 3.8, 2023)
- pubmed.ncbi.nlm.nih.gov/42651270 (MTS effectiveness, limitations and global implementation, narrative review)
- ncbi.nlm.nih.gov/pmc/articles/PMC2548283 (paediatric prospective study; first edition)
- ncbi.nlm.nih.gov/pmc/articles/PMC3893080 (improving MTS for paediatric emergency care)
- ncbi.nlm.nih.gov/pmc/articles/PMC7872278 (updating MTS with vital signs)
- ncbi.nlm.nih.gov/pmc/articles/PMC9546792 (MTS safety for children with congenital heart disease)
- ncbi.nlm.nih.gov/pmc/articles/PMC5289484 (validity of MTS, prospective study)
- ncbi.nlm.nih.gov/pmc/articles/PMC5016055 (main flowcharts and discriminators in paediatric care)
- metricgate.com/docs/manchester-triage-priority (secondary)


## 16. Research pass 2026-10-23: Wong-Baker FACES® Pain Rating Scale

Evidence base, stated plainly: two standard web searches and two page fetches of wongbakerfaces.org, giving
search summaries and a landing page. Not read: the Foundation's Instructions for Use, FAQ or Terms of Use pages
(the fetched page only links to them), the Garra 2010 full text, or any licence agreement. Facts that rest on
summaries are marked **[uncertain]** and go to AS-11 for verification.

### 16.1 What it is

- A self-report scale of six cartoon faces, scored 0, 2, 4, 6, 8, 10, for physical pain intensity. The Foundation
  says it is for people aged 3 and older, not only children.
- It is **self-report only**. The Foundation says it is for physical pain, not emotion or comfort, and must not be
  used with unresponsive patients. Observer scales (FLACC, CPOT, PAINAD) are a separate instrument and a separate
  record.
- Validation: Garra et al. 2010 compared it with a 100 mm visual analogue scale in children aged 8 to 17 in a
  paediatric emergency department; mean VAS rose by about 17 mm per face **[uncertain: abstract only]**. Some
  versions, such as Hicks et al., score 0 to 5 **[uncertain]**, so the stored score must name the instrument.

### 16.2 Licence and content policy (the same shape as OQ-T1)

- The artwork is copyrighted. The Foundation offers a free personal-use agreement for individuals only, and a paid
  licence for organisations. An institutional licence reportedly covers use in electronic medical records and gives
  digital files and translations. Fees are quoted on request, and I found no price list.
- Rules stated on the Foundation pages: no modification without written permission, and adding colour counts as
  infringement. Publishing and research use must contact the Foundation first. Citation text is given on the site.
- For us: an Apache- or MIT-style open-source repo cannot ship the faces without a licence. **The repo must hold
  no face artwork until OQ-P1 is settled.** Without it, the 0 to 10 numeric rating scale (NRS) works with no
  artwork and no licence, so the NRS ships first and the FACES widget is a licensed, optional add-on.

### 16.3 How it fits this project

- It is a vital sign: an `Observation` (RC-6). Candidate LOINC codes **[uncertain: from a search summary, verify
  against loinc.org]**: 72514-3 *Pain severity, 0-10 verbal numeric rating [Score], Reported*, and 38221-8 *Pain
  severity, Wong-Baker FACES pain rating scale*. LOINC 56840-2 is discouraged in favour of 72514-3. UCUM unit is
  `{score}`.
- Two numbers must not be mixed. A FACES 6 is not a NRS 6. Store the **instrument** with the value, show it, and
  never plot or compare across instruments as one series without saying so. Trend charts split by instrument.
- Absent evidence is `null` with a reason: "patient unable to self-report", "declined", "not asked", "asleep" are
  distinct `dataAbsentReason`s. An unassessed pain is never recorded as 0 ("no hurt"), and a missing score never
  counts as low pain (matches the triage rule in §15.3).
- Pain is a Manchester discriminator (§15.1). TR-5 and TR-6 should take pain from this observation, not from a
  second field, and TR-9 may prompt on a rising score. A prompt, never an automatic change.
- Reassessment matters clinically: pain after an analgesic is a *second* observation linked to the first, with the
  time of the intervention. That is a flowsheet row (DO-5), not a new design.
- UI: a large-target single choice (radio group with an accessible name per face, not colour or image alone), a
  keyboard path, and a text label under each face. Alt text and labels are clinical content, so translations stay
  machine-written until a speaker reviews them. The CSP forbids inline `style=""`, so any face sizing uses Svelte's
  `style:` directive.
- Clinical risk: a wrong-instrument or wrong-scale entry changes analgesia decisions. Every item is **Clinical**.

### 16.4 Risks specific to this scale

| Risk | Mitigation |
|---|---|
| Copyright on the faces | Licence first (PN-1); NRS ships without artwork; no image in the repo until OQ-P1 |
| Face scores read as NRS scores | Store and show the instrument; separate trend series; a test that refuses a unitless pain value |
| Used on a patient who cannot self-report | Reason-coded absent evidence; offer an observer-scale path (OQ-P3); never default to 0 |
| Mis-keyed or mis-tapped face | Confirm step shows the chosen face and its number; the value is an append-only observation, corrections are new rows |
| Altered artwork (colour, scaling distortion) | Use only licensed files, unmodified; a checksum test on the shipped asset |
| Translation drift | Machine-written until reviewed; never claim reviewed |

### 16.5 Open questions (to move to spec/16-open-questions.md)

- **OQ-P1.** Does the project obtain a Foundation licence, and who holds it (the project, or each deploying
  organisation)? Recommendation: the deploying organisation, with the artwork loaded from a licensed pack like
  OQ-T1.
- **OQ-P2.** Which LOINC codes and value set bind pain observations, and do UK Core and US Core accept them?
- **OQ-P3.** Do we support observer pain scales (FLACC, CPOT, PAINAD) for patients who cannot self-report?
  Recommendation: design the instrument field for it now, build later.

### 16.6 Order of work

PN-1 and PN-2 first (licence, instrument model). Then the observation record and NRS entry (PN-3, PN-4), hazard
rows (PN-5), the FACES widget only if PN-1 allows (PN-6), then trends, reassessment and triage links.

### 16.7 Sources (this pass)

Summaries only, so treat details as **[uncertain]**.

- wongbakerfaces.org (landing page ?p=4; licensing dashboard pages for personal, professional, publishing,
  educator and commercial use)
- rdcoas.c-path.org (COA At-a-Glance: Wong-Baker FACES)
- researchconnect.stonybrook.edu (Garra et al. 2010, validation in paediatric ED patients; PMID 20003121)
- findacode.com and loinc.org (72514-3, 56840-2); VSAC pain intensity value set (38221-8) via tx5.health-samurai.io
- Secondary, low trust: carepatron.com template, metricgate.com

## 17. Research pass 2026-10-23: typical EPR modules

Evidence base, stated plainly: the module list you pasted (six groups) and three standard web searches, which gave
search summaries only. Not read: the NHS Digital Capabilities Framework itself (a source says it needs a login), the
HIMSS EMRAM criteria, the PRSB discharge standard (the search returned nothing on it), or any vendor's module. What is
below is a gap analysis against our own plan and tasks, and facts from the searches are marked **[uncertain]**
for AS-11 to confirm.

### 17.1 What the searches added

- NHS England's developer guidance groups EPR modules as records, assessments and plans, ordering and managing
  medicines, and asset and resource management (staff and beds), and says not every module applies to every care
  setting. The Digital Capabilities Framework (formerly Minimum Digital Foundations) has over 100 core requirements
  and is broadly in line with HIMSS EMRAM stage 5 **[uncertain: secondary source]**.
- Closed-loop medicines administration (barcode check of patient and drug at the bedside) is tied to EMRAM stage 6
  **[uncertain]**. §7.2 already plans eMAR for M7, but no task exists for it.
- Ombudsman findings (SPSO) show missed admission assessments (MUST, falls, sepsis screen) as a recurring harm, which
  supports building them as required, time-based assessments **[uncertain: one case, a secondary summary]**.
- No NHS-published FHIR profile was found for fluid balance, VTE risk or sepsis screening. A third-party guide
  (Savannah GHI) has falls and MUST codes. Neither is a source to build on.
- In 2025 data, 93% of providers had an EPR but only 30% had fully integrated bi-directional data flows
  **[uncertain]**. Integration, not module count, is the weak point, so the module work stays behind the standards
  already chosen (§4).

### 17.2 Coverage against the pasted list

| Module group | Typical content | Where we stand |
|---|---|---|
| Clinical documentation | progress notes, templates | covered: DO-1 to DO-8 |
| Assessments and care plans | falls, skin, nutrition, VTE, sepsis; pathways | care plans CA-2 only; **no nursing assessments** (EM-3) |
| Order comms and results | CPOE, results inbox | covered: OR-1 to OR-4. Specimen labelling is **not** (EM-6) |
| PACS integration | view images from the chart | out of scope for 1.0 with a link via `ImagingStudy`/DICOMweb, but **no task** (EM-7) |
| EPMA | prescribing, CDS | prescribing and CDS covered (OR-1, OR-5, OR-6). **Administration (eMAR) has no task** (EM-2) |
| Theatre / perioperative | WHO checklist, anaesthetic record | not planned (EM-10, post-1.0) |
| Emergency department | board, wait clocks, triage | triage post-1.0 (TR-*, OQ-T2); board is EM-12 |
| Critical care / device feeds | ventilator and monitor data | not planned (EM-11, post-1.0) |
| e-Obs | vitals and NEWS2 | covered: DO-5, RC-11. Fluid balance is **not** (EM-4) |
| Bed management and flow | occupancy, discharges, isolation | basic bed board in CA-1; flow is EM-5 |
| Outpatient | clinic letters, follow-up, virtual visits | scheduling CA-4. **Letters and discharge summary** (EM-8) and **virtual visits** (EM-9) are not covered |
| Patient portal | appointments, results, summaries | covered: CA-5 |
| Handover | ward list, outstanding tasks | not covered (EM-13) |

### 17.3 Ranking of new items

| Rank | Item | Why | Depends on |
|---|---|---|---|
| 1 | EM-1 module map and scope decisions | Everything else is a decision about scope first | none |
| 2 | EM-2 eMAR (closed-loop administration) | In scope for M7 yet untasked; highest wrong-drug and wrong-patient risk | OR-1, RC-3, RC-5 |
| 3 | EM-3 nursing assessments | Recurring omission harm; needs due-time rules and tested scoring | DO-7, RC-6, CA-2 |
| 4 | EM-8 discharge summary and clinic letters | Highest-volume transfer of care document | CA-1, RC-10 |
| 5 | EM-4 fluid balance | Simple, structured, safety relevant | RC-6, DO-5 |
| 6 | EM-5 bed flow and EM-13 handover | Operational value; clinical when it hides a patient | CA-1, CA-2 |
| 7 | EM-6 specimen labelling | Wrong-blood-in-tube is a never event | OR-1, OR-4 |
| 8 | EM-7 imaging link, EM-9 virtual visits | Link out only; no image storage | OR-4, CA-4 |
| 9 | EM-14 DCF and EMRAM mapping | Evidence for buyers | EM-1 |
| 10 | EM-10 perioperative, EM-11 critical care, EM-12 ED board, EM-15 specialties | Post-1.0, named so nobody assumes otherwise | see tasks |

### 17.4 Risks specific to modules

- A module that is only a form still changes what a clinician sees. Every EM item is **Clinical** unless it says
  otherwise, and its hazard rows come before its code (EM-16).
- Device and PACS feeds make the record a medical device input (OQ-6). They stay post-1.0 and behind a flag.
- Scores such as Waterlow, MUST and VTE risk are copyrighted or licensed in places **[uncertain]**; check before any
  tool text enters the repo, as for MTS (OQ-T1) and FACES (OQ-P1).
- The EM items are many. Building them all would be a second product, so EM-1 decides which stay in 1.0, M7 or post-1.0.

### 17.5 Open questions (to move to spec/16-open-questions.md)

- **OQ-M1.** Which module groups are in the 1.0 claim, and which are post-1.0 (EM-1)? Recommendation: eMAR, assessments
  and discharge summary in M7, the rest post-1.0.
- **OQ-M2.** Which risk tools (Waterlow, Braden, MUST, falls, VTE) may we encode, and under what licence?
- **OQ-M3.** Do we build closed-loop barcode scanning ourselves or integrate a wristband and drug-barcode service?

### 17.6 Order of work

EM-1 first, then EM-16 (hazard rows for what EM-1 keeps), then EM-2, EM-3 and EM-8 in that order. Operational and
post-1.0 items wait.

### 17.7 Sources (this pass)

Summaries only, so treat details as **[uncertain]**.

- [NHS England developer guidance: acute, community and mental health care](https://nhsd-proxy.openprescribing.net/developer/guides-and-documentation/building-healthcare-software/acute-community-and-mental-health-care)
- [HTN: NHS England on digital by default, EPR adoption and optimisation](https://htn.co.uk/2026/04/07/nhs-england-on-digital-by-default-epr-adoption-optimisation-improving-workflows/)
- [The Access Group: HSC and the NHS DCF](https://www.theaccessgroup.com/en-gb/blog/hsc-nhs-dcf/) (secondary)
- [Digital Health: Gloucestershire barcode medication verification](https://digitalhealth.net/2024/09/gloucestershire-hospitals-utilises-barcode-tech-to-verify-medication)
- [SPSO decision report 201803102](https://www.spso.org.uk/decision-reports/2019/july/decision-report-201803102-201803102)
- [Savannah GHI FHIR profile guide](https://build.fhir.org/ig/savannahghi/sil_fhir_profile_ig/CodeSystem-clinical-score-codesystem.html) (third party)

## 18. Research pass 2026-10-23: the Patient Administration System (PAS) as a neighbour

Evidence base, stated plainly: I read `~/git/patient-administration-system/patient-administration-system`: its
`AGENTS.md`, `spec/02-scope.md`, the capability and task lists in `tasks.md`, `plan.md` sections 7 to 12, and a few
lines of `spec/08` and `spec/09`. Not read: its source code, tests, migrations or governance files, so everything below
is what its documents claim, not something I checked running. Nothing in that repository was changed.

### 18.1 What the PAS is

An NHS Wales **administrative, not clinical** system (Axum, SeaORM, Postgres, a SvelteKit UI, 10 locales). It owns
appointments, slots and schedules, waiting lists and RTT clocks, referrals, reminders and replies, letters, transport
and virtual-visit requests, and the measurement of fairness. It holds only opaque `person_id` and `worker_id`; identity
and workforce live in two other services (`person-service`, `worker-service`). It has a transactional outbox (HL7 v2 and
webhooks), FHIR **R5** at its edge (R4 is its NEW-1, not built), and it already records an externally assigned triage
category (NEW-22 to NEW-24) and a pictorial pain preference (NEW-26).

### 18.2 The decision that comes first: boundary

Our CA-1 (ADT), CA-3 (referrals), CA-4 (scheduling and reminders), CA-5 (portal) and OR-7 overlap the PAS scope.
Building both would be two systems that disagree about when a patient is coming. Options:

1. **EPR defers to the PAS** for appointments, waiting lists, RTT, reminders and letters, and keeps the clinical context
   (encounter, orders, notes). The EPR calls the PAS over FHIR and consumes its events. Least duplication.
2. **EPR keeps a minimal scheduler** for standalone use (the project is open source and must run without the PAS) and
   treats the PAS as an optional peer behind the same interface.
3. Merge them. Not recommended: the PAS is deliberately separate from clinical data, which is why its privacy,
   analytics and consent rules are lighter.

Recommendation: 2, with an interface (a scheduling port) that the PAS implements. CA-4 shrinks to the port and a basic
adapter, and PA-1 records the decision.

### 18.3 Ideas worth taking, assessed

| Rank | Idea (PAS source) | Value to EPR | Cost and caveat |
|---|---|---|---|
| 1 | **Support-only rule with a schema guard test** (NFR-11, `cap10_support_test.rs`): no score may reduce care; columns named `score`, `rank`, `deprioritise` and so on fail a test | Directly guards every EPR score (NEWS2 prompts, triage, future risk tools) against becoming a gate | Small. A name-based guard is easy to evade, so it is a tripwire, not a proof |
| 2 | **Gated features: switch plus named approvers plus shadow mode** (no-show score) | Fits DO-8, SE-11 and any AI feature; ties to AS-1 and the CSO rule | Medium; needs an approvals table |
| 3 | **Proxy authority with scopes, expiry and revocation** (NEW-7) | Concrete model for CA-5 proxy access and adolescent rules | Clinical: decides what a carer sees |
| 4 | **Reasonable adjustments and communication needs as coded records with review dates** (NEW-5, 6, 20) | Belongs in the patient banner and portal; interpreter need affects care | Clinical; its 19-code catalogue needs a licence check |
| 5 | **Privacy-safe analytics**: k at least 5, secondary suppression, differencing budget | Needed by OR-7, TR-10, IO-7 and IO-14, none of which has a small-cell rule | Its own doc admits what it does not prevent |
| 6 | **Single-use hashed tokens, GET reads and POST acts** | A tested pattern for IO-10 SMART Health Links and portal links | Small |
| 7 | **Hostile-input test over every route** (chaos testing, seeded) | We have a route census (CQ-16) but no hostile-input sweep | Medium; do it on a throwaway database |
| 8 | **Consent and quiet hours at one choke point** for outbound messages | Any EPR notification (escalations, portal messages) should pass one gate | Small |
| 9 | **Referral triage outcomes** (returned with advice, redirected) | Extends CA-3 beyond accepted or rejected | Clinical |
| 10 | **Clinical priority with a review-by date; follow-up target dates; delayed-follow-up report** (NEW-15, 17) | Clinical context that the EPR should show, not own | Display and event consumption only, since the PAS owns it |
| 11 | **PIFU episodes** (NEW-3) | A clinic outcome the clinician sets in the EPR | Clinical |
| 12 | **Group consultations** (slot capacity, NEW-18) | Only matters if EPR keeps a scheduler | Skip unless option 2 |
| 13 | **Tolerances in red, amber, green** (TOL-1) | Useful for operational metrics (CQ-17 `/metrics`) | Skip for now |

### 18.4 Integration seams (where the two meet)

- **Identity.** The PAS keys everything by a `person_id` from `person-service`. Our matcher and MPI would be one
  source of that person, so a link table and a merge-event rule are needed (a merge must reach the PAS).
- **Arrival and encounter.** A PAS check-in (arrived, room) should open or update an EPR `Encounter`, and an emergency
  encounter is where its triage category already attaches.
- **Events.** The PAS outbox (appointment and slot changes, `SlotReleased`) can feed EPR tasks and worklists (CA-2, CA-6).
- **FHIR versions.** The PAS speaks R5 and we speak R4 (§4). Either side needs an edge: its NEW-1, or an R4/R5 mapping
  on ours. This is the largest unknown.
- **Triage and pain.** The PAS found SNOMED CT UK acuity concepts for the triage category (its `spec/09`, FR-189),
  where our `spec/triage/index.md` says we found no code. One of the two is wrong or they mean different things;
  reconcile before TR-3 relies on either **[uncertain: not checked against a primary source]**.
- **Governance.** Its clinical-safety, DPIA and retention documents are a model for ours, and its list of what is left
  ("mostly people and contracts") matches ours.

### 18.5 Risks

- A split of scheduling across two systems can show a clinician an appointment the other system has cancelled. Events
  must be at-least-once and idempotent, and the EPR must show the age of what it displays.
- The PAS is administrative on purpose. Importing its rules (looser analytics, no clinical data) into a clinical
  record would be wrong, and importing clinical detail into its messages would break its NFR-13.
- Its Welsh context (RTT rules, DHCW, NHS Wales App) does not transfer to England, and we should not copy it.
- Two repositories under one author can drift. Pin the contract (an OpenAPI and a FHIR profile) and test against it.

### 18.6 Open questions (to move to spec/16-open-questions.md)

- **OQ-PA1.** Does the EPR defer to the PAS, keep a minimal scheduler behind a port, or neither (§18.2)?
- **OQ-PA2.** Which system owns the person identity, and how does a merge propagate?
- **OQ-PA3.** FHIR R4 or R5 at the boundary, and who builds the edge?
- **OQ-PA4.** Do the SNOMED acuity codes in the PAS exist and mean what it says? (verify, then reconcile with TR-3)

### 18.7 Order of work

PA-1 (the boundary decision) and PA-2 (identity) first. Then the guards that cost little and protect most (PA-3, PA-4),
then the data the EPR should show (PA-5 to PA-7), then the integration (PA-8, PA-9). Optional items wait.

### 18.8 Sources (this pass)

Local files in `~/git/patient-administration-system/patient-administration-system`: `AGENTS.md`, `plan.md`,
`tasks.md`, `spec/02-scope.md`, `spec/08-architecture.md`, `spec/09-api-surface.md`, `spec/06-functional-requirements.md`.

## 19. Research pass 2026-10-24: the emergency care system repository

Evidence base, stated plainly: I read `~/git/emergency-care-system`'s `README.md`, `AGENTS.md`, the start of
`spec/index.md`, the title of every one of its 22 Gherkin features and 171 scenarios, and the full text of about fifteen
scenarios (unknown-patient registration, triage with missing vitals and paediatrics, reassessment alerts, allergy
override, barcode failure, critical troponin correction, queue with time-critical conditions, provider preference,
network loss, discharge against advice, major-incident surge and drill, audit of emergency access, emergency
authentication override, LWBS analysis). Not read: the SvelteKit app (`src/`), the twelve test suites, the GitHub Pages
site, and the remaining scenario bodies. Nothing in that repository was changed.

**How much to trust it.** Its README says all content is synthetic and was generated by Claude for teaching. It is a
US-flavoured planning demo (ESI triage, insurance authorisation, "VIP" status, door-to-balloon), not a clinical source
and not a running system. So every idea below is a prompt to check against a real UK source, marked **[uncertain]**,
and none is copied as a rule. Its numbers (wait times, ratios, percentages) are invented and are not used.

### 19.1 What it is

A spec-first repository: 22 feature files in Given/When/Then are the source of truth, and the same 171 scenarios are
ported one-to-one into twelve test suites (Selenium and Playwright in six languages) beside a demo SvelteKit app with no
backend. The scenarios walk one emergency department visit: registration (walk-in, ambulance, unknown), triage and
reassessment, beds, physician chart, orders, results and critical alerts, medicines and allergy checks, queue and
provider assignment, discharge and follow-up, dashboard and metrics, major incident, cardiac arrest, authentication and
audit.

### 19.2 Overlap with what we already plan

Most of its flow is already ours: registration and matching (ID-*), triage (TR-*), orders and results (OR-*), allergies
(RC-5), notes (DO-*), eMAR (EM-2), bed board (CA-1, EM-5), handover (EM-13), ED board (EM-12), portal and follow-up
(CA-5), audit and break-glass (SE-*). It adds scenario-level detail, and it exposes gaps in our lists.

### 19.3 Ideas worth taking, assessed

| # | Idea | Where it comes from | Verdict |
|---|---|---|---|
| 1 | **Unknown-patient registration** with a placeholder id, "identity verification required", unverified fields and conversion to a verified record | features 01 and 02 | **Take (EC-2).** We have no task for it, and it is a wrong-patient risk both ways: a placeholder merged into the wrong person, or a conscious patient given a new record beside an existing one |
| 2 | **Placeholder names must not count as evidence** | the `UNK` ids | **Take (EC-2).** The same family of fault as the placeholder NHS numbers fixed in CQ-2 |
| 3 | **Corrected and cancelled critical results**: cancel the alert, tell every recipient, remove the red flag, audit the lab error | feature 10 | **Take (EC-3).** OR-4 does not mention corrections |
| 4 | **Acknowledgement and escalation of a critical result** | feature 10 | **Take with EC-3.** Check OR-4 and OR-5 first |
| 5 | **Structured justification for an override** (rationale, risk, monitoring plan, alternatives reviewed) | features 11 and 12 | **Take (EC-4)** as acceptance additions to EM-2, OR-5 and RC-5 |
| 6 | **Manual entry after a failed barcode scan needs a second person and a review** | feature 11 | **Take (EC-4).** Settles part of EM-2 |
| 7 | **Resuscitation status and cardiac arrest call**: activation, timed event log, return of circulation, stop, family present, false alarm | feature 20 | **Take (EC-5).** We have no resuscitation-status field in the banner at all. DNACPR or ReSPECT is the UK form **[uncertain]** |
| 8 | **Capacity and discharge against advice** with witness | feature 15 | **Take (EC-6).** UK law is the Mental Capacity Act, not a US AMA form **[uncertain]** |
| 9 | **Left without being seen** as a recorded outcome | features 18 and 04 | **Take (EC-6).** It is a safety event with a follow-up, not only a metric |
| 10 | **Time-critical conditions** (stroke, heart attack) with a countdown beside the queue | feature 13 | **Take carefully (EC-7).** A countdown is display; auto-reordering the queue by a rule is not (NFR-12) |
| 11 | **System reorders the queue and assigns providers**, including by "VIP" or patient preference | features 13 and 14 | **Reject as written.** A status that moves someone ahead is the failure NFR-12 forbids. Confidentiality flags are a different thing (EC-7) |
| 12 | **Reassessment due** from an hourly check, with standard and urgent alerts | feature 04 | **Take (EC-8)** as the clock for TR-10 and EM-3. The intervals come from the triage pack, not from this repo |
| 13 | **Major-incident mode**: minimal registration, numbered ids, tracking board, family information, surge, stand-down | feature 19 | **Take, post-1.0 (EC-9).** Planned with EM-12 |
| 14 | **Drill mode** that is visibly "NOT A REAL EMERGENCY" with test patients kept apart | feature 19 | **Take earlier (EC-10).** Synthetic and drill records leaking into the real record is a hazard in every environment |
| 15 | **Authentication outage override** (2-hour, read-only, supervisor approved) | feature 21 | **Take the problem, change the answer (EC-11).** Prefer the downtime snapshot (CA-7) to a login bypass |
| 16 | **Post-emergency access review**: necessity, minimum necessary, tell the patient, quality review | feature 22 | **Take (EC-11).** SE has break-glass but no review workflow |
| 17 | **Stale data warning and sync priority** on a mobile chart | feature 07 | **Take (EC-12).** PA-1 already says to show the age of displayed data |
| 18 | **Single-sex room and isolation rules in bed choice** | feature 05 | **Take as a note to EM-5.** UK mixed-sex accommodation rules **[uncertain]** |
| 19 | **Given/When/Then feature files as the source of truth for acceptance** | the repo's design | **Take in part (EC-13).** Keep one feature set; do not port it into twelve suites |
| 20 | **Insurance authorisation, billing status on discharge** | features 08 and 15 | **Skip.** Out of scope (billing) and US-specific |
| 21 | **Monthly LWBS, satisfaction and financial reports** | feature 18 | **Skip as built; reuse the rule.** Aggregates follow PA-7 (small cells) and OR-7 and TR-10 |

### 19.4 Risks specific to this source

- It was written by a model, for teaching. Treating its scenarios as clinical requirements would launder invented
  detail into our spec. Each EC item must name a real UK source before it enters the spec.
- Its triage is ESI. Ours is MTS, with other systems behind a pack interface (OQ-T4). Do not import ESI levels.
- Twelve ports of one scenario set is a large maintenance cost. A change to one scenario touches twelve suites, and the
  repo says so itself. We should not copy that.
- Several scenarios let the system decide order (queue, provider, bed). Each is a place where NFR-12 applies.

### 19.5 Open questions (to move to spec/16-open-questions.md)

- **OQ-E1.** Does the EPR own resuscitation status (DNACPR or ReSPECT) and show it in the banner, or read it from a
  shared care record? Recommendation: own a read-only copy with its source and date, and decide at EM-1.
- **OQ-E2.** How is an unidentified patient's record joined to the real record later: a merge (ID-5) or a link?
  Recommendation: a merge with a reason, never an automatic link.
- **OQ-E3.** Does the ED board (EM-12) need a major-incident mode in 1.0? Recommendation: no, post-1.0.

### 19.6 Order of work

EC-1 (the status note) first, then EC-10 (drill and test-data separation) and EC-2 (unknown patients), which touch
identity, then EC-3 and EC-4 (results and overrides), then EC-5 to EC-8. EC-9 and the optional items wait.

### 19.7 Sources (this pass)

Local files in `~/git/emergency-care-system`: `README.md`, `AGENTS.md`, `spec/index.md`, and the feature files
`01` to `22` in `tests-with-given-when-then-features/` (titles for all, bodies for about fifteen scenarios).

## 20. Research pass 2026-10-28: removing a hazard from the log

The request: only the Clinical Safety Officer (CSO) can remove a hazard, and must put in evidence that it is removed.
The rule, its evidence table and what is enforced today are in [`spec/hazard/index.md`](spec/hazard/index.md).

### 20.1 Assessment

- The repository already enforces half of it: on a pull request, only a login in `reviewers.txt` (empty until a CSO is
  named, OQ-2) can set `accepted` or `closed`, change `reviewed_by`, or touch a row, and no row can be deleted at all.
- "Remove" is better built as **retire**: the row stays with a closing status. DCB0129 vendor guidance (secondary
  sources, not the standard) says closed hazards stay on the log for audit, and the project's own principle is that
  clinical history is append-only. Deleting would also blind the checker, which compares rows by id.
- Five gaps, ranked by risk: (1) the sign-off job runs on pull requests only and work goes straight to `main`, so a close
  can bypass it; (2) `closed` needs no closure evidence; (3) a login is not a signature; (4) evidence paths can vanish
  after closure; (5) one person can be both maintainer and CSO.

### 20.2 Ideas, assessed

| Rank | Idea | Value | Cost and caveat |
|---|---|---|---|
| 1 | **Run the sign-off comparison on every push to `main`**, against the previous commit, and require pull requests for `main` | Closes the bypass | Small to run; branch protection is a repository setting only the owner may change |
| 2 | **Closure columns**: `closed_on`, `closure_evidence`, `closure_note`; `closed` without them fails | Makes "must put in evidence" a rule the machine checks | Small; changes the CSV shape, so the checker, its tests and the generated docs move together |
| 3 | **Signed closing commit**: the commit that closes a hazard must be signed by a key listed for the CSO | A login becomes an identity | Medium; key management and a trust file |
| 4 | **Evidence watch**: fail when a closed hazard's evidence path disappears; list code paths per hazard so a change there asks for review | Stops silent decay | Medium; path lists need upkeep |
| 5 | **Reopen with a reason by anyone**; closing stays CSO-only | Doubt is cheap to record | Small |
| 6 | **Release freeze**: a signed tag per release freezes the log; the safety case cites the tag | A log as the release saw it | Small |
| 7 | **Separate the roles** in the risk plan: the maintainer may not be the CSO for a release | Keeps a second reviewer | A people decision (OQ-2), not code |

### 20.3 Risks and what is uncertain

- I read vendor guides, not the NHS England standard text: closure criteria and the retention period are **[uncertain]**
  (AS-11).
- Item 2 tightens a check that must never be weakened to get green; the existing 35 rows have no closure data, so none is
  closed today and none is affected.
- Nothing here names a CSO or changes any hazard's status.

### 20.4 Order of work

AS-13 (items 1, 2 and 5), then AS-14 (3), then AS-15 (4 and 6). OQ-2 comes first for all of them.

### 20.4a ISO 14971

Added at the request "add iso 14971": a clause-by-clause mapping in `spec/hazard/index.md` and task AS-16. The biggest gap
is that the risk plan has **no acceptability criteria**, so `accepted` has no yardstick; second, the log has no `harm` or
`control_type` field, and likelihood is of the hazard, not of harm. Whether 14971 applies at all depends on the MHRA device
boundary. Clauses are from memory of a paywalled standard and are **[uncertain]** (AS-11).

### 20.5 Sources (this pass)

- presencis.com/regulations/dcb0129/article-hazard-log/ (secondary)
- cdn.presencis.com/regulations/dcb0129/guide/clinical-safety-officer (secondary)
