# Order entry: lab, imaging, referral and medication orders

Public-source research read 2026-11-14 plus this repository's own files. A proposal and a reading, not clinical advice: nothing here is built, and no clinician has reviewed it.

Method: web searches and fetches (some full pages, many search snippets; "Not verified" says which) and the project's specs: [domain model](../05-domain-model.md) (5.2.1 pathway first), [functional requirements](../06-functional-requirements.md) (FR-6.1 to FR-6.6), [implementation status](../14-implementation-status.md), the [service spec](../../electronic-patient-record-service-with-rust/spec/index.md) (results with a version-named sign-off, referrals with a state table, timeouts and a worklist, the HL7 v2 ADT feed), the [closed-loop referral](../interoperability/closed-loop-referral.md) page, [measurement](../clinical-safety/measurement.md), and [`tasks.md`](../../tasks.md) entries OR-1 to OR-7, RC-10, EM-2 and EM-6. Style follows the [virtual ward](../virtual-ward/index.md) page. The companion for notes is [clinical documentation](../clinical-documentation/index.md). Inpatient first.

## What it is

Order entry (computerised provider order entry, CPOE) is how a clinician asks for something to be done for a patient and is told it was done: a blood test, an image, a referral, a medicine, a nursing task. An order has a life: drafted, signed, sent, accepted or refused by the department that fills it, done, resulted, and the result seen by the person who asked. Most harm in this area is in the gaps: an order on the wrong patient, an order never sent, a result nobody owns.

- **Order kinds.** Laboratory, imaging, referral (built separately, see below), nursing and other tasks, and medication orders. Inpatient medication ordering is the drug chart (electronic prescribing and medicines administration); it shares the order life but brings dose, route, frequency, administration and reconciliation (EM-2, RC-10).
- **Order sets.** A named, versioned group of orders with defaults for a situation (for example a chest pain admission), applied as drafts.
- **Decision support at ordering.** Checks when an order is chosen and again at signing: allergy, duplicate, interaction, dose, and wrong-patient guards.
- **Results back to the orderer.** The result links to its order, and its sign-off goes to the orderer or their team.

## Evidence and how others do it

- **CPOE reduces prescribing errors.** A meta-analysis found CPOE cut the likelihood of an error on a medication order by 48% (95% CI 41% to 55%); applied to US hospital adoption in 2008, that was an estimated 12.5% fewer medication errors, about 17.4 million a year. The authors say it is unclear whether fewer errors means less harm [RADLEY2013] (abstract).
- **CPOE also causes errors.** A study of one system at a teaching hospital found 22 kinds of medication error risk that the system made easier, from fragmented displays to interface problems; most house staff saw each weekly or more. Critics noted it measured perceptions, not error rates [KOPPEL2005] (abstract and press).
- **Systems miss dangerous orders.** In a simulation test hospitals run on their own systems, 39% of potentially harmful drug orders were not flagged in 2015 data, and 13.9% of potentially fatal ones in 2013 and 2014 [LEAPFROG2015]. A ten-year analysis of the same test found systems met basic safety standards less than 70% of the time [CLASSEN2020] (trade-press summary; the paper not read).
- **Wrong-patient orders.** The retract-and-reorder measure flags an order placed, retracted within minutes, and placed by the same clinician on another patient; 170 of 223 flagged events were confirmed wrong-patient orders (positive predictive value 76.2%). In a randomised trial of 901,776 ordering sessions, an identity re-entry step reduced such events more (odds ratio 0.60) than an identity-verify alert (0.84) [ADELMAN2013] (abstract). A later trial of 12.1 million orders found limiting clinicians to one open record did not reduce wrong-patient order sessions, though few unrestricted users had several open [ADELMAN2019].
- **Alerts are mostly overridden.** A 2006 review found drug safety alerts overridden in 49% to 96% of cases [VDSIJS2006] (the range is quoted by later reviews; the table was only partly seen). A later review across 23 articles gives 46.2% to 96.2% [OVERRIDE2020] (abstract).
- **Tiering helps.** At two centres sharing one interaction service, the site that tiered alerts by severity (hard stops for the most severe) had 29% acceptance against 10%, and 100% against 34% for the most severe [PATERNO2009]. Observational, two sites.
- **Hard stops cut both ways.** A systematic review of 32 studies found hard stops beat soft stops in three of four comparisons but caused delays to care, workarounds and more alerts [POWERS2018] (abstract).
- **Self-assessment.** The US national coordinator's SAFER guides, revised 2025, include one for CPOE with decision support, rated by the organisation on a five-point scale [SAFER2025] (search summary).
- **Elsewhere.** These are mostly US studies. The [equity rule](../02-scope.md) applies: order entry must work where the laboratory has no interface (a printed or exported request) and where no drug knowledge base is licensed (only the built-in allergy and duplicate checks, labelled as such).

