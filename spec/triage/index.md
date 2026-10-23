# Emergency triage

Post-1.0 (OQ-T2, decided 2026-10-23). Tasks `TR-1` to `TR-13` in [`tasks.md`](../../tasks.md); research in
[`plan.md`](../../plan.md) §15. **Planned; nothing here is built.** NOT FOR CLINICAL USE.

Every item on this page is clinical: a wrong triage changes who a clinician sees first.

## 1. Content policy

No Manchester Triage System (MTS) flowchart, discriminator or wording is in this repository, in tests or in demo
data, until the licence is settled (OQ-T1, TR-1). The project ships the **framework**: the pack interface and a
synthetic pack that is clearly not MTS. The deployer licenses MTS content from its publisher. The project does not
claim MTS accreditation or conformance.

## 2. Model

- **Triage pack.** A versioned set of flowcharts, each an ordered list of discriminators, with a category and a
  target time per category. A pack declares its system (`mts`, `synthetic`, later others), its version and its
  edition. An unknown, invalid or unsigned pack is refused loudly.
- **Triage assessment.** An append-only record on an `Encounter`: pack and version, flowchart, the discriminator that
  fired, the suggested category, the confirmed category, assessor (a reference, never a copy of a person), time and
  the clock start. Nothing updates or deletes an assessment.
- **Discriminator answers** are `present`, `absent` or `not-assessed`. `not-assessed` carries a reason, in the same
  way as `dataAbsentReason`. A `not-assessed` discriminator is never read as `absent`, so a missing assessment never
  lowers the category.
- **Suggested versus confirmed.** The system suggests the category of the first applicable discriminator. The
  clinician confirms it or overrides it with a reason. The system never saves a suggestion as a triage, and never
  changes a confirmed category.
- **Re-triage.** A change of category is a new assessment that links to the one it supersedes, with a reason.
- **Time targets** run from the clock start. A breach raises one signal per patient, and an audit event.

## 3. FHIR mapping (OQ-T3)

Checked against search summaries only; the UK Core and US Core profile pages were not opened (**[uncertain]**).

| Concept | FHIR R4 element | Basis |
|---|---|---|
| Patient has been triaged | `Encounter.status = triaged` | In the R4 status value set; it is a state, not an acuity |
| Confirmed category | `Observation` (the assessment), `valueCodeableConcept` from the pack's own code system | `Encounter.priority` is 0..1 with only an example binding to ActPriority, so it cannot carry five clinical categories safely |
| Category on the encounter | `Encounter.priority`, **copied** from the latest Observation as a convenience for lists | Optional; the Observation is the source |
| Flowchart and discriminator | `Observation.component` | Keeps them with the assessment |
| Assessor, pack, override reason | `Provenance` on the Observation | Matches the rest of the record |
| Supersedes | `Observation.derivedFrom` or `Provenance.entity` | **[uncertain]** which; decide when building TR-5 |

- **Code system.** LOINC has a code for the Emergency Severity Index (75636-1, from a search result **[uncertain]**).
  I found no LOINC or SNOMED CT code for an MTS category. The Welsh NHS data dictionary has a local triage score
  (01 immediate to 05 non-urgent). Until a national code is confirmed, the pack publishes its own `CodeSystem`
  under this project's canonical URL, and a mapping to ESI or a national code is a later `ConceptMap`.
- UK Core Encounter has emergency care extensions (discharge status, method, outcome of attendance). I found none
  for triage category, and no published NHS England FHIR profile for the Emergency Care Data Set **[uncertain]**.
- Do not map MTS categories to ESI levels automatically. A published comparison aligns them only roughly.

## 4. Rules

1. The clinical record is append-only. There is no delete path for a triage.
2. Every read and write is audited, and a triage is subject to consent and access control like any other record.
3. A triage never changes an order, a score or a patient link. NEWS2 or a paediatric score is shown beside the
   category and may prompt, never change it (TR-9).
4. The wording says "suggested", and the UI asks the nurse to confirm. A tool that triages by itself may be a
   medical device (AS-1, OQ-6).
5. A triage stamps its pack version, so a record stays readable after the pack is updated or withdrawn.
6. Flowchart text is clinical content. Translations stay machine-written until a speaker reviews them.

## 5. Not decided

- Which element links a re-triage to the one it replaces (§3).
- Whether `Encounter.priority` is copied at all, or the board reads the Observation.
- The code system for the confirmed category, until a national or LOINC code is checked.
- Whether MTS may be encoded here (OQ-T1).
