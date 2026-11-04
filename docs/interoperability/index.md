# Interoperability: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

## Definition

In ISO/IEC 25010:2023 interoperability is a sub-characteristic of compatibility: the degree to which two or more systems, products or components can exchange information and use the information that has been exchanged [1]. I saw an iso25000.com summary, not the standard's text (the ISO page gave HTTP 403) [2]. The second half matters for an EPR: a message that parses but whose meaning is lost (a code with no mapping, a missing unit) has been exchanged but not used.

Healthcare work usually splits the idea into layers: transport, syntax (HL7 v2, FHIR JSON, CDA), semantics (terminologies such as SNOMED CT, LOINC) and process (who does what with it). This note uses those four layers; the layering is a common convention, not a quotation from 25010. It is a separate document here because it is central to an EPR. See [compatibility](../compatibility/index.md) for co-existence and versions.

## Why it matters for an EPR

HL7 v2 is everywhere and varies by site. The HL7 FHIR comparison page calls variation in implementation one of the biggest challenges of v2 interoperability, and says Z-segments are opaque without a prior explanation from the sender [3]. So an inbound ADT feed needs a tolerant, tested parser and a mapping agreed per sender.

FHIR is the modern route, but a server's claim must be true. FHIR requires a CapabilityStatement stating the interactions and resources a server supports [4], and the US ONC (g)(10) certification is tested by the Inferno test kit, which covers SMART App Launch, US Core and Bulk Data and had a version 8.0.5 listed in the search results I saw [5]. For the UK, UK Core is published by NHS England and can be checked with the HL7 validator with the package loaded, according to community threads from 2020 to 2022 that I saw only as snippets [6]. Bulk Data Access is on version 3.0.0 (STU 3) over R4 according to its home page [7].

Failures of exchange become safety events when a record cannot be found at the point of care. The NHS and other health systems have lost access to records when central services fell over (October 2022 Oracle Cerner outage, reported by The Register [8]; Welsh GPs unable to reach records because of data-centre faults [9]; both snippets only). I did not find a public source I could read that measures harm from failed integrations specifically, so I make no number claim.

## What the project says today

- **Standards plan** (`spec/interoperability/index.md`): FHIR R4 with UK Core and US Core, IPS, SMART App Launch 2.2, CDS Hooks 2.0, Bulk Data 2.0, HL7 v2.5.1 inbound (ADT, ORU, SIU), C-CDA only if a US network requires it. Tasks IO-1 to IO-19, OR-3.
- **Built:** `/fhir/R4/metadata`, `Patient`, `AllergyIntolerance`, read-only `Observation` search; a census test that the stated interactions equal the mounted routes (IO-1); `interop::fhir_patient` and `fhir_allergy` via the `fhir` crate; a read-only R5 facade, where conversion loss fails the response (IO-16); an HL7 v2 reader for ADT A01, A04, A08, A28 and A31 with `AA`/`AE`/`AR` acknowledgements, and an MLLP listener that is off unless `EPR_MLLP_BIND` is set (OR-3); an OpenAPI 3.1 document derived from the routes (OS-7).
- **Test oracle:** the HL7 FHIR validator, in `tests/fhir_validator.rs` and the CI job `fhir-validator`, with a control that must fail. This proves only what the validator checks.
- **Not built:** SMART App Launch (IO-2), conformance via Inferno (after IO-2 and IO-4), Bulk `$export` (IO-4), IPS `$summary` (IO-5), Subscriptions (IO-6), SQL-on-FHIR (IO-7), EHI export (IO-8), NHS national APIs (IO-9), SMART Health Links (IO-10), national profile packs (IO-18, a design only: OS-10 "Pack mechanism", proposed and not confirmed by the maintainer), outbound HL7 or an outbox sink (OS-3, partly landed; no worker).
- **Honesty rules** in the spec: `metadata` lists only profiles that pass validation; the stored record is base FHIR; no pack changes the clinical rules; packs are unreviewed until a person who knows the country reviews them.
- **Benchmark** (`spec/benchmark.md`): SMART App Launch is a table-stakes gap among active comparators; OpenAPI and the CapabilityStatement census are in place.

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
|---|---|---|---|
| Validator errors on written FHIR | HL7 validator over every resource the service writes, per claimed profile | Zero errors; a control resource must fail | `tests/fhir_validator.rs`, CI `fhir-validator` |
| Capability statement truth | Interactions stated versus routes mounted | Zero mismatch | IO-1 census test |
| External conformance | Inferno suites (SMART, US Core, Bulk) pass count | Pass all tests of the claimed profiles; do not claim certification | not built (after IO-2, IO-4) |
| HL7 v2 acceptance | Corpus of real-shaped ADT messages from several vendors: accepted, rejected with `AE` or `AR` and why | 100% of the corpus gets a defined outcome; none silently dropped | `hl7_adt.rs` tests; corpus not built |
| Round-trip loss | Resource to FHIR and back; R4 to R5 loss or warning | Zero silent loss | IO-16 tests |
| Semantic mapping coverage | Share of incoming codes mapped to a declared code system; unmapped kept as-is with reason | Never invent a mapping; report unmapped count | terminology work (`spec/terminology`) |
| Integration lead time | Days from a partner's request to a working connection in test | Record per partner; set a target after three | not collected |
| Message reliability | Messages received, acknowledged, retried, dead-lettered | Zero lost; each failure visible | outbox events and attempts (OS-3), `/metrics` |
| Pack validity | Packs passing their typed schema and validator | Each claimed pack passes with a failing control | pack loader tests (phone pack built) |