## Standards

- **FHIR R4 `ServiceRequest`** for lab, imaging, referral and nursing orders. Status: `draft`, `active`, `on-hold`, `revoked`, `completed`, `entered-in-error`, `unknown`. Intent: `proposal`, `plan`, `directive`, `order`, `original-order`, `reflex-order`, `filler-order`, `instance-order`, `option`. Priority: `routine`, `urgent`, `asap`, `stat`. `requisition` groups orders signed together by one author; `basedOn` points at a `CarePlan`, `ServiceRequest` or `MedicationRequest`; `replaces` names the order this one supersedes [FHIR-SR].
- **FHIR R4 `MedicationRequest`.** Status: `active`, `on-hold`, `cancelled`, `completed`, `entered-in-error`, `stopped`, `draft`, `unknown`. It has `groupIdentifier`, `priorPrescription`, `detectedIssue` (for interactions and duplicates) and `eventHistory` (Provenance of state changes) [FHIR-MR].
- **FHIR R4 `RequestGroup` and `PlanDefinition`.** A RequestGroup is "often, but not always, the result of applying a specific PlanDefinition to a particular patient", and shares its action structure; an order set is a `PlanDefinition`, and its application to a patient a `RequestGroup` of draft requests [FHIR-RG]. In R5 the resource is `RequestOrchestration` [FHIR-RO] ([uncertain]: the R5 page read did not state the rename).
- **FHIR R4 `Task`** tracks the doing of a request; a ServiceRequest may authorise a Task or be what a Task fulfils [FHIR-SR] [FHIR-TASK].
- **FHIR R4 `DiagnosticReport`** carries the result, with `basedOn` pointing back at the ServiceRequest [FHIR-DR].
- **CDS Hooks.** `order-select` fires "after the clinician selects the order and before signing", with `selections` and `draftOrders` (maturity 4) [CDSH-SELECT]. `order-sign` fires when the clinician is ready to sign, also on re-signing revised orders, with `draftOrders` as a Bundle of R4 `MedicationRequest`, `ServiceRequest` or `DeviceRequest` (maturity 5) [CDSH-SIGN]. Cards have indicator `info`, `warning` or `critical`; the feedback endpoint records `accepted` or `overridden`, with an optional coded `overrideReason` and comment; services "SHOULD respond quickly (on the order of 500 ms)" and the spec does not say what a client does on timeout [CDSH-2].
- **HL7 v2.** `ORM^O01` (general order) and `OML^O21` (laboratory order) out, `ORU^R01` (result) back. `ORC-1` carries the order control code: `NW` new, `CA` cancel request from the placer, `OC` cancelled notice from the filler, `SC` status changed; `ORC-2` and `ORC-3` are the placer and filler order numbers [HL7V2-CH4] [NIST-ORC]. A cancel is a request that only the filler confirms, as in the built referral table.

## Workflows

Each step names what the clinician needs on screen. Inpatient ward first.

