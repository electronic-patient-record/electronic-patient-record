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
├── electronic-patient-record-service-with-rust/    Axum and `axa` service: record, FHIR facade, audit, consent
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
| Service framework | Axum 0.8 with `axa` 0.12 (features `postgres-sqlx`, `outbox`, `idempotency`, `problem`, `ratelimit`, `cursor-pagination`); no JWT library: tokens are PASETO or Keycloak, verified by `authentication-verifier` |
| Database | PostgreSQL 18 through `sqlx` (`axa`'s `postgres-sqlx` feature), no ORM; migrations are `.sql` files in the service's `migrations/`, with no down files |
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
   ▼  REST + FHIR (PUBLIC_API_BASE_URL, default :1241)
electronic-patient-record-service-with-rust  (Axum / axa)
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

### 20.4b The same rule inside the product

Asked again as "in the epr system". The service has no CSO role, hazard table or route, so this is a design, not a gap in
code. Ranked ideas:

| Rank | Idea | Value | Cost and caveat |
|---|---|---|---|
| 1 | **CSO role with one permission, `hazard:retire`**, checked by the existing route census | Makes "only the CSO" a failing test, not a convention | Small; needs OQ-2 |
| 2 | **Append-only `hazard_events`** with a no-update, no-delete trigger, same pattern as `audit_events` | Retirement can never erase history | Small; proven pattern |
| 3 | **Evidence with a SHA-256 and a 422 when absent** | "Must put in evidence" becomes a rule the API enforces | Small; a hash shows no change, not truth |
| 4 | **Two registers**: manufacturer baseline (read-only for deployers) and the deployer's own | Matches DCB0129 versus DCB0160 | Medium; **[uncertain]** until the standards are read |
| 5 | **Link a hazard to a feature gate** (PA-4) | An open hazard keeps a risky feature off | Medium; makes retirement clinical |
| 6 | **Scheduled re-hash that reopens** a hazard on changed evidence | Stops evidence rot in production | Medium; needs an alert path |
| 7 | **Two named CSOs and a four-eyes rule** | No lock-out, no self-certification | A people decision; hard on small sites |

Order: AS-17 then AS-18; the repository-file work (AS-13 to AS-15) stays first because it protects the log that exists.

### 20.5 Sources (this pass)

- presencis.com/regulations/dcb0129/article-hazard-log/ (secondary)
- cdn.presencis.com/regulations/dcb0129/guide/clinical-safety-officer (secondary)

## 21. Research pass 2026-10-28: FHIR R5

The request "add FHIR R5". The spec is in [`spec/interoperability/index.md`](spec/interoperability/index.md) "FHIR R5".

### 21.1 Assessment

- §4 keeps R4 as the baseline and says to design so a second release can be added as a second facade. That still holds: UK
  Core and US Core are R4 guides, US Core's next target is reported as R6 (secondary source), and R5 adoption figures are
  weakly sourced (one site claims about 10 percent; unverified).
- The reason to add R5 anyway is a neighbour: the patient administration system speaks R5 (§18, OQ-PA3), and R5 has native
  topic-based subscriptions where R4 has only a backport (IO-6).
- **The author's `fhir` crate already has an `r5` feature and `convert::between::<R4, R5>`**, which returns a loss report. That
  makes a derived facade cheap and, more to the point, makes silent loss visible. It fits the project's rule that absent or
  lost evidence must never look like "none".
- I did not read the HL7 R5 pages. The field-level differences in the spec are from memory and marked uncertain.

### 21.2 Ideas, assessed

| Rank | Idea | Value | Cost and caveat |
|---|---|---|---|
| 1 | **Derive R5 from stored R4; fail a clinical response on any reported loss** | One set of converters; no quiet drop of a reaction, range or notice | Small to medium; the loss report may flag harmless differences, so allow-list them, never block-list |
| 2 | **Read-only first** (Patient, AllergyIntolerance, Observation, metadata) | Settles the PAS edge for reads | An R5 write is a separate decision |
| 3 | **Validate with the HL7 validator at `-version 5.0` in the existing job** | Same proof as R4 | Small; the jar already supports it |
| 4 | **State base R5 only, no profile claim** | Honest: no R5 UK Core or US Core found | None |
| 5 | **R5 subscriptions** (IO-6) | Native topics instead of the backport | Medium; after the read facade |
| 6 | **`fhirVersion` content negotiation** | Standard way to ask for a release | Behaviour from memory: check in IO-17 |
| 7 | **Do not serve R5 `AuditEvent` or `Consent` yet** | Avoids the heaviest reshaping | A partner wanting them waits |

### 21.3 Risks

- A second shape of the record is a second place a clinician or partner can see something different: `clinical-impact`.
- A permissive conversion could hide a dropped field; the fail-closed rule and a planted-loss test are the guard.
- Two releases double the validator and census work; keep the R5 route list short.

### 21.4 Order of work

IO-17 (read the specification) then IO-16. OQ-PA3 stays open until IO-16 lands.

### 21.5 Sources (this pass)

- [US Core STU5 guidance](https://hl7.org/fhir/us/core/STU5/guidance.html) (R4-based despite the label)
- [MITRE, FHIR versions](https://mitre.github.io/fhir-for-research/modules/fhir-versions) (secondary: US Core's next target is R6)
- [Medplum on R5](https://www.medplum.com/blog/fhir-r5) (secondary)
- [UK Core R4 package on get-ig.org](https://get-ig.org/fhir.r4.ukcore.stu3.currentbuild) (fhirVersions 4.0.1)
- the `fhir` 4.2.4 crate source: features `r5`, module `convert`


## 22. Research pass 2026-10-30: other open-source EPR and EHR systems

The request: research and improve code quality, maintainability, security and scalability by studying other open-source
EPR and EHR systems, and gather their capabilities, features and configuration. The result is
[`spec/benchmark.md`](spec/benchmark.md) (the field, two matrices, ranked gaps) and the `OS-` tasks. Nothing was built.

Evidence base, stated plainly: twelve repositories were read through the GitHub API (`contents` listings and raw files)
and their documentation sites, not cloned; exact stars and push dates from one API call on 2026-10-30; our own side was
inventoried file by file first. GitHub code search was rate-limited in three of five sweeps, so a few cells are marked
unverified in the spec. Three distributions (Bahmni, Ozone, GNU Health) were read for capabilities only.

### 22.1 Assessment

- **Inside the repository we are at or above the field.** Lint and clippy at `-D warnings`, `#![forbid(unsafe_code)]`
  and `#![deny(missing_docs)]` on the service, zero `unwrap` outside tests, four census tests, a seeded hostile-input
  sweep, a hand-run mutation pass, cargo-deny, Dependabot, provenance attestation and a CycloneDX SBOM on release, an
  audit chain that refuses an unaudited write, break-glass with review, an in-product hazard register. No comparator has
  the census tests, the write refusal or the hazard register; only Blaze signs and attests; nobody ships an SBOM.
- **What we lack is operational.** Of 25 table-stakes items, 7 are missing and 7 partial (`spec/benchmark.md`): no
  coverage or SAST published from CI, no rate limit (the config comments promise one that does not exist), no job
  runner or outbox (promised in `spec/08` and the config), no performance numbers (CQ-18 waits on them), counters but
  no tracing, a CapabilityStatement but no OpenAPI (and `tasks.md` names an `src/openapi.rs` that does not exist), an
  in-process UI session store, English-only strings, no extension mechanism yet, and six stale documents.
- **Functionally we are a scaffold and the field is not.** OpenEMR, OpenMRS, Marley, GNU Health and VistA each ship
  scheduling, encounters, notes, orders, results, medicines and billing. That is the roadmap (M2–M7), not a finding;
  the benchmark takes from them the *configuration* patterns, not the modules.
- **The two Rust comparators are instructive.** Haste (axum, sqlx, 39 crates) runs pedantic clippy, daily cargo-audit,
  Trivy, proptest and a built-in OIDC and SMART server; Helios (axum, 22 crates) publishes llvm-cov coverage, runs
  Inferno, Playwright with axe and a two-node cluster smoke test in CI, and ships Fluent locales. Neither uses
  cargo-deny, nextest or `forbid(unsafe_code)`; we do the first and last.
- **Corrections to our own claims found on the way:** the `audit.rs:25` line in CQ-18 is now `audit.rs:23` (the key)
  and `:431` (the lock); `policy.rs` says nothing calls `evaluate`, but `controllers/common.rs:172` and `adt_feed.rs:162`
  do; `auth.rs:19` says consent and break-glass are not built; `SECURITY.md` says the same and that only compare
  writes an audit row; `deny.toml` cites a `security.yml` workflow that does not exist; `spec/08` lists tantivy, an
  outbox and row-level security that are not in `Cargo.toml` or any migration.

### 22.2 Ideas, assessed

| Rank | Idea | Who does it | Value here | Cost and caveat |
|---|---|---|---|---|
| 1 | **Rate limit and login lockout**, Redis-backed, audited refusals | Medplum `ratelimit.ts`, Haste `crates/rate-limit`, OpenMRS lockout | `$match` and sign-in without a limit are an enumeration risk; the config already promises it | ~2 days (OS-2); the limit must never refuse a clinician mid-task silently: 429 with `Retry-After`, logged |
| 2 | **Outbox and a worker** in the same transaction as the audit row | Haste `crates/worker` + `async_jobs.sql`, Medplum `workers/`, HAPI Batch2 | Unblocks IO-6 subscriptions, CDS Hooks, HL7 v2 out, and `$export`; keeps "events leave through an outbox" true | ~3 days (OS-3); append-only table, delivery recorded, never a delete |
| 3 | **Coverage, SAST, audit from CI** | Helios `cargo-llvm-cov` → codecov; OpenMRS CodeQL; OpenEMR semgrep | Cheapest way to see the untested paths; the field treats it as a floor | ~1 day (OS-4, extends CQ-25); CodeQL's Rust support must be confirmed, else semgrep |
| 4 | **Published performance numbers** in `BENCHMARKS.md`, run on `main` | EHRbase JMeter in every build, Helios `fhir-benchmark.yml`, Blaze `docs/performance.md` | CQ-18 (the audit lock) cannot be decided without a number | ~2 days (OS-5, half of AS-6) |
| 5 | **OTLP tracing and labelled metrics, database-checking readiness** | Helios `crates/observability`, Medplum `otel/`, HAPI starter OTel agent | NFR-8 promises it; three counters cannot find a slow route | ~2 days (OS-6) |
| 6 | **OpenAPI with a freshness gate and a route census** | OpenEMR `api-docs.yml`, Haste `openapi-schema-generator`, Medplum `openapi.ts` | `tasks.md` has claimed `src/openapi.rs` since M0 | ~2 days (OS-7) |
| 7 | **Shared UI session store** | OpenEMR `LockingRedisSessionHandler.php`, Medplum Redis sessions | NFR-3 says stateless app tier; the UI is not | ~1 day (OS-8) |
| 8 | **Documentation census**: a test that every source path named in config, `SECURITY.md`, `deny.toml`, `tasks.md` exists | OpenEMR's freshness gate (idea) | Six stale statements found today; the test keeps it at zero | ~half a day (OS-9) |
| 9 | **Packs as versioned data, loaded idempotently, with typed schemas and `*_enabled` keys** | OpenMRS Initializer and O3 config schemas, HAPI starter keys, Ozone child-distro | The IO-18 and IO-19 mechanism, designed from working examples | design first (OS-10) |
| 10 | **Access policy as data** | Medplum AccessPolicy | The ABAC half of `policy.rs` could become reviewable configuration for clinical governance | post-1.0; the policy table header says it is an engineer's draft (OQ-OS2) |
| 11 | **Secret scanning in CI** | nobody in CI; Metriport at pre-commit | Cheap; we would lead | ~1 hour (OS-11) |
| 12 | **Expiring advisory ignores** | — | 13 `RUSTSEC` ignores with no re-check date is drift waiting to happen | ~1 hour (OS-12); **done 2026-10-30**, and it found 12 of the 13 were dead |
| 13 | **Audit retention and partitioning** | HAPI partitioning, EHRbase `data-retention`, Blaze `job-prune` | The retention schedule is a draft; the table grows forever | design after governance sets periods (OS-13) |
| 14 | **cosign on images** | Blaze | We attest provenance and ship an SBOM; signing adds a verifiable pull | ~half a day, inside CQ-23 |
| 15 | **Inferno (g)(10) in CI** | OpenEMR, Helios, Haste | The conformance oracle for SMART, bulk data and US Core, when those exist | after IO-2 and IO-4 |
| 16 | **Pedantic clippy** | Haste | Marginal over `-D warnings` with `missing_docs`; many allows | skip for now |
| 17 | **Fuzz target** for the HL7 v2 parser and the matcher | Haste proptest, Blaze test.check | Already in CQ-30 and `spec/11` | keep there |
| 18 | **A code of conduct** | EHRbase, OpenEMR | Governance hygiene; most of the field lacks it too | ~1 hour (OS-14) |

Rejected: copying any comparator's clinical modules (the roadmap already orders them); a plugin API in the service
(packs are data, not code, by the equity rule); Elasticsearch as a search backend (Postgres suffices until measured).

### 22.3 Risks

- Rate limiting and lockout can deny a clinician at the bedside. A limit on sign-in must favour a slow refusal over a
  lockout, say so in the message, and be in the hazard log before it ships (`clinical-impact`).
- An outbox that delivers a subscription without the consent and policy check of the read path is a disclosure. The
  worker must evaluate the same `policy::evaluate` as a read.
- Coverage numbers invite gaming; the gate should be a floor that cannot drop, not a target to chase.
- Benchmarks on a laptop are not a capacity claim; `BENCHMARKS.md` records the machine and the commit.

### 22.4 Open questions (to move to spec/16-open-questions.md)

- **OQ-OS1.** Does a request limit apply to clinicians at all, or only to unauthenticated and integrator traffic?
  Recommendation: per-subject limits high enough never to trip in clinical use, strict on sign-in and `$match`.
- **OQ-OS2.** Should the ABAC policy become data (Medplum-style) reviewable by clinical governance, or stay code with
  the census tests? Recommendation: stay code until a Clinical Safety Officer asks to review it.
- **OQ-OS3.** Is CodeQL's Rust analysis acceptable as the SAST gate, or semgrep? Recommendation: CodeQL for the UI now,
  Rust when its support is confirmed, semgrep meanwhile.

### 22.5 Order of work

OS-9 (the documentation census and the six fixes) first, because the stale statements mislead every later reader;
then OS-4 (coverage and SAST, a day); then OS-2 (rate limit, with a hazard row) and OS-3 (outbox and worker); OS-5 and
OS-6 together (numbers and tracing, which CQ-18 needs); OS-7 (OpenAPI); OS-8; OS-10 as the design step for IO-18.
OS-11 to OS-14 whenever a day has an hour spare.

### 22.6 Sources (this pass)

Repositories, read through the API on 2026-10-30: [openemr/openemr](https://github.com/openemr/openemr),
[medplum/medplum](https://github.com/medplum/medplum), [hapifhir/hapi-fhir](https://github.com/hapifhir/hapi-fhir) and
[hapi-fhir-jpaserver-starter](https://github.com/hapifhir/hapi-fhir-jpaserver-starter),
[openmrs/openmrs-core](https://github.com/openmrs/openmrs-core), [samply/blaze](https://github.com/samply/blaze),
[ehrbase/ehrbase](https://github.com/ehrbase/ehrbase), [metriport/metriport](https://github.com/metriport/metriport),
[hastehealth/hastehealth](https://github.com/hastehealth/hastehealth), [HeliosSoftware/hfs](https://github.com/HeliosSoftware/hfs),
[earthians/marley](https://github.com/earthians/marley), [WorldVistA/VistA](https://github.com/WorldVistA/VistA),
[HospitalRun/hospitalrun-frontend](https://github.com/HospitalRun/hospitalrun-frontend) (archived).
Documentation: [Medplum access policies](https://www.medplum.com/docs/access/access-policies) and
[bots](https://www.medplum.com/docs/bots); [OpenMRS Initializer](https://github.com/mekomsolutions/openmrs-module-initializer/tree/master/readme)
and [O3 configuration system](https://o3-docs.vercel.app/docs/configuration-system);
[OpenEMR features](https://www.open-emr.org/wiki/index.php/OpenEMR_Features) and
[emergency access](https://www.open-emr.org/wiki/index.php/6._Emergency_Access_Procedure);
[Ozone distributions](https://docs.ozone-his.com/implementers/create-distro/);
[GNU Health features](https://docs.gnuhealth.org/his/features.html); [Frappe DocTypes](https://docs.frappe.io/framework/user/en/basics/doctypes);
[EHRbase](https://github.com/ehrbase/ehrbase) `application.yml`; HAPI starter `application.yaml`.
Secondary (capabilities only): Purkayastha et al. 2019, *Healthcare Informatics Research* 25(2), comparison of
open-source EHRs ([PMC6517630](https://pmc.ncbi.nlm.nih.gov/articles/PMC6517630)).


## 23. Research pass 2026-11-02: Openfront Hospital, read through the PAS comparison

The request: read the Patient Administration System's comparison of Openfront Hospital and add its ideas here. The source
is that repository's `spec/compare/openfront-hospital/index.md` (researched 2026-11-02 from the public repository
<https://github.com/openshiporg/openfront-hospital>: README, schema map, model and mutation file lists, three model files).
**I read the comparison, not the repository**, so every claim about Openfront below is second-hand, and the comparison
itself says nothing was run. It was written for the PAS, where the gaps differ from ours; each idea below was
re-checked against this repository's code before it was ranked. Nothing is built by this pass except where a task says so.

### 23.1 Assessment

- **Two of the comparison's three headline gaps are not gaps here, or only partly.** Every REST update in this service
  already carries `expected_version` and answers 409 with `current_version`; the FHIR update takes `If-Match`. Patient
  search already writes an audit row (which criteria kinds were used, how many results, how many hidden by consent, never
  the values). What is missing is narrower: **a retried create has no idempotency key** (two clicks, or a client retry
  after a timeout, can write two allergies), and **not every search and list route is proven audited** (the census walks
  the 15 patient-scoped reads, not the FHIR searches or `$match`).
- **Records governance is a real gap.** There is no hold that blocks archiving or disposal, no recorded retention
  decision, and the audit export is a synchronous download, not an expiring job. OS-13 archives and never deletes, so
  a hold is cheap to honour now and expensive to retrofit.
- **Wards, beds and the front desk belong to the PAS side of the boundary** (`spec/02-scope.md`, OQ-PA1). Timed bed
  reservation, an occupancy event log and an arrival queue are design notes for CA-1 and PA-8, not new EPR features.
- **Per-area permission flags and a first-administrator bootstrap do not fit.** Sign-in and roles are Keycloak's, and the
  policy matrix (`spec/access-control`) already separates areas by role. We have no password accounts to bootstrap.
- **A schema map is cheap and keeps itself true.** One generated table of every table, its migration and its columns,
  checked in CI like the implementation status.
- **Do not copy:** clinical lists, claims and payments, a second patient master, anonymous self-booking. The comparison
  reaches the same view.

### 23.2 Ideas, assessed

| Rank | Idea | Source | Value here | Cost and caveat |
|---|---|---|---|---|
| 1 | **Idempotency key on creates** (`Idempotency-Key`, replay the stored answer, refuse a changed body) | Openfront `operationKey` | A duplicate allergy or consent is a clinical-record error, and retries are normal | ~2 days (OF-1); clinical, hazard row; the key store needs an expiry and must not hold more than the response already shows |
| 2 | **Prove every search and list is audited** | Openfront `PhiReadAudit` | Closes the "searches are not audited" worry for the routes not yet walked | ~1 day (OF-2); a digest of the criteria is not proposed: a hash of a date of birth is easy to reverse |
| 3 | **Record hold and a retention decision record** | Openfront `RecordHold`, `RetentionDisposition` | Archiving (OS-13) is not yet a decision anyone has recorded; a hold must stop it | ~4 days (OF-3); clinical; needs OQ-OF1 answered first |
| 4 | **Schema map generated from the migrations, checked in CI** | Openfront `docs/schema-map.md` | New readers and the data-protection assessment need one list of what is stored | ~1 day (OF-6); built with this pass |
| 5 | **Expiring, encrypted export jobs** | Openfront `ExportJob` | The 10000-row cap makes a full audit export a loop of files | ~3 days (OF-4); needs OQ-OF2 (where the key lives) |
| 6 | **Timed bed reservation, occupancy event log, arrival queue** | Openfront `BedReservation`, `BedOccupancyEvent`, `CareQueueTicket` | Design input for CA-1 and PA-8 | notes only (OF-5); built on the PAS side |
| 7 | Per-area permission flags; once-only first-admin bootstrap | Openfront capability flags | None while Keycloak holds identity | not adopted (OF-7, a record of the decision) |

### 23.3 Risks and what is uncertain

- The comparison's "not found" statements come from a text search in the PAS repository; ours come from this one, read for
  this pass (the `expected_version` routes, the `Search` audit detail and the `FHIR If-Match`), not from a full audit.
- An idempotency store is a second place a response is kept. If the response holds patient data, the store holds it
  too, so it needs the same retention and the same erasure rules as the record.
- A legal hold changes what a clinician can no longer see only if archiving hides rows; it does not (the
  `audit_events_all` view). It still changes what an operator may do, so it is audited and needs a role.
- Openfront is early (about 30 commits, one star as of the comparison) and mostly untested in its repository. It is a
  design reference.

### 23.4 Open questions (to move to spec/16-open-questions.md)

- **OQ-OF1.** Who may place and lift a record hold, and does a hold also stop an archive that only moves rows?
  Recommendation: a named governance role, never the clinician role; a hold stops disposal, and stops archiving too
  until governance says archive-only is acceptable.
- **OQ-OF2.** Where does the key for an encrypted export live, and how long does a job live? Recommendation: a key held
  outside the database, a job that expires in 24 hours, and no export without an audit row.

### 23.5 Order of work

OF-6 first (built with this pass, no risk); then OF-2 (a census, no behaviour change); then OF-1 (hazard row first);
OF-3 after OQ-OF1; OF-4 after OQ-OF2. OF-5 and OF-7 are notes that need no schedule.

### 23.6 Sources (this pass)

The PAS repository's `spec/compare/openfront-hospital/index.md` (read in full, 2026-11-02); through it, the repository
<https://github.com/openshiporg/openfront-hospital> (not opened by me). This repository: `src/controllers/patients.rs`
(`search_service`), `src/controllers/*.rs` (`expected_version`), `tests/audit_coverage.rs`, `migration/src/`.


## 24. Research pass 2026-11-02: the Care Coordination and 360 Referrals guide

The request: research <https://kevinmayfield.github.io/PatientCareCoordination/en/index.html>, assess it, advise, and add
the ideas to `plan.md`, `tasks.md` and `spec/*`. Nothing was built. The draft state model is in
[`spec/interoperability/closed-loop-referral.md`](spec/interoperability/closed-loop-referral.md).

**Evidence base, stated plainly.** I read seven pages of the guide through a page-to-text fetch that answers a prompt, not the
raw pages: the home page, closed-loop referral (360X), the order-and-report workflow pattern, the NHS England services
assessment, patient flags, virtual wards and dynamic care planning. I did **not** read its other pages (the referral
forms, outcome reports, EU Health Data API, directory, patient identity, transport and data-model pages, the BaRS and
diagnostics appendices), its source repository, or the IHE and HL7 specifications it builds on, so everything about 360X, QEDm, MHD
and DCP is the guide's own account. The fetch tool summarises, so a quotation below is the summary's, not checked against the page.

### 24.1 What it is

An HL7 FHIR R4 Implementation Guide, version 0.1.0, "Draft as of 2026-11-02", built with the FHIR build tools, **copyright
Kevin Mayfield (one named author)**, **no licence stated**, under an example domain (`fhir.acmeexample.org.uk`). It calls itself
"a proof of concept", "demonstration only", "not an authoritative source", "not endorsed by NHS England, IHE or HL7", and says
it "must not be used for implementation or clinical purposes". Its subject is the **closed-loop referral**: a clinician
refers, the recipient accepts or declines, delivers care, and returns a result "so the referrer knows what happened". It adapts
IHE 360X to England, proposes one **order-out-and-report-back** pattern that also covers lab tests, imaging and prescriptions,
and assesses eRS, BaRS, Transfer of Care, MESH, GP Connect, IM1 and EPS against it, recording their gaps. It adds a draft patient-flag
pattern, virtual wards (hospital at home) and dynamic care planning (a shared, versioned care plan).

### 24.2 Assessment

- **A design reference, not a dependency, and nothing to copy.** One author, a draft a day old, no licence, and a stated ban on clinical use.
  Its value is the questions it asks and the gaps it finds, which we can check against our own plan.
- **It fills a thin spot in our plan.** CA-3 is two lines ("`ServiceRequest` plus `Task`, with status tracked until the outcome
  is received") and PA-14 adds redirects and returns. The guide gives a concrete state table, an identifier rule and a FHIR
  mapping to test ours against (24.4).
- **Its sharpest finding is ours too.** The commonest gap in the national services is an outcome that does not say which referral
  it answers. We have no written rule for an outcome that cannot be matched to its request (OR-4 routes results to the ordering clinician; it says nothing about an unmatched one), so PC-2 proposes one: show it to a person, do not file it.
- **Where we are ahead.** It has no authorisation, consent, break-the-glass or tamper-evident audit design beyond a mention of
  audit events; no timeouts, retries, acknowledgements or escalation ("the source has no section on timeouts"); identifiers
  that are typed (system and value) only in the sense of NHS Number plus a local MRN, where BaRS leaves a patient with no NHS Number
  an untyped local identifier. We have typed identifiers, `expected_version` on every update, an outbox, and a policy engine.
- **Where it is England-shaped, and so a pack, not core.** eRS and its UBRN, BaRS, MESH, Kettering XML, GP Connect, IM1, EPS and
  CP-IS are NHS England services. `spec/02-scope.md`'s equity rule puts them in a pack; the core model carries a neutral
  "referral identifier" and "flag".
- **Its own doubts are the useful part.** It marks the very states we would want (`information-requested`, `deferred`,
  `redirected`) and who raises a redirected referral as "To confirm", and whether a flag should be checked before a scheduled referral
  as "Open". We copy that habit: our spec marks a proposal a proposal.

### 24.3 Overlap with what we already plan

| The guide | Ours | Gap |
|---|---|---|
| Referral request, accept or decline, outcome | CA-3, PA-14 | no state table, no identifier rule |
| Order-out-and-report-back for lab, imaging, medicines | OR-1 CPOE, OR-4 results inbox and closed loop, EM-2 | three closed loops designed apart |
| Care plan shared by several providers | CA-2 | no rule for who may change which part |
| Reasonable adjustments | PA-6 | no flag that points to a plan someone else owns |
| Interim note, repeatable | none | add to CA-3 |
| Subscription and event notice for status | IO-6, OS-3 (outbox) | not tied to a referral's status |
| Hospital discharge report as the outcome | CA-1 discharge summary | not aligned to a published document shape |
| Virtual ward | none | post-1.0, a design note |

### 24.4 Ideas, assessed

| Rank | Idea | Where the guide has it | Value here | Cost and caveat |
|---|---|---|---|---|
| 1 | **An explicit referral state table, refused transitions, one stored status** | 360X page: nine states, three provisional | CA-3 is under-specified; the guide faults BaRS for inferring status from several resources' statuses | ~2 days (PC-1); a pure function with tests first, no database; the three provisional states stay "proposed" |
| 2 | **One referral id on every message and report; an outcome with no referral goes to an unmatched queue** | "The referral identifier closes the loop"; "most often missing from NHS letters today" | Closes the commonest real gap; reuses the shape of the feed-refusal queue | ~3 days (PC-2); clinical (a lost outcome) |
| 3 | **Timeouts and escalation: no response, accepted with no outcome, appointment passed with no outcome** | none: the guide says it has no such rule | CA-3 already asks for "N days"; the guide shows nobody else defined it | ~2 days (PC-3), with a fake clock; clinical; needs OQ-PC3 (the periods) |
| 4 | **A cancellation is a request, shown as such** | PCC-58: the recipient "may not honour it" | A clinician who believes a referral is cancelled while the service still sees the patient is a real hazard | ~1 day (PC-4); clinical; hazard row first |
| 5 | **One order-and-report workflow for referral, test, imaging and medicine** | the workflow-pattern page: same steps, same `basedOn` link | OR-4, CA-3 and EM-2 would otherwise be three loops with three bugs | a design decision (PC-5); no code until OR-1 and CA-3 are scheduled |
| 6 | **Patient flags that point to a plan, not copy it; reading one notifies the owner** | patient-flags page | Safeguarding detail must not spread into referrals and letters; PA-6 needs somewhere to live | ~3 days for a design (PC-7); clinical and the most sensitive item here |
| 7 | **A shared care plan where each contributor changes only their own parts and retries on a version conflict** | DCP page | Fits `expected_version`, which we already have; adds the ownership rule | folds into CA-2 (PC-6); clinical |
| 8 | Referral content by reference, not copy | "data sharing through APIs … highly recommended"; a flag's detail "SHOULD NOT" be copied into a referral | Smaller referrals, less spread of sensitive data | a rule and a test in CA-3 (PC-2) |
| 9 | Hospital at home as an inpatient-class encounter at the patient's home, with a step-up or step-down route | virtual-wards page | Informs CA-1 | a design note only (PC-8); OQ-PC1 says whether it is in scope |
| 10 | Discharge and outcome documents shaped as HL7 Europe's Hospital Discharge Report | reports and appendices | CA-1's discharge summary is unspecified | a note (PC-9); the guide itself says emergency and mental-health discharge do not fit yet |

### 24.5 Risks

- **Following an unreviewed proof of concept.** Every rule above is "from the source, to be checked against 360X". The tasks say so and start with a pure model and tests, not a database.
- **Safeguarding.** Flags about child protection, looked-after children and FGM are the most sensitive data a record can hold. Task PC-7 is a design with a hazard row and the Clinical Safety Officer, not a build. Nothing here sets a hazard to accepted.
- **England bias.** Letting UBRN, CP-IS or MESH into the core would break the equity rule. They are pack content.
- **Two loops.** If the PAS also runs a referral loop, two copies of status will disagree. PA-1's boundary stands: the EPR owns the clinical loop, the PAS the administration, linked by an external reference; a state table does not change that.
- **A state machine is not a safety case.** A refused transition is a bug found early; it is not evidence that the loop never drops a referral.

### 24.6 Open questions (to move to spec/16-open-questions.md)

- **OQ-PC1.** Is hospital at home (a home-located inpatient stay with monitoring) in scope for 1.0? Recommendation: no; a design note in CA-1, post-1.0.
- **OQ-PC2.** Who may cancel or redirect a referral, and may a recipient redirect without the referrer's consent? The guide leaves both open. Recommendation: only the referrer cancels (a request the recipient may refuse); only the recipient redirects, creating a new linked referral; both audited. **Assumed 2026-11-03, awaiting the maintainer:** the sender of a referral is the referrer, any other permitted subject with a care relationship is the recipient; the redirect-creates-a-linked-referral step is not built.
- **OQ-PC3.** What are N and M for a referral with no response, or accepted with no outcome? Recommendation: configuration per service type with no default for urgent referrals, so a deployment must choose.

### 24.7 Order of work

PC-1 (the state table, pure) and PC-4 (the cancel-request rule and its hazard row) first, because they cost little and shape everything else;
then PC-2 (the identifier rule and the unmatched queue) and PC-3 (timeouts, after OQ-PC3); PC-5 as the decision before OR-1 or CA-3 is scheduled. PC-6 and PC-7 with CA-2 and PA-6.
PC-8 and PC-9 are notes.

### 24.8 Sources (this pass)

All fetched 2026-11-02 from <https://kevinmayfield.github.io/PatientCareCoordination/en/>: [`index.html`](https://kevinmayfield.github.io/PatientCareCoordination/en/index.html),
[`360X.html`](https://kevinmayfield.github.io/PatientCareCoordination/en/360X.html), [`WorkflowPatterns.html`](https://kevinmayfield.github.io/PatientCareCoordination/en/WorkflowPatterns.html),
[`NHSEngland.html`](https://kevinmayfield.github.io/PatientCareCoordination/en/NHSEngland.html), [`PatientFlags.html`](https://kevinmayfield.github.io/PatientCareCoordination/en/PatientFlags.html),
[`VirtualWards.html`](https://kevinmayfield.github.io/PatientCareCoordination/en/VirtualWards.html), [`DCP.html`](https://kevinmayfield.github.io/PatientCareCoordination/en/DCP.html).
Named by the guide and not read: IHE PCC 360X Rev. 1.2 (Trial Implementation), HL7 Europe Hospital Discharge Report 1.0.0 (STU 1), the EU Health Data API, IHE QEDm, MHD and DCP.

## 25. Research pass 2026-11-03: the patient-administration-system repository, read for what the record can learn

The request: research `~/git/patient-administration-system/patient-administration-system` and add any improvements. Nothing was built.
This is the sister project (Rust PAS, SvelteKit UI), not an outside source, so it is read as a working example: what it did
after the points already taken in §23 (Openfront) and §24 (care coordination) that this repository lacks. Each idea below was
**checked against this repository's code** before being written down; the check is named, and where a check was not made it says so.

**Evidence base, stated plainly.** I read the PAS at commit `4f75348` (2026-11-03): its `tasks.md` task titles and the first lines of
about eighty of them, `spec/hazard/index.md`, `governance/production-readiness.md`, the names of its `governance/` files and its
recent commit messages (OFH-1, OFH-4, OFH-10, PCC-1, PCC-2, PCC-5). I did **not** read its code, its `plan.md`, its UI, or the bodies of
its governance documents other than the path-to-production checklist, so I cannot say how well any of its controls work. The task
text is the PAS author's own account of what landed. The PAS covers appointments, waiting lists, wards, messaging and national
integration; those belong to a PAS under the boundary decision PA-1 (`spec/02-scope.md`) and are **not** proposed here.

### 25.1 What was already taken from it

Idempotency keys (OF-1), the read-auditing and refusal-audit censuses (OF-2, CQ-39), the schema map (OF-6), the closed-loop referral
table (PC-1) and the identifier rule (PC-2) all came from earlier passes. The PAS has since moved on from them in ways that matter below.

### 25.2 Ideas, ranked

| ID | Idea | Source in the PAS | Checked here | Value | Cost |
|---|---|---|---|---|---|
| PR-1 | A list that is cut says so. | QUAL-14 (`paging`, `PageMeta`) | `feed_queue`, `break_glass` and the new referral queue return the oldest 50 with a `limit` and no flag; audit search already answers `truncated`; the feed screen warns when it shows `limit` rows, the break-glass review screen does not. A local test database with 57 unreviewed grants hid the newest from the list (seen 2026-11-03). | High: an oldest-first safety queue hides what arrived last | S |
| PR-2 | A hazard control names its test, and the log check proves the test exists. | SAFE-5, SAFE-6 | `scripts/check-hazard-log.py` checks that each `evidence` path exists, not that a named test does; a control can survive a renamed test. | High for assurance | S |
| PR-3 | A stored list of words is checked against the database `CHECK` that guards it. | QUAL-28 | `referral_versions.state` has a `CHECK` of 12 words and `State::ALL` has 12, tied only by a unit test of serde; no test reads the constraint (no test in `tests/` reads `pg_constraint`). | Medium | S |
| PR-4 | A background job runs on one replica at a time. | QUAL-2 (`run_exclusive`, an advisory lock per job name) | `outbox::run_loop` claims with `FOR UPDATE SKIP LOCKED` and the idempotency purge is a plain `DELETE`; `audit_retention::run_loop` makes and archives partitions with no lock. **Not checked:** whether two replicas archiving at once fail harmlessly. | Medium, unverified | S |
| PR-5 | A controllable clock. | QUAL-25 (`Clock`, `FixedClock`) | 15 `Utc::now`/`SystemTime::now` calls in `src/`; tests move time by writing rows (the expired idempotency key test does). PC-3's timeouts cannot be tested without one. | Medium; needed by PC-3 | M |
| PR-6 | REST reads carry the version as an `ETag`, and a write may send it as `If-Match`. | OFH-1, OFH-10 (NFR-27) | Only the FHIR routes send a weak `ETag`; REST writes carry `expected_version` in the body. A decision, not a defect: see OQ-PR1. | Medium | M |
| PR-7 | Governance documents the PAS has and this repository lacks: a data-flow description, an MFA plan, a path-to-production checklist, an audit of national assumptions. | `governance/` listing; `production-readiness.md` read in full | `governance/` here holds the DPIA, record of processing, retention schedule, translation register and `clinical-safety/`; the four named documents are absent (a grep of `plan.md`, `tasks.md` and `spec/` for "production-readiness", "MFA plan" and "national-assumptions" finds none). | Medium: they are what a deploying organisation asks for | S each |
| PR-8 | Not adopted, recorded so it is not asked again: a weekly mutation run, appointment and bed features, waiting-list validation, PIFU, transport, reminder ladders. | QUAL-18 and others | Mutation testing is already an acceptance line in `tasks.md`; the rest is PAS scope (PA-1). | none | none |

### 25.3 Assessment

The strongest finding is PR-1, because it is a safety defect that appeared in this session's own test run and not a stylistic point: the
referral queue (H-044), the feed-refusal queue and the break-glass review list all show oldest first, so once 50 entries are waiting the
ones added last are the ones nobody sees, and only one of the three screens says so. The cure is cheap and the PAS already has the shape
(a page-meta that says the list was cut).

PR-2 and PR-3 are cheap checks that remove ways for a record of assurance to be quietly wrong. PR-4 may be nothing, and is written as a
question for that reason. PR-5 should be done with PC-3, not before. PR-6 is a choice about API style, not a gap, and PR-7 is writing, not code.

### 25.4 Risks

- **PR-1** changes the shape of three responses; the UI and the OpenAPI document must change in the same commit and an older client must
  keep working (add a field, do not rename one).
- **PR-2** will fail the hazard-log check on every hazard that names a file but not a test; the first run will be a long list. Land it as
  a report first, then make it a failure.
- **PR-3** can only compare words; it cannot say a word means what the code thinks.
- The PAS is the same author's code and the same style of claim. Its tasks say "landed" about its own work; nothing here was run.

### 25.5 Open questions (moved to spec/16-open-questions.md)

- **OQ-PR1.** Should REST reads send an `ETag` and accept `If-Match` beside the body field `expected_version`? Recommendation: yes, additive, with the body field kept; decide before any generic client or cache sits in front of the API.
- **OQ-PR2.** Is a deployment expected to run more than one service replica? Recommendation: assume yes, and settle PR-4 now; the cost is small and a single-replica assumption is easy to forget.

### 25.6 Order of work

PR-1 first (a safety defect, with a test that reproduces it), then PR-2 as a report, PR-3, PR-4's check, PR-7 as writing at any time. PR-5 with
PC-3. PR-6 after OQ-PR1.

### 25.7 Sources (this pass)

Read 2026-11-03 in `~/git/patient-administration-system/patient-administration-system` at commit `4f75348`: `tasks.md` (QUAL-2, QUAL-14,
QUAL-18, QUAL-25, QUAL-28, SAFE-1 to SAFE-6, OFH-1, OFH-4, OFH-10, OFH-11, PCC-1 to PCC-8, DTAC-0 to DTAC-12, NEW-1 to NEW-22),
`spec/hazard/index.md`, `governance/production-readiness.md`, the file names under `governance/`, `spec/compare/`, and `git log` messages.
This repository: `src/feed_queue.rs`, `src/break_glass.rs`, `src/referral_unmatched.rs`, `src/audit.rs`, `src/audit_retention.rs`, `src/outbox.rs`,
`scripts/check-hazard-log.py`, `src/routes/admin/*` in the UI, and `tests/`.

## 26. Research pass 2026-11-04: more of the checking done locally, less waiting on GitHub

The request: research how to do more with local CI/CD rather than relying on GitHub, which the author finds unreliable and costly; assess and advise;
add ideas to `plan.md`, `tasks.md` and `spec/*`. **Nothing was built or installed**, and none of the tools below was run. The reading is of this
repository's own CI and run history (via `gh`, 2026-11-04) and of the sources in §26.7; vendor pages are marked as such.

### 26.1 What the repository does today (checked)

- `.github/workflows/ci.yml` has 15 jobs behind one aggregate check, `ci-ok`; `release.yml` builds on a version tag. Every job runs on `ubuntu-latest`.
- One recent green run (push, 2026-11-04) took **39.6 job-minutes** across its jobs and **about 11 to 16 minutes** by the clock. The largest: the container build
  11.2, CodeQL for Rust 8.6, the API clippy job 4.8, the database tests 4.3.
- The last 200 runs: **143 success, 14 failure, 39 cancelled, 1 running**. The cancellations are the `cancel-in-progress` rule meeting the one-commit-one-push habit
  (a newer push cancels the older run). Of the 14 failures I opened three: a database-gated test, a UI type check and a UI image build, all **real failures of our own**, not platform faults.
  The other eleven were not opened, so "GitHub's own faults made none of them red" is **not** established.
- The repository is **public** (`gh repo view`). GitHub states that standard hosted-runner use on public repositories remains free, so **the money cost today is nil**; the cost is
  waiting: about 15 minutes per push, and a push per item by the project's own rule (AGENTS.md).
- The author's machine is an Apple-silicon Mac (arm64, 16 cores, 128 GB) with Podman 6.1.4. CI is x86-64 Linux. The gates already run locally (AGENTS.md "Gates"); the
  pre-commit hook runs only the cheap ones; `electronic-patient-record-service-with-rust/compose.test.yaml` starts the test Postgres.

### 26.2 Why GitHub feels unreliable and costly (sourced, third-party figures marked)

- **Reliability.** GitHub's own July 2026 availability report lists eight incidents, three of them on Actions: on 9 July hosted-runner jobs were delayed or failed to start (about 8% of runs delayed
  over five minutes, about 2% failed to start, 9 h 18 m); on 19 to 20 July **self-hosted and larger runners** were delayed by an expired certificate (5 h 10 m, up to 21.4% of runs); on 25 July workflow runs
  failed (about 25% then about 60% in two windows). A third-party tracker counts 257 GitHub incidents in 12 months, 57 of them Actions outages, but trackers disagree with each other (another counts 178),
  so treat the counts as indicative.
  **A self-hosted runner does not escape this**: the 19 to 20 July incident hit self-hosted runners and spared standard hosted ones.
- **Cost.** In December 2025 GitHub announced up to 39% lower hosted-runner prices from 1 January 2026 and a $0.002-per-minute platform charge, including for self-hosted runners from 1 March 2026.
  GitHub then **postponed** the self-hosted charge "to take time to re-evaluate our approach", with no new date. So a design that leans on free self-hosted minutes is exposed to a policy that
  is only paused. The price per minute of hosted Linux was **not** checked, and I give no dollar figure.
- **If the repository became private**: the Free plan's included compute is described as "over 33 hours" (about 2,000 minutes). At about 40 job-minutes per run and 8 to 12 pushes on a busy day, the
  allowance would last roughly a week. That is arithmetic on today's habits, not a quote from GitHub.

### 26.3 The options, assessed

| Option | What it is | For | Against | Verdict |
|---|---|---|---|---|
| **A. Local gates, one command, tiered** | `scripts/ci-local.sh` runs the AGENTS.md gates; a pre-push hook runs the fast tier; GitHub stays as the second, clean-room check | No new service; uses what already works; the 128 GB machine is faster than a 2-core hosted runner (to be measured, not assumed) | Self-attested; arm64 not x86-64; depends on one person's machine | **Do first** (LC-2 to LC-6) |
| **B. One definition of each gate** | `ci.yml` calls the same `scripts/gates/*` the local command calls | Local and remote cannot drift; shrinks YAML | A refactor of 15 jobs; actions such as CodeQL and cargo-deny have no local twin | Do after A, gate by gate (LC-3) |
| **C. Trim what GitHub runs** | Path filters, heavy jobs only on `main` or a schedule, fewer pushes | Cuts minutes and waiting with no new tool; `ci-ok` already treats `skipped` as pass | Less checking per commit; a schedule finds problems later | **Do** (LC-7), with the cost named |
| **D. `act`** | Runs the GitHub workflow files locally in containers (MIT licence) | Same YAML, no rewrite | Talks to the Docker API; a community thread says it does not transparently support Podman; **service containers under Podman were not confirmed** (we use `services: postgres`) | A one-day spike only (LC-9) |
| **E. Self-hosted GitHub runner on our machine** | A runner process on a box we own | Minutes not billed today | GitHub: self-hosted runners "should almost never be used for public repositories" (any fork PR can run on it); still needs GitHub to dispatch (the July outage); the postponed fee returns on GitHub's timetable | **Not adopted** for this public repository (LC-13) |
| **F. Forgejo plus Forgejo Runner (Podman)** | Our own forge with its own CI; GitHub becomes a mirror | Independence from GitHub's queue and billing; workflows in `.forgejo/workflows` are close to GitHub's | Docs: "not designed to be compatible"; some job keys are ignored (`permissions`, `continue-on-error`; `timeout-minutes` in v12); the runner is remote code execution on our host; service-container support **not confirmed**; we operate it | Spike only if OQ-LC2 is yes (LC-10) |
| **G. Woodpecker CI** | Lightweight container-first CI, `woodpecker-cli exec` for local runs | Small, runs locally | Which forges it supports and how services work were **not confirmed** here | Not assessed further |
| **H. Dagger** | Pipelines written in code, run by an engine container locally and in any CI; its docs list Podman | Same pipeline on laptop and CI | Rewrites every job in an SDK; one more engine to run and trust | Defer; revisit if B proves hard |
| **I. Faster builds** (`cargo nextest`, `sccache`, a faster linker) | Cuts compile and test time | Helps every option | Figures come from a CI vendor's blog and a training book, **not our code**; a cold cache removes the gain; sccache can slow release builds | Measure first (LC-8); adopt only what measured |

### 26.4 Advice

1. **Make local the first gate and GitHub the independent second check.** Do not leave GitHub. The `fresh-clone`, `secrets`, `deny` and CodeQL jobs, and the clean x86-64 environment, catch what one person's machine cannot (the intermittent `http_amendments` failure, not yet investigated, is a reminder that a pass on one machine is one observation). Local runs save the waiting; they do not replace the witness.
2. **Do A, C and the measuring first** (LC-1 to LC-8): all are small, need no new service and pay back at once. Record a baseline (LC-1) before changing anything so the gain is shown, not asserted.
3. **A locally produced pass is self-attested.** Sources on SLSA say a self-hosted build does not reach Build Level 2 by itself. For this project that means: use the local record to push with confidence and to
   explain a commit, **not** as the evidence a Clinical Safety Officer relies on; releases keep building in the hosted clean room (`release.yml`) until OQ-LC3 is answered.
4. **Plan for GitHub being down at release time**: today a tag push builds the release only on GitHub. Write the by-hand route down (LC-11) and keep a second remote of the source, so an outage delays a release but cannot strand the code.
5. **Fix the cause of the shared-database flakes.** Local runs meet the shared test database's leftovers (this session met two: more than 50 unreviewed break-glass grants, and 28 old idempotency keys). A scratch database per run (LC-4) removes that whole class and matches CI's fresh one.
6. **Do not adopt Forgejo, act or Dagger on reading alone.** Spike each for a day against three jobs and write down what failed (LC-9, LC-10). The `services:` Postgres is the likely trouble.
7. **Keep the one-commit-per-item rule but not necessarily one push per commit** (OQ-LC4): 39 of 200 runs were cancelled by a newer push; pushing when the local gates are green and checking CI on the last commit of a batch would save most of them. This is the author's call.

### 26.5 Ideas, ranked

| ID | Idea | Checked here | Value | Cost |
|---|---|---|---|---|
| LC-1 | Measure the baseline: job minutes, wall time, cancelled share, failures by cause, from `gh` | Numbers in §26.1 were got this way for one run and 200 statuses; no 30-day table | Shows what to cut | S |
| LC-2 | `scripts/ci-local.sh --fast / --full`: the AGENTS.md gates in one command with a result table | AGENTS.md lists the gates; no such script exists (`ls scripts`) | Removes the long command lines I paste by hand | S |
| LC-3 | One definition per gate, called by both `ci.yml` and the script | 15 jobs, many inline `run:` blocks | No drift | M |
| LC-4 | A scratch database per local run, dropped after | The shared-DB failures met this session | Removes a flake class | S |
| LC-5 | `.githooks/pre-push` running the fast tier for the projects touched | `.githooks/pre-commit` exists; `install-hooks.sh` installs hooks | Red CI for formatting stops reaching GitHub | S |
| LC-6 | A signed local gate record (commit, clean tree, tool versions, results, durations, host arch) | Commits are already ssh-signed | Evidence of what ran, honestly labelled self-attested | S |
| LC-7 | Trim GitHub: path filters, heavy jobs on `main`/schedule | `ci-ok` fails only on `failure`/`cancelled`, so a skipped job passes | Fewer minutes, less waiting | S |
| LC-8 | Speed spike: `cargo build --timings`, nextest, sccache, linker; adopt on measured gain | Numbers elsewhere are a vendor's | Maybe halves the loop | S |
| LC-9 | `act` spike on three jobs under Podman | Not run | Cheap test of the "same YAML" route | S |
| LC-10 | Forgejo and Forgejo Runner spike, GitHub as mirror | Not run; needs OQ-LC2 | Full independence | M |
| LC-11 | A "GitHub is down" route: release by hand, a second remote of the source | `release.yml` runs only on a tag push | An outage no longer blocks a release | S |
| LC-12 | A preflight in the script: Podman VM clock drift, ports 5543 and 6390, `PW_PORT`, jars | AGENTS.md documents the clock-drift failure | Fewer confusing local failures | S |
| LC-13 | Record "no self-hosted GitHub runner on the public repository" as not adopted | GitHub's secure-use page | Settles the question | S |

### 26.6 Risks

- A local pass that differs from CI breeds false confidence: arm64 against x86-64, a dirty tree, a stale test database, a different Postgres version. LC-6 refuses a dirty tree and records the arch; CI stays as the check.
- A pre-push hook that is slow gets skipped with `--no-verify`. Keep the fast tier under a few minutes or measure why not.
- A second forge adds an operations burden and an attack surface (the runner runs repository code on our host). For a safety-related record that burden should be chosen, not drifted into.
- Counting on a postponed fee never returning, or on a vendor blog's speed figures holding for our code.
- Treating a local record as clinical-safety evidence without anyone agreeing it may be.

### 26.7 Open questions (in `spec/16-open-questions.md`)

OQ-LC1 stay public; OQ-LC2 a second forge; OQ-LC3 may a local record count as evidence; OQ-LC4 push cadence.

### 26.8 Order of work

LC-1, LC-2, LC-4, LC-5, LC-12, then LC-7 and LC-6, then LC-8; LC-3 gate by gate; LC-11 before the next release; LC-9 and LC-10 only after the above and OQ-LC2.

### 26.9 Sources (read 2026-11-04)

- GitHub, *2026 pricing changes for GitHub Actions* (hosted price cut from 1 January 2026; self-hosted charge postponed 15 December 2025; public repositories stay free): <https://github.com/resources/insights/2026-pricing-changes-for-github-actions>
- GitHub changelog, *Update to GitHub Actions pricing* (16 December 2025, the announcement): <https://github.blog/changelog/2025-12-16-coming-soon-simpler-pricing-and-a-better-experience-for-github-actions/>
- GitHub, *Availability report: July 2026* (the three Actions incidents quoted): <https://github.blog/news-insights/company-news/github-availability-report-july-2026/>
- GitHub Docs, *Secure use reference* (self-hosted runners on public repositories; just-in-time runners): <https://docs.github.com/en/actions/reference/security/secure-use>
- IncidentHub (third party), *GitHub reliability and outage history 2025 to 2026*: <https://blog.incidenthub.cloud/github-reliability-outage-history-2025-2026> (counts disagree with StatusGator; indicative only)
- Forgejo, *Forgejo Actions administration*: <https://forgejo.org/docs/latest/admin/actions/>; *Forgejo Actions and GitHub Actions* (differences): <https://forgejo.org/docs/next/user/actions/github-actions/>
- Community guides to the Forgejo runner under rootless Podman (socket exposure caveat): <https://berrypi.codeberg.page/info/forgejo-runner/>, <https://honeypot.net/2025/05/15/i-wanted-to-experiment-with.html>, issue <https://code.forgejo.org/forgejo/runner/issues/139>
- nektos/act (MIT; Docker API): <https://github.com/nektos/act>; community thread on Podman: <https://github.com/ChristopherHX/github-act-runner/discussions/178>
- Woodpecker CI introduction: <https://woodpecker-ci.org/docs/intro>
- Dagger integrations (any OCI runtime, Podman listed): <https://docs.dagger.io/integrations>
- Depot, *Guide to faster Rust builds in CI* (vendor; nextest, sccache, mold): <https://depot.dev/blog/guide-to-faster-rust-builds-in-ci>
- Self-hosted builds and SLSA Build Level 2 (secondary guide): <https://guptadeepak.com/guides/verified-builds-slsa-sigstore/>

### 26.10 Not done, not verified

No tool above was installed or run. The billing page of the author's account was not read, so "the cost today is nil" rests on GitHub's statement about public repositories. Eleven of the 14 recent failures were not opened.
The one-run timing is one run. Hosted-runner prices, mold on macOS, whether the CodeQL command-line tool may be used locally, Woodpecker's forge support, and every service-container claim (act, Forgejo, Woodpecker) were not checked.
Third-party incident counts disagree. No benchmark was run on this repository, so no speed-up is claimed.

## 27. Research pass 2026-11-04: mobile viewability, care settings, specialty views, pathway design and homomorphic encryption

The request (five points): emphasise mobile viewability; close the gap that systems are built for one of acute, community or mental-health care, so an acute provider adding community or
mental health finds it works badly; add homomorphic encryption for data analysis; add specialties such as a renal-specific view; let clinicians design and redesign care pathways. Documents only; **nothing built,
no tool installed, no clinician consulted.** Specs: [`mobile-viewability`](spec/mobile-viewability.md), [`care-settings`](spec/care-settings.md), [`pathway-design`](spec/pathway-design.md), [`privacy-analytics`](spec/privacy-analytics.md).
Builds on §26's parent idea, the pathway-first rule (spec/05 §5.2.1).

### 27.1 Assessment

| Point | What the repository has | What the sources say | My assessment |
|---|---|---|---|
| Mobile viewability | A viewport tag and a dark-mode query; Playwright on desktop Chrome only; `<table>`s in four screens; idempotency keys already make a retry safe | WCAG 2.2: reflow at 320 CSS px, targets at least 24 px | **Highest value and cheapest.** The UI already exists, so each screen added without a phone test adds debt. Do first (XP-1 to XP-4). The banner on a small screen is a **patient-safety** point, not cosmetics |
| Acute versus community and mental health | The scope says ambulatory first, inpatient core next; CA-1 plans encounters and a minimal bed board | Trade-press and supplier examples: mobile access, Mental Health Act compliance, statutory returns, long caseload care | **Agree it is a real design risk**, and the pathway-first rule is the main defence: a discharge ends a stay, not the care. Make "no bed, no admission in the core" a stated rule now (XP-5), build profiles later |
| Homomorphic encryption | Encryption at rest and an integrity MAC; nothing for analysis | Orders of magnitude slower, megabyte ciphertexts, thin operations and standards | **A narrow tool, post-1.0, spike first.** It will not make clinical care safer and does not by itself make an output safe; pair it with output rules. Compare with a trusted research environment and differential privacy before committing (XP-8) |
| Specialty views | Lily widgets, packs mechanism designed (OS-10) | A renal registry feed and dated, uneven eGFR results are real constraints | **Do as packs on one record.** A separate record per specialty is the failure to avoid. Renal as the worked example, written with a renal clinician |
| Clinician-designed pathways | Nothing; pathway basics are CA-2 | HL7 CPG layering exists; `PlanDefinition` is hard for clinicians; **no clinician-authored tool found** | **The most valuable and the riskiest.** Valuable because the people who know the pathway can fix it; risky because a changed template changes live care. Needs versioning, review and simulation (XP-12, XP-13) |

### 27.2 Ideas, ranked

| ID | Idea | Value | Cost |
|---|---|---|---|
| XP-1 | Mobile audit: Playwright phone projects, axe and a no-horizontal-scroll check at 320 px; record what breaks | High | S |
| XP-2 | Make the core screens reflow: banner, patient list, record page, referral screens | High | M |
| XP-3 | A hazard row and a rule for what the banner and allergies show at 320 px | High | S |
| XP-4 | A design note for poor signal: downtime snapshot read, queued writes (OQ-5, OQ-MV1) | Medium | S |
| XP-5 | Read the spec and code for acute-only assumptions (bed, admission, discharge as end of care) and list them | High | S |
| XP-6 | Setting profiles (acute, community, mental health): work-unit view and defaults, after CA-1 and CA-2 | High | M |
| XP-7 | What a legal-status record needs for mental health, as a pack, no country's law in the core | Medium | S |
| XP-8 | Homomorphic-encryption spike on synthetic data, with differential privacy and plaintext as comparators | Medium | S |
| XP-9 | Secondary-use export design: purpose of use, minimum cell size, the data protection officer's question | Medium | S |
| XP-10 | Specialty-pack design: what a pack holds, review status, how a view adds and never hides | High | S |
| XP-11 | Renal view on synthetic data, written with a renal clinician | Medium | M |
| XP-12 | Pathway template model: versioned, immutable, applied to a patient as a plan; migration rules | High | M |
| XP-13 | Clinician pathway editor with draft, review, publish and a simulation | High | L |
| XP-14 | Pathway analytics: time in step, open loops, skipped steps | Medium | S |

### 27.3 Advice and order

1. **Mobile first (XP-1, then XP-3 and XP-2).** It improves what exists and shapes everything after.
2. **XP-5 now, profiles later.** The cheap, valuable thing is to stop the core assuming a bed; the profile mechanism waits for CA-1 and CA-2 (and OQ-ST1).
3. **Pathway model before the editor.** XP-12 needs CA-2 and OQ-CP3; XP-13 is an L-sized item whose safety case (review, versioning, simulation) is the real work, not the drawing.
4. **Homomorphic encryption last and small.** XP-8 is a week on synthetic data; the answer may well be "use a trusted research environment".
5. **Renal with a renal clinician or not at all** (XP-10, XP-11): no one outside renal care should decide what the summary page leads with.

### 27.4 Risks

- A phone layout that hides allergies, alerts or the patient's identity to fit.
- A setting profile that becomes a fork, so acute and community drift into two systems.
- A pathway template changed in the editor that silently changes live patients' care.
- Treating encrypted data as no longer personal, or sending out a small-cell aggregate.
- A specialty view that hides safety information, or ships clinical content nobody reviewed.
- Claiming any of this is done: every item is a direction.

### 27.5 Open questions (spec/16-open-questions.md)

OQ-MV1 devices and offline; OQ-ST1 which settings in 1.0; OQ-ST2 mental-health law as a pack; OQ-SP1 who authors a specialty pack; OQ-HE1 status of encrypted data; OQ-HE2 whether to do homomorphic encryption at all; OQ-PD1 who may publish a pathway template.

### 27.6 Sources (read 2026-11-04; most as search results or in part)

- WCAG 2.2 success criteria 2.5.8 and 1.4.10, via secondary summaries: <https://kb.daisy.org/publishing/docs/wcag/target-size-minimum.html> (confirm on w3.org).
- Community and mental-health EPR examples: <https://www.digitalhealth.net/2016/06/special-report-mental-health-systems/>; <https://www.theaccessgroup.com/en-gb/health-social-care/health-and-support-case-studies/health-related-case-studies/somerset-partnership-nhs-foundation-trust/>; <https://buildingbetterhealthcare.com/lincolnshire-nhs-trust-goes-live-with-rio-electronic-patient-record--148402>.
- Renal: UK Renal Registry, <https://ukkidney.org/sites/renal.org/files/23rd_UKRR_ANNUAL_REPORT.pdf> and its Q&A, <https://renalregistry.atlassian.net/wiki/spaces/UD/pages/2359853069/Questions+and+Answers+-+UKRR>; a primary-care renal-data study, <https://eprints.soton.ac.uk/398654/1/AcceptedNDT20160625.pdf>.
- Pathway design: HL7 Clinical Practice Guidelines IG, <https://hl7.org/fhir/uv/cpg/documentation-methodology.html> and <https://hl7.org/fhir/uv/cpg/StructureDefinition-cpg-pathwaydefinition.html>; FHIR `PlanDefinition`, <https://hl7.org/fhir/R4/plandefinition.html>.
- Homomorphic encryption: <https://homomorphicencryption.org/standard/>; ISO/IEC 18033-6, <https://www.iso.org/standard/67740.html>; Chalmers thesis, <https://odr.chalmers.se/items/5d2cf46b-ce60-459c-95df-3c9bd6d53987>.

### 27.7 Not done, not verified

Nothing was built. I did not run a phone or an emulator, install a homomorphic-encryption library, read the full text of any standard, or talk to a clinician. The acute versus community comparison is the author's observation supported by examples, with no direct comparison source found.
Renal content comes from registry material and a research paper, not a renal clinician; vascular access and transplant are unsourced. The HL7 pathway material was read from search results and part of one guide. The ActCode values for `Encounter.class` are to be checked.
Third-party and student-thesis sources on homomorphic encryption are indicative only. No hazard row was added (documents only); rows are due with the first code.

## 28. Research pass 2026-11-04: comparisons with other EPRs

The request: compare with Epic, Nervecentre, Oracle Cerner, System C, Altera, Alcidion, TPP SystmOne, EMIS and others, one `spec/<slug>/index.md` each. Twelve pages: [Epic](spec/epic/index.md), [Oracle Health (Cerner)](spec/oracle-cerner/index.md), [MEDITECH](spec/meditech/index.md), [Nervecentre](spec/nervecentre/index.md), [Alcidion](spec/alcidion/index.md), [System C](spec/system-c/index.md), [TPP SystmOne](spec/tpp-system-one/index.md), [EMIS](spec/emis/index.md), [Access Rio](spec/access-rio/index.md), [Altera](spec/altera/index.md), [InterSystems TrakCare](spec/intersystems-trakcare/index.md), [Dedalus](spec/dedalus/index.md). Documents only; **no vendor document under contract, no product demonstrated, nobody who uses these systems consulted.** Marketing, trade press and third-party guides are marked as such on each page.

### 28.1 Assessment

The vendors are not this project's competitors in any real sense: they are working products, and this is a scaffold (M0) with no clinical record. What they give is a list of ways a working EPR has failed in public. Five patterns repeat across the pages, and they are worth more than any feature list:

| Pattern | Pages | What it means here |
|---|---|---|
| A message left and nobody noticed it did not arrive | Dedalus, EMIS, Oracle Cerner | The closed-loop referral should cover discharge and transfer messages, and an order to an unowned destination must surface (VC-1, VC-2) |
| Go-live is where it goes wrong | Epic, Oracle Cerner, System C, TrakCare, Nervecentre | A staged plan, a known-defects list and a migration reconciliation are part of the product's safety (VC-5) |
| Outages and poor signal | EMIS, Oracle Cerner, Nervecentre | Downtime is a designed mode, not an exception (VC-4, VC-9) |
| A sharing preference missed on one path | TPP SystmOne | Prove a preference on every outbound path by test (VC-6) |
| Conformance claims nobody can check | Altera, Dedalus, System C, Epic | Generate ours from the code; publish who may read what (VC-3) |

### 28.2 Ideas, ranked

VC-1 to VC-12 in `tasks.md`, in value order. Ideas on the vendor pages that restate existing work (setting profiles, legal status, the Mental Health Act record) stay with XP-5 to XP-7.

### 28.3 Not done, not verified

Each page has its own "Not verified" list. Vendor claims are claims. Incident accounts are public reports, usually from trade press or inspectors, and are not a finding of cause. No page says a product is better or worse overall, and none should be read that way. No hazard row was added: the patterns are candidates for the Clinical Safety Officer.

## 29. Research pass 2026-11-04: clinical safety measures, maternity, manual steps, data in and out, data quality, productivity, training

The request, eight points: how do we measure clinical safety; how does maternity work; how does the information change at a manual step; how do you get data in and out; data quality assurance and governance; how to measure before-and-after productivity; what training will be available; and build a training overlay. Documents only: **the training overlay is a design, not a build, and no clinician, midwife, trainer or records manager was consulted.** Specs: [measuring clinical safety](spec/clinical-safety/measurement.md), [maternity](spec/maternity/index.md), [manual steps](spec/manual-steps/index.md), [data exchange](spec/data-exchange/index.md), [data quality](spec/data-quality/index.md), [productivity measurement](spec/productivity-measurement/index.md), [training](spec/training/index.md), [training overlay](spec/training-overlay/index.md).

### 29.1 Assessment

| Point | What the repository has | My assessment |
|---|---|---|
| Measuring safety | A hazard log; an audit trail and queues that could be measured | **No measure is a verdict.** About sixteen measures across seven hazard classes, thresholds left to a baseline and the Clinical Safety Officer. Start with identity measures, which the scaffold can already compute (SM-3). Nobody can review any of it until OQ-2 is answered |
| Maternity | Twins are never auto-linked; nothing else | **Decide build or integrate first (OQ-MT1).** The identity and access layers are ours whichever way that goes: two patients, one event; hidden disclosures; a baby's own consent. My recommendation is to integrate with a specialist system and own the pathway, identity and access layers |
| Manual steps | Append-only versions with author and time | **A value's history must say how it got there.** Reported by, transcribed from, and when it occurred, each `null` with a reason, never defaulted (DX-8). A back-keyed entry must not carry the keying time (DX-9) |
| Data in and out | Inbound ADT, FHIR reads, feed refusal queue, no bulk import or `$export` | **An import is a run with an identity**: checksum, mapping version, operator, counts, and a dry run that writes nothing (DX-1, DX-2). "Empty must never become none" |
| Data quality | Matcher, refusal queues, census tests | **Rules in a catalogue, a dictionary a census can check, and a duplicate sweep** (DX-4 to DX-6). A score never without its denominator, and never read as clinical truth |
| Productivity | Audit stream, no baseline | **Measure teams, not people, and never speed without its paired safety measure** (PM-1, PM-5). A go-live dip is expected and is not evidence either way |
| Training | Synthetic data for tests | **Competence, not completion.** A role-to-task matrix drives the lessons; downtime is practised (TO-7, TO-8) |
| Training overlay | Nothing | **Training must never be mistakable for real care.** A separate deployment and realm with an `EPR_ENVIRONMENT` setting that has no default, then an overlay on the real screens. TO-1 comes first because it is the safety control |

### 29.2 Ideas, ranked

SM-1 to SM-5, MT-1 to MT-4, DX-1 to DX-10, PM-1 to PM-5 and TO-1 to TO-9 in `tasks.md`. Do first: TO-1 (a real risk once anyone runs a demonstration), SM-1 and SM-3 (small, and they make the hazard log useful), DX-8 (it shapes every clinical table, so it is cheapest before they exist), MT-1 (a decision).

### 29.3 Not done, not verified

Nothing was built, including the overlay. Each page has a "Not verified" list; candidate hazards on them are candidates, not rows in the log, and are added with the first code. Figures from the literature (error rates, burden) are as reported and are not baselines for this project. Maternity content comes from public reviews and data-set documents, not from a midwife or an obstetrician, and must not be used as clinical content. No measure has been computed.

## 30. Research pass 2026-11-04: system quality attributes

The request: research system quality attributes, one `docs/<attribute>/index.md` each. Twelve pages, indexed at [`docs/index.md`](docs/index.md): ISO/IEC 25010:2023's nine product-quality characteristics, plus interoperability, privacy and auditability, which matter too much here to stay as sub-characteristics. Documents only.

### 30.1 Assessment

The pages agree on one thing: **the project's strongest quality evidence is about safety of change (census tests, guards, audit refusal); the weakest is about operation.** Nothing has been restored from a backup, so NFR-3's RPO and RTO cannot be tested. Nothing has run on two replicas. Coverage floors have not run on GitHub, and mutation testing covers two matcher files. Flexibility is entirely design. Privacy has no retention periods. Where a page gives a target, read it as a target.

### 30.2 Ideas, ranked

QA-1 to QA-12 in `tasks.md`. Many ideas on these pages extend existing tasks (AS-7 restore, OS-5 benchmarks, SE-5 tenancy, SE-10 second factor, AU-3, CQ-18) and are noted on the pages rather than repeated. AS-7 stays the highest-value operational item.

### 30.3 Not done, not verified

The ISO/IEC 25010:2023 wording comes from secondary summaries; the standard was not read. For the reliability, security, privacy and auditability pages, the crashed session's own note said only one source was checked at the time and the others were cited from memory; a link check on 2026-11-04 tests that they exist, not that they say what is claimed. Legal points on those pages (the subject-access deadline, the Article 17 health exemption) are recalled and need checking. Each page's "Not verified" section says so.

## 31. Research pass 2026-11-04: roles and protected information, care homes, virtual wards and step-down

Three requests: research EPR roles, assignments, permissions and operations, especially for highly protected information such as sexually transmitted infections, gender transition and violent attacks; research care home integration; research how a clinician discharges into a virtual ward or step-down facility. Specs: [roles and assignments](spec/access-control/roles-and-assignments.md), [protected information](spec/access-control/protected-information.md), [care homes](spec/care-homes/index.md), [virtual ward](spec/virtual-ward/index.md), [step-down facility](spec/step-down-facility/index.md). Documents only, **except one new hazard row, H-045.** No lawyer, clinician, data protection officer, care home or patient was consulted.

### 31.1 Assessment

| Point | What the repository has | My assessment |
|---|---|---|
| Roles and assignments | 11 roles, a route gate, a record decision, care relationships, break-glass | **The algorithm is ahead of the human process.** No assignment record, no end dates, no cover, no recertification, no monitoring. Real misuse is usually stale access or a log nobody read (RA-1, RA-5, RA-6) |
| Protected information | Six labels, masking designed, no labelled record | **A defect: `ETH` means substance use in HL7, and the code reads it as ethnicity (H-045, PI-1).** Fix before SE-4. Beyond that, a mask on a screen is the smallest part: most leaks are portals, proxies, letters, texts, printouts and exports (PI-7). Hiding is not free either: break-glass shows nothing of a labelled record, which may be a hazard of its own (OQ-PI2) |
| Gender history | Nothing | **The hardest case.** A deliberate break between records meets the matcher, the append-only record and clinical safety, and s.22 of the Gender Recognition Act may make disclosure an offence. Do not build before a lawyer and trans patients have reviewed it (PI-4, PI-5) |
| Care homes | Care relationships, closed-loop referral | **Integrate, do not replace.** England has assured suppliers and a shared dataset; the main risk is the transfer of medicines. An organisation as a holder of a care relationship is the first thing to test (HM-1) |
| Virtual ward, step-down | Closed-loop referral | **One discharge workflow, many destinations.** Discharge blocked until acceptance, a "who is responsible" panel, and missing observations shown as missing. Hospital at home stays out of 1.0 until pathways exist (OQ-PC1) |

### 31.2 Ideas, ranked

RA-1 to RA-7, PI-1 to PI-9, HM-1 to HM-5 and VW-1 to VW-6 in `tasks.md`. PI-1 comes first of everything in this pass: it is small, and it blocks SE-4.

### 31.3 Not done, not verified

Nothing was built. The protected-information page was written after the crash of the session that researched it, from that session's saved search results, and no new searches were run: it inherits their limits. All law is a lay reading. The order under s.22 of the Gender Recognition Act, the point that matters most, was not read. Adoption, witness protection, psychotherapy-note law and genetic-information law were not researched. Whether any immigration data-sharing agreement is in force is unknown. People with lived experience of these categories have not seen the page. Each page has a "Not verified" list. Only H-045 went into the log; the remaining candidate hazards go in with the first code.

A link check of all 401 URLs cited in §28 to §31 ran on 2026-11-04: 20 returned 404 and are noted on the page that cites them, and what rests on them is unconfirmed. Another 95 returned 403 and 24 did not answer; those are mostly bot walls and timeouts, and were not treated as dead, but they are not confirmed either. A live link shows that a page exists, not that it says what is claimed.

## 32. Research pass 2026-11-04: bereavement, suicide watch, survivor grief, trauma treatment

The request: research and add aspects for bereavement, suicide watch, survivor grief counselling and trauma treatment. Specs: [bereavement](spec/bereavement/index.md), [survivor grief](spec/survivor-grief/index.md), [suicide watch](spec/suicide-watch/index.md), [trauma treatment](spec/trauma-treatment/index.md). Documents only, **except two new hazard rows, H-046 and H-047.** No clinician, lawyer, or person who has been bereaved, at risk or traumatised was consulted, and these pages should not be built from until such people have shaped them.

### 32.1 Assessment

| Point | What the repository has | My assessment |
|---|---|---|
| Bereavement | **No concept of death**: no `death` or `deceased` in the service or matcher source; PID-29 and PID-30 are dropped (H-046); a FHIR body with `deceased[x]` is refused; a proxy can outlive the patient | **Design this before any letter or portal exists**: it is far cheaper now. A death is an append-only assertion with evidence and status, not a column, because it can be wrong, and each effect must be restorable. Local notice, registration and an inquest run on very different clocks, so waiting for formal status keeps writing to families in the hardest cases (BV-1 to BV-4) |
| Survivor grief | Closed-loop referral | **Offer, do not auto-refer, and be careful what is not built.** Referral does not fit a charity with no EPR, so a referral would sit unanswered for ever: separate information given, signposted and referred (SG-1). No family view, no list of people bereaved by suicide, no anniversary mailing, no grief score (SG-2) |
| Suicide watch | Observations with separate recorded and effective times; no support-worker role; alerts do not exist | **The core must hold no risk score.** NICE NG225 bars risk tools and stratification for prediction or for treatment and discharge decisions, and NHS England guidance says to remove stratification from record systems; a US deployment needs a validated screen, so that belongs in a pack (SW-1). Falsified observations recur in coroners' reports. Append-only shows a late entry as late, but cannot show that an on-time entry is false: **do not claim it solves this** (SW-2) |
| Trauma treatment | Six labels, none on clinical records; a matcher rule for placeholder names | **Both senses need work.** Physical: a temporary identity state, since the safer naming format reads as real to the matcher (H-047, TT-1). Psychological: section 28 of the Victims and Prisoners Act 2024 and its code bear on what a therapist writes, and a note cannot be withdrawn from an append-only record. That is the sharpest conflict with a survivor's interests, and it needs a lawyer and survivors (OQ-TT1) |

### 32.2 Ideas, ranked

BV-1 to BV-8, SG-1 to SG-5, SW-1 to SW-7 and TT-1 to TT-7 in `tasks.md`. Do first: SW-1 (a one-line guard against a harmful default), BV-1 and BV-2 (death, before anything writes to a patient), TT-1 (matcher spec first).

### 32.3 Not done, not verified

Nothing was built. Each page has a "Not verified" list. NICE pages returned 403, so NG225 and NG116 were read from third-party PDF copies and their wording may differ; other NICE guidelines were seen through search. All law is a lay reading; the Crime and Policing Act 2026 was not read. The Lampard Inquiry has not reported, so nothing from it is a finding. No study was found behind "tell your story once", and no measure of letters reaching dead patients. Whether a sexual assault referral centre's records are kept apart from GP records was not found. Evidence is English-language and mostly from high-income countries. H-046 and H-047 come from reading code and the spec, not from running a message or a name through it. A link check on 2026-11-04 tests only that cited pages exist. Candidate hazards on the pages go in the log with the first code. `electronic-patient-record-service-with-rust/spec/index.md` says referrals have no route, though they do; that was not changed here.

## 33. Research pass 2026-11-05: the patient administration system's modules

The request: research `~/git/patient-administration-system/patient-administration-system` and add capabilities to this repository. Unlike §25, which read task titles and no code, this pass read the module headers of 19 PAS modules at commit `15e6389`, `src/adjustments/mod.rs` to line 200, and NFR-28 and NEW-25. It did not read the rest of the PAS code or run it, so what the PAS says of itself is the PAS's account.

The PA-1 boundary stands: appointments, waiting lists, wards, reminders, transport and interpreter booking belong to a PAS. The test here was whether a capability is about the person and the record, not about running a service.

### 33.1 Built from this pass

PA-6, reasonable adjustments and communication needs, in [`spec/reasonable-adjustments/index.md`](spec/reasonable-adjustments/index.md). The PAS's catalogue of 20 codes was taken with its wording, and three interpreter codes added, because the PAS holds those in a separate module. Changes from the PAS: versions are append-only here, where the PAS sets columns on one row; every change needs a reason; a spoken interpreter needs a language; a person may need two languages at once.

### 33.2 Ideas, ranked

| ID | Idea | Source in the PAS | Checked here | Value | Cost |
|---|---|---|---|---|---|
| PX-1 | The patient sees and can correct their own needs. | NEW-20; the Accessible Information Standard's first step is to ask | No route for a patient or a proxy in `controllers/adjustments.rs` | High | M |
| PX-2 | Reviews that are overdue, across patients, oldest first, for a person to follow up. | `needs_review` | Overdue shows only on one record | High | S |
| PX-3 | An adjustment as a FHIR `Flag`. | `adjustments::to_flag` | No `Flag` route. Needs a decision on code systems | Medium | S |
| PX-4 | A FHIR `AuditEvent` claims a BALP profile only when it meets the profile's constraints. | `fhir_audit::violations` | Done 2026-11-07 for a read, registration or update of the patient record, checked by the validator with `ihe.iti.balp` 1.1.4. A search and a row about another record claim nothing: `spec/audit/index.md` | Medium | M |
| PX-5 | `/_health` says which dependency is down, with one answer reused for 5 seconds so an open route cannot drive load. | NFR-28 | `/_health` is loco's built-in; not read further | Medium | S |
| PX-6 | Not adopted, so the question is not asked again: waiting-list validation, standby, PIFU, transport, ward movements, reminder ladders, delivery receipts and billing are PAS scope. Support flags wait for PC-7. Red, amber and green tolerances wait for the rule in `spec/clinical-safety/measurement.md` that a threshold follows a baseline. | `list_validation`, `standby`, `pifu`, `transport`, `adt`, `support`, `tolerance` | PA-1 | none | none |

### 33.3 Not done, not verified

The catalogue's words were written for a PAS and read by no clinician here. The Accessible Information Standard is cited as the PAS cites it; I did not read it for this pass. No national code list was checked for licence.

## 34. Research pass 2026-11-09: NHS Wales national systems

The request: research the Welsh Clinical Portal, the Welsh GP Record, the Welsh Immunisation System, the Welsh Intensive Care Information System, the Welsh Nursing Care Record and the Welsh Community Care Information System, assess them, and advise. Public sources only, read 2026-11-09; the findings, each with its source, are in [`spec/nhs-wales/index.md`](spec/nhs-wales/index.md). Much of what is known comes from DHCW describing its own work, and that page marks it.

### 34.1 Assessment

Wales built a national portal (WCP) over national repositories (results, documents, the GP summary) rather than one record. What has gone well in public sources is narrow and standardised: WNCR's nursing forms, each a versioned national standard with named sign-off, reached all eligible wards; WIS reached 10 million vaccinations. What has gone badly is broad and shared: WCCIS, one record for 29 organisations, was found by Audit Wales to mean different things by "live", to slow its users, and to lack Welsh, mobile working and NHS interfaces seven years in; it is being replaced by separate products joined by standards. WICIS, a supplier's ICU system, has no confirmed live unit six years after contract. Outages (21 in six months of 2018, a missed chemotherapy) and write-back gaps (flu vaccinations keyed in by hand) are the clinical cost of a record held in more than one place.

For this project the lessons are mostly ones it already follows (append-only, standards first, stating what is not done); the new ones are small and specific, below.

### 34.2 Ideas, ranked

| ID | Idea | From | Checked here | Value | Cost |
|---|---|---|---|---|---|
| WL-1 | A result sign-off records the result version it was made on; a corrected result reopens it. | WCP results sign-off | OR-4 specifies acknowledgement and escalation, not the version | High | S |
| WL-2 | A patient choice follows the person: an objection survives a move and a merge of two records. | WGPR opt-out lost on a change of practice | SE-2 holds consent per patient; merges were not checked against it | High | S |
| WL-3 | An assessment form is a versioned standard with named clinical sign-off, and each completed one keeps its form version. | WNCR | EM-3 is open; OQ-M2 asks about licences | High | M |
| WL-4 | Permission at the point of care: "asked and agreed", or "could not ask" with a clinical reason, recorded on the view. | WGPR | Purpose and break-the-glass exist; no "asked" basis | Medium | M |
| WL-5 | A record kept in two places has one direction of flow, a write-back and a reconciliation report. | WIS write-back gap | RC-5 immunisations are not built; DX has no write-back | Medium | S (spec) |
| WL-6 | A watch list with a required reason and an end. | WCP | No watch list | Medium | S |
| WL-7 | A narrower view by role (medicines and allergies only for a vaccinator). | WGPR (snippet) | PI and RA restrict by category, not by role view | Medium | M |
| WL-8 | Welsh from the start: clinician strings in Welsh, reviewed by a speaker, before any Welsh deployment. | WCCIS lacked it | `cy-001` exists and falls back to English | Medium | M |
| WL-9 | The Wales FHIR guide as a national pack. | WHC/2023/018 | IO-18 designs packs; no Wales pack | Medium | M |
| WL-10 | A device reading counts only once a clinician validates it, with device and time of capture kept. | WICIS | EM-11 is post-1.0 | Low now | S (spec) |
| WL-11 | "Live" is a list of functions in use, per site. | WCCIS ("live meant different things") | The generated status counts what is built, not where it is used | Low now | S |
| WL-12 | Not adopted, already held: an exit path (DX `$export`), one instance not copies, downtime (CA-7), monitored audit (RA-6), honest reporting. | WCCIS, WNCR, PAC 2018 | Recorded so the question is not asked again | none | none |

### 34.3 Advice

Do WL-1 and WL-2 next: both are small, clinical and close to code that exists (OR-4's design and SE-2). WL-3 should be decided with EM-3, because every risk tool needs a licence answer first (OQ-M2), and the All-Wales forms' licences were not looked for. WL-4 and WL-7 belong with the access-control work (SE-4, PI, RA), not before it. WL-8 and WL-9 matter only if Wales is a target deployment (OQ-WL1); until then they are recorded, not started. Do not copy WCCIS's shape: one shared record for many organisations failed where standards between separate systems are now the plan.

### 34.4 Risks

- Most capability claims are DHCW's own; independent sources cover failures more than successes, so the picture leans.
- The Welsh repository's model (openEHR in 2021, FHIR in 2026) was not settled; nothing here should depend on it.
- A Welsh deployment would bring Welsh law and standards (the Welsh Control Standard, the Welsh language standards for health bodies) that this pass did not read.

### 34.5 Open questions

OQ-WL1 (is Wales a target deployment?) and OQ-WL2 (the licence of the All-Wales nursing forms) in [`spec/16-open-questions.md`](spec/16-open-questions.md).

### 34.6 Not done, not verified

No system was used. The Welsh GP Record FAQ, the January 2025 WICIS release and the Digital Health articles could not be opened, so what they say rests on search snippets. No clinician read the ideas. The Welsh Control Standard and the Wales FHIR guide were not opened. Sources are listed in the spec page.