## Trade-offs with other attributes

- **Security and privacy.** Every exchange path needs consent and policy checks before data leaves; outbox design checks policy and consent before a sink ([security](../security/index.md), [privacy](../privacy/index.md)).
- **Safety.** A wrongly matched inbound patient is the highest hazard; absent evidence blocks an automatic link ([safety](../safety/index.md)).
- **Auditability.** Inbound and outbound events must be audited without a flood of rows ([auditability](../auditability/index.md)).
- **Performance.** Bulk export and subscriptions load the same database as clinical reads ([performance efficiency](../performance-efficiency/index.md)).
- **Maintainability and flexibility.** Every extra standard or country is a pack to review and keep current ([maintainability](../maintainability/index.md), [flexibility](../flexibility/index.md)).
- **Functional suitability.** A read facade that returns less than the record is honest only if it says what it omits ([functional suitability](../functional-suitability/index.md)).

## Gaps and risks

1. No SMART, Bulk, Subscriptions, outbound HL7 or national API, so the system cannot yet join a real exchange.
2. Conformance rests on the HL7 validator alone; no external kit has run.
3. No multi-vendor HL7 v2 corpus; the parser may be strict or lenient in the wrong places.
4. No terminology service mapping (FR-3.4), so semantic interoperability is untested.
5. Country lists in the spec were written from the author's knowledge and not checked against each publisher (IO-18).
6. Standards move (US Core tracking 8.x, Bulk 3.0.0, R6 ahead); version drift is a standing cost.
7. Any exchange failure mode that leaves a clinician without a record is a hazard row that does not yet exist for each path (AS-1).

## Ideas

1. Collect a public, synthetic HL7 v2 corpus (ADT from several vendors, with Z-segments) and make it a fuzz and regression set (links to the planned HL7 v2 fuzzing in `spec/11-testing-strategy.md`).
2. Run the Inferno suites in CI once IO-2 and IO-4 exist; publish the pass list, not a certification claim.
3. Add an "unmapped code" report to inbound handling so semantic loss is visible.
4. Write an integration scenario per partner in six-part form (source, stimulus, response, measure) before building each interface (IO-9).
5. Add a hazard row per exchange path (inbound ADT, FHIR read, export) with a downtime fallback (FR-8.7 downtime snapshot).
6. Check the country list in the spec against each publisher before building packs (IO-18).
7. Add `Accepts-version` and `fhirVersion` negotiation tests (IO-17, PA-9).
8. Publish a conformance statement page generated from the CapabilityStatement census.

## Open questions

- Which partner systems come first: a lab, a PAS, a national service?
- Which FHIR search parameters are promised (RC-2) and at what limits?
- Does the project want to certify against any national scheme, or only publish test results?
- Who reviews each country pack?

## Not verified

- ISO/IEC 25010:2023 text.
- Inferno version numbers (the search results disagreed).
- UK Core validation guidance (2020 to 2022 community posts, snippets); check NHS England directly.
- Bulk Data details beyond the version on its home page.
- The outage reports (snippets only).
- That the specs' "built" statements match the code; I read the specs, not the code.
- Nothing here was measured.

## Sources

1. https://iso25000.com/index.php/en/iso-25000-standards/iso-25010 : interoperability definition in 25010 (summary).
2. https://www.iso.org/standard/78176.html : ISO catalogue page (403 on fetch).
3. https://www.hl7.org/fhir/R4B/comparison-v2.html : HL7 v2 variation and Z-segments (snippet).
4. https://hl7.org/fhir/R4/http.html : CapabilityStatement and `fhirVersion` rules.
5. https://inferno.healthit.gov/suites : Inferno test kits (snippet; also https://www.github.com/onc-healthit/onc-certification-g10-test-kit).
6. https://chat-archive.fhir.org/stream/179189-uk/topic/Validating.2FTesting.20FHIR.20in.20the.20UK.20-.20Tools.html : UK Core and the HL7 validator (snippet).
7. https://hl7.org/fhir/uv/bulkdata/ : Bulk Data Access 3.0.0 over R4 (home page only).
8. https://www.theregister.com/2022/10/12/oracle_cerner_outage_uk/ : Oracle Cerner UK outage (snippet).
9. https://www.datacenterdynamics.com/news/welsh-gps-unable-to-access-patient-records-due-to-technical-issues-affecting-nhs-data-centers/ : Welsh GPs without records (snippet, undated).