1. **Open the patient.** Banner: name, birth date, national health number, location and bed, allergies summary, weight with its date (or "not recorded"). Data: identity and allergies (built); weight is an observation (built kind, not yet in the banner).
2. **Start an ordering session.** The session is bound to one patient. Switching patient with unsigned drafts asks what to do with them; drafts never move to another patient.
3. **Find an order** from a catalogue (searchable, coded, each entry with its specimen, container, fasting or preparation note, and whether the receiving department takes it electronically). Or apply an order set: its name, version, review status, and each order as an unticked or ticked draft the clinician can change.
4. **Fill in details.** Priority, reason (coded where possible, linked to a problem or the note), clinical question for imaging, timing, specimen. Show "already ordered" when the same test is active or was resulted within its window, with the result if one exists.
5. **Order-select checks** run on each chosen order: allergy (class level via terminology), duplicate, and any external service. Cards are passive unless high severity and actionable.
6. **Link to the pathway.** Each order is a step on a pathway, or "not on a pathway" (5.2.1 rule 2). Default: the pathway of the admission.
7. **Review and sign.** The sign screen lists every draft in the session, the patient's name and birth date in large type, and asks for identity confirmation when the user switched patients in the last N seconds (OR-1), for example by typing the patient's initials and birth year (the re-entry form that worked best in [ADELMAN2013]).
8. **Order-sign checks.** Interruptive cards need an action; an override needs a reason (coded list plus comment) and is stored. If the external service fails or times out, signing proceeds, the screen says checks were not run, and the order records "external checks not run: timeout" (OR-5 fail-open, a hazard to log).
9. **Send.** Each signed order goes to its receiving department (electronic message, or a printable request where none exists) and shows `sent`, then `accepted` or `refused` when the filler answers. An unanswered order is listed after a configured time, as unanswered referrals are (PC-3).
10. **Specimen and doing.** Specimen collection against the order (EM-6), or the image done. A nursing order becomes a task on the ward list.
11. **Result.** The result arrives (`ORU^R01` or filed by hand, as today) linked to its order. It goes to the orderer's sign-off list, or to the team's, or to a fallback queue when the orderer has left (OR-4). Abnormal first.
12. **Sign off and act.** The orderer signs off the result version (built), and the order is `completed`. The pathway step closes only when the result is signed off, not when it arrives.
13. **Change or stop.** A signed order is never edited: a change is a new order that `replaces` it, and a stop is a cancel request the filler confirms. An order placed on the wrong patient is marked `entered-in-error` with a reason, and the filler is told.

Medication orders (after EM-2 and RC-10, not in the first slice): dose, route, frequency, duration, indication, and the drug chart view; administration against the order; reconciliation at admission and discharge.

## Rules

Each is a testable sentence.

1. An order is stored as append-only versions; a database trigger refuses `UPDATE`, `DELETE` and `TRUNCATE`.
2. An order's states are `draft`, `active` (signed), `on-hold`, `completed`, `revoked` and `entered-in-error`; a transition not in the state table is refused.
3. A signed order cannot be edited; a change is a new order whose `replaces` names the old one, and the old one is revoked in the same transaction.
4. Signing names the version signed and the patient id; a mismatch with the session's patient is 409.
5. Sign requires identity re-entry when the user's last patient switch was within the configured seconds; without it the sign is refused.
6. Applying an order set creates draft orders only; none is signed automatically.
7. An order set and a catalogue entry are data with an id, version, approver and day; an order records which version it came from.
8. Every interruptive card shown at signing has a stored outcome: `accepted` or `overridden`, and an override without a reason is refused for cards marked as needing one.
9. A failed or timed-out external check never blocks signing, and the order records that the check was not run and why; the absence is never shown as "no alerts".
10. An allergy recorded as active to a drug class fires on an order for a drug in that class (penicillin allergy on an amoxicillin order).
11. A duplicate check fires when the same coded test is active or resulted within its configured window; a test with no configured window does not claim to be checked.
12. A cancel of a sent order is a request; the order shows `cancel-requested` until the filler confirms.
13. A result whose order exists is linked to it; a result for an order with no current clinician goes to the fallback queue, never nowhere.
14. A pathway step for an order closes only when its result is signed off or the order is revoked with a reason.
15. Every read and state change is audited; the audit row holds ids, versions, states and counts, never the test, value or drug.
16. The retract-and-reorder count (an order revoked within a configured window and the same coded order placed by the same user on another patient within the window) is computable from stored data.

## Hazards to add when built

Described here, not added to the [hazard log](../../governance/clinical-safety/hazard-log.csv); each goes in with the first code (AS-1).

- **OE-H1 Order on the wrong patient.** Controls: one patient per session, re-entry at sign, retract-and-reorder monitoring.
- **OE-H2 Order signed but never sent or never received.** Controls: `sent` and `accepted` states, unanswered-order worklist.
- **OE-H3 Result nobody sees.** Controls: result linked to order, routing, fallback queue, step closes only on sign-off.
- **OE-H4 Alert missed or ignored.** Fatigue from passive noise hides a critical card. Controls: tiering, override reasons, analytics (OR-7).
- **OE-H5 Check silently not run.** An external service down is read as "no interactions". Controls: rule 9.
- **OE-H6 Order set default wrong for this patient.** Controls: drafts only, defaults visible, version and review status shown.
- **OE-H7 Cancelled order still done** (or a done order believed cancelled). Controls: rule 12.
- **OE-H8 Hard stop delays urgent care.** Controls: hard stops only where reviewed; an emergency route with reason and audit.

## First slice

Inpatient laboratory orders only, filled by hand, results filed against them. No medicines, no imaging, no outbound message, no external CDS.

- **Store.** Table `order_versions` (patient id, order id, version, kind `lab`, catalogue entry id and version, code, priority, reason, status, intent `order`, requisition id, replaces, pathway step or `not-on-pathway`, author, signed version, reason for change, created at) and view `order_current`; append-only by trigger. A `result_versions` row gains an optional order id (a migration that only adds a column and a view, no delete path). Catalogue as data, like `src/assessment_forms.toml`.
- **Routes.** `GET /api/patients/{id}/orders`, `POST /api/patients/{id}/orders` (drafts, one requisition), `PUT …/orders/{order_id}` (drafts only, `expected_version`), `POST …/orders/sign` (`{requisition, versions, patient_id, identity_check}`), `POST …/orders/{order_id}/revoke` (`{reason}`), `POST …/orders/{order_id}/entered-in-error` (`{reason}`). Policy resource `ServiceRequest` (clinical). No `/api/v1`.
- **Screen.** An "Orders" section on the patient page: catalogue search, draft list, sign screen with the patient named and the re-entry box, and each order's state; results filed against an order show on it.
- **Tests.** Rules 1 to 6, 10 (with the built allergy store and the terminology fixture), 13 and 15; a wrong-patient guard test with a fake clock; sign p95 measured (OR-1 asks for 2 s or less); a census test that every order route is audited.

## What this repo has and lacks

| Need | Today | Gap |
|---|---|---|
| A request with a state table, timeouts and a worklist | Built for referrals (CA-3, PC-1 to PC-3) | Not for orders; the pattern can be reused |
| Results with a version-named sign-off | Built (OR-4 first slice) | No link to an order; no routing; no inbox across patients |
| HL7 v2 | ADT inbound with a refusal queue | No ORM/OML out, no ORU in |
| Allergy record and terminology | Built (allergies; terminology fixture with a pinned version) | No drug classes; no drug-allergy check |
| Catalogue as reviewed data | Pattern built (assessment forms, suspicion pathways) | No order catalogue or order sets |
| Pathway steps | Designed (5.2.1) | No pathway store |
| CDS Hooks client | None | OR-5 |
| Medicines | None | EM-2, RC-10, RC-5 |

## Not verified

- Most study figures come from abstracts, press releases or trade summaries, not full papers [RADLEY2013] [KOPPEL2005] [ADELMAN2013] [CLASSEN2020] [VDSIJS2006] [OVERRIDE2020] [POWERS2018].
- The 49% to 96% override range is attributed to [VDSIJS2006] by later reviews; [uncertain] that the range is in that paper as stated.
- The SAFER CPOE guide's recommendations were not read; only that it exists and how it is rated [SAFER2025].
- The CDS Hooks `order-sign` page lists hook version 1.0 while its change log lists later versions; which applies is [uncertain] [CDSH-SIGN].
- Whether `OML^O21` was introduced in v2.4 or v2.5 is [uncertain]; sources differ [HL7V2-CH4].
- UK Core and US Core profiles for ServiceRequest and MedicationRequest were not checked.
- No clinician, pharmacist, laboratory scientist or lawyer has reviewed any of this.

## Sources

- [RADLEY2013] Radley et al. Reduction in medication errors in hospitals due to adoption of computerized provider order entry systems. JAMIA 2013: https://pmc.ncbi.nlm.nih.gov/articles/PMC3628057
- [KOPPEL2005] Koppel et al. Role of computerized physician order entry systems in facilitating medication errors. JAMA 2005: https://pubmed.ncbi.nlm.nih.gov/15755942/ ; criticism: https://healthcareitnews.com/node/399721
- [LEAPFROG2015] The Leapfrog Group. Technology to prevent medication errors fails too often (2015 release): https://www.leapfroggroup.org/node/290 ; fact sheet: https://ratings.leapfroggroup.org/sites/default/files/2026-04/2026%20CPOE%20Fact%20Sheet.pdf
- [CLASSEN2020] Classen et al., ten-year CPOE evaluation tool results (summary): https://www.hcplive.com/view/ehrs-fail-detect-1-3-harmful-drug-interactions-medication-errors
- [ADELMAN2013] Adelman et al. Understanding and preventing wrong-patient electronic orders: a randomized controlled trial. JAMIA 2013: https://journalclub.wustl.edu/2013/08/13/understanding-and-preventing-wrong-patient-electronic-orders-a-randomized-controlled-trial/ ; later use of the measure: https://pmc.ncbi.nlm.nih.gov/articles/PMC4447590
- [ADELMAN2019] Adelman et al. Effect of restriction of the number of concurrently open records on wrong-patient order errors. JAMA 2019: https://psnet.ahrq.gov/issue/effect-restriction-number-concurrently-open-records-electronic-health-record-wrong-patient ; trial: https://clinicaltrials.gov/study/NCT02876588
- [VDSIJS2006] van der Sijs et al. Overriding of drug safety alerts in computerized physician order entry. JAMIA 2006: https://pmc.ncbi.nlm.nih.gov/articles/PMC1447540
- [OVERRIDE2020] Systematic review of alert override rates (46.2% to 96.2%): https://www.pubmed.gov/32706721/
- [PATERNO2009] Paterno et al. Tiering drug-drug interaction alerts by severity increases compliance rates. JAMIA 2009: https://pmc.ncbi.nlm.nih.gov/articles/PMC2605599
- [POWERS2018] Powers et al. Efficacy and unintended consequences of hard-stop alerts in EHR systems: a systematic review. JAMIA 2018: https://pmc.ncbi.nlm.nih.gov/articles/PMC6915824
- [SAFER2025] US Office of the National Coordinator for Health IT. SAFER guides: https://healthit.gov/topic/safety/safer-guides
- [CDSH-SELECT] CDS Hooks, order-select: https://cds-hooks.org/hooks/order-select/
- [CDSH-SIGN] CDS Hooks, order-sign: https://cds-hooks.org/hooks/order-sign/
- [CDSH-2] CDS Hooks specification 2.0 (cards, feedback): https://cds-hooks.hl7.org/2.0/
- [FHIR-SR] HL7 FHIR R4 ServiceRequest: https://hl7.org/fhir/R4/servicerequest.html
- [FHIR-MR] HL7 FHIR R4 MedicationRequest: https://hl7.org/fhir/R4/medicationrequest.html
- [FHIR-RG] HL7 FHIR R4 RequestGroup: https://hl7.org/fhir/R4/requestgroup.html
- [FHIR-RO] HL7 FHIR R5 RequestOrchestration: https://hl7.org/fhir/R5/requestorchestration.html
- [FHIR-TASK] HL7 FHIR R4 Task: https://hl7.org/fhir/R4/task.html
- [FHIR-DR] HL7 FHIR R4 DiagnosticReport: https://hl7.org/fhir/R4/diagnosticreport.html
- [HL7V2-CH4] HL7 v2.6 chapter 4, order entry (mirror): https://www.hl7.eu/HL7v2x/v26/std26/ch04.html
- [NIST-ORC] NIST HL7 v2+ demo, ORC segment: https://usnistgov.github.io/v2plusDemo/segment-definition/ORC.html
