# Medication administration: the inpatient eMAR and the closed loop

Public-source research read 2026-11-14 plus this repository's own files. A proposal and a reading, not clinical advice: nothing here is built, and no clinician or pharmacist has reviewed it.

Method: web searches and fetches (the FHIR R4 MedicationAdministration page read in full; the rest mostly abstracts and search snippets, marked "snippet") and the project's own files: [domain model 5.2.1](../05-domain-model.md), [device readings](../device-readings/index.md), [virtual ward](../virtual-ward/index.md) (for shape), [implementation status](../14-implementation-status.md), the [service spec](../../electronic-patient-record-service-with-rust/spec/index.md), [open questions](../16-open-questions.md) (OQ-M1, OQ-M3, OQ-6), and tasks EM-2, OR-6, RC-5, RC-10 and EM-11 in [tasks.md](../../tasks.md). The companion page is [nursing flowsheets](../nursing-flowsheets/index.md): the eMAR is a flowsheet whose rows are prescribed doses.

## What it is

An **electronic medication administration record (eMAR)** is the ward's chart of what is due, given, not given and why, for each prescribed medicine. A **closed loop** joins four steps so that each checks the one before:

1. **Prescribe.** A prescriber writes an order: medicine, dose, route, frequency or times, start, review or stop, indication, and for "as needed" (PRN) doses the reason and the maximum in 24 hours.
2. **Verify.** A pharmacist screens it (allergy, interaction, dose, renal function, duplication) and marks it verified, or queries it.
3. **Dispense.** Pharmacy or ward stock supplies the medicine, ideally barcoded to the unit dose.
4. **Administer.** A nurse identifies the patient (wristband scan), scans the medicine, sees the order beside what was scanned, gives it, and records it at the time it was given; or records it not given, with a reason.

The "rights" are the checks at step 4. The traditional five are right patient, drug, route, time and dose; one review proposed nine, adding right documentation, indication, response and form ([Elliott and Liu 2010](#s-elliott)) (abstract and secondary summaries; the full nine not confirmed).

## How others do it, and what the evidence says

- **Barcode eMAR reduces errors where order entry already exists.** At one US academic centre, units with barcode eMAR had non-timing administration errors fall from 11.5% to 6.8% (a 41.4% relative fall), potential adverse drug events from 3.1% to 1.6%, timing errors by 27.3%, and transcription errors from 6.1% to none ([Poon 2010](#s-poon)) (abstract). The hospital already had order entry and pharmacy barcoding; the events were potential, not actual.
- **Workarounds are common and come from design.** Across five hospitals nurses overrode barcode alerts for 4.2% of patients and 10.3% of medicines charted; causes included unreadable barcodes and failing scanners, and the authors traced workarounds mainly to design, implementation and workflow ([Koppel 2008](#s-koppel)) (abstract). A 2025 study of five London wards found medicine scanning from 5.6% to 67% and patient scanning from 4.6% to 89% by ward, falling over time, with unreadable barcodes and missing scanners the usual reasons ([UCL 2025](#s-ucl-scan)) (snippet). A health system that tracked overrides per unit reported overrides sustained below 2% ([override study](#s-override2)) (snippet).
- **Interruptions cause errors.** Nurses in two Sydney hospitals were interrupted in 53.1% of administrations; each interruption added 12.7% to clinical errors, and the risk of a major error rose from 2.3% with none to 4.7% with four ([Westbrook 2010](#s-westbrook)) (abstract and press summaries). A drug round screen should hold a nurse's place and resume.
- **Omitted and delayed doses.** A national agency reviewed 18,527 incidents (2006 to 2009) of omitted or delayed medicines, named critical medicine groups, and asked hospitals to define their own list and audit omissions ([NPSA 2010](#s-npsa)) (snippet; the alert is no longer active). A national pharmacy service now runs a time-critical medicines programme to cut delayed and omitted doses ([SPS](#s-sps)) (snippet).
- **Parkinson's medicines.** A 2022 audit cited by Parkinson's UK found 58% of people with Parkinson's admitted to hospital did not get their medicine on time; the charity asks for electronic prescribing to be used to monitor missed and delayed doses ([Parkinson's UK](#s-parkinsons)) (snippet).
- **Controlled drugs.** Ward practice, set by local policy under national guidance, usually requires two registered staff from selection to administration, both signing the ward register with the time; administration without a witness is a policy breach in at least one published policy ([a Scottish board's guideline](#s-cd-borders), [an English trust's SOP](#s-cd-wwl)) (snippet). The witness rule is policy, not statute [uncertain]; NICE NG46 asks for a risk assessment on whether Schedules 3 to 5 are handled like Schedule 2 ([NICE NG46](#s-nice-ng46)) (snippet).
- **Infusions.** Linking smart pumps to the record (auto-programming from a scanned order) reduced administration errors 15.4% to 54.8% for errors it could affect in a three-study review; errors remained ([Borrelli 2025](#s-pump-review)) (snippet). Pump integration is device territory (EM-11, OQ-6), post-1.0.
- **Open source.** Bahmni's inpatient drug chart takes a doctor's order onto a nurse-scheduled chart by shift, ends in "Stop Drug", and records emergency doses with an "EMERG" tag followed by a prescriber's acknowledgement ([Bahmni](#s-bahmni-mar)) (snippet).
- **Barcodes.** GS1 DataMatrix on the medicine carries the GTIN with batch and expiry; the patient wristband carries a GSRN; England's Scan4Safety programme uses both at the bedside ([GS1 patient ID](#s-gs1-patient), [GS1 UK](#s-gs1uk)) (vendor material).

## Standards

FHIR R4 (to be checked against the profiles a deployment uses):

- **MedicationRequest** ([R4](#s-fhir-mr)) is the order (status `active`, `on-hold`, `stopped` and others; `intent` `order`; `dosageInstruction` with timing, `asNeeded[x]` for PRN and `maxDosePerPeriod`). The pharmacist's verification has no dedicated element [uncertain]: a `Task` or a `Provenance` on the verified version is one option (OQ-MA2).
- **MedicationDispense** ([R4](#s-fhir-md)) is the supply from pharmacy or stock, against the request.
- **MedicationAdministration** is one administration event. `status` is 1..1 (`in-progress`, `not-done`, `on-hold`, `completed`, `entered-in-error`, `stopped`, `unknown`); `statusReason` 0..* says why it was not done; `effective[x]` 1..1 is a time or a period (an infusion); `performer.actor` with `function` (the giver and, for a controlled drug, the witness); `request` points at the order; `device` the pump; `dosage` route, dose and rate; `eventHistory` the Provenance trail. R3's `notGiven` was removed. Maturity 2, trial use. It covers self-administration too, and differs from MedicationStatement, which is a report that a medicine was taken, outside the prescribe-dispense-administer chain ([FHIR R4](#s-fhir-ma), read).
- **Medication** ([R4](#s-fhir-med)) is the product (code, form, ingredients, batch with lot and expiry). The UK code system is dm+d; others use RxNorm or a national list (terminology page, RC-7).
- **GS1**: GTIN, batch and expiry in a DataMatrix on the pack; GSRN on the wristband and staff badge; GLN for the location ([GS1 patient ID](#s-gs1-patient)).

## Workflows

Each step names what the nurse (or prescriber or pharmacist) needs on screen.

1. **Prescribe** (prescriber). Medicine picked from the formulary (coded), dose, route, form, times or frequency, PRN reason and 24-hour maximum, indication, review date, and whether it is time-critical (from the hospital's list). Allergy and duplicate checks fire (OR-6). Data: allergies (built), weight and height (built as vitals), renal function (results, not built).
2. **Verify** (pharmacist). Unverified orders listed; verify, query or suggest a change with a reason. An unverified order may still be given where policy allows (out of hours, first dose), and is shown as unverified on the chart. Data: order, history, results.
3. **Dispense** (pharmacy or ward stock). Supply recorded against the order with product, batch and expiry. Data: stock location.
4. **Open the drug round.** The ward's due list by time: due now, overdue (time-critical first and in red with words), due later, PRN available, and doses already given by another nurse. Data: active orders, administrations.
5. **Identify the patient.** Scan the wristband; a mismatch blocks the chart. With no scanner, two identifiers typed and a reason recorded. Data: identity banner, photo, allergies.
6. **Scan the medicine.** The scanned GTIN is matched to the order's medicine (product or its ingredient and strength); wrong drug, wrong strength, expired batch or a dose already given block or warn. The rights panel shows patient, drug, dose, route, time, the indication, and the last dose given. Data: order, product, batch, last administration.
7. **Give and record.** Dose given (a partial dose allowed with a reason), route, site for injections, time given (defaults to now; a past time needs a reason). For PRN: the reason given, the dose count in the last 24 hours against the maximum, and a reminder to record the effect. Data: administration record.
8. **Not given.** Choose a reason: patient refused, patient away from ward, nil by mouth, medicine unavailable, clinical reason (held), vomited, unable to take (swallowing), or other with text. Each is `not-done` with a coded reason. A time-critical dose not given raises a prompt to tell the prescriber, recorded as told or not. Data: reason list, escalation.
9. **Controlled drug.** The witness scans their badge or signs in, is shown the same dose, and is recorded as a second performer; the ward stock balance is decremented and shown. Data: stock register, witness identity.
10. **Infusion.** Start (time, rate, volume, line, pump if known), rate changes, bag changes and stop, each an event on one administration with an `effectivePeriod`. Pump values, if fed, are unvalidated until accepted (WL-10). Data: infusion record, fluid balance (EM-4).
11. **Correct.** Entered in error with a reason, or a new version; the original stays in history; a dose wrongly recorded as given never vanishes.
12. **Review** (prescriber, pharmacist). Omitted and delayed doses by ward and by medicine, time-critical ones first; PRN use; overrides with reasons. Data: reports.

## Rules

Each is a sentence a test can check.

1. A scan of a wristband that is not the open patient's blocks recording against that chart.
2. A scanned medicine that matches no active order for the patient blocks the dose; an override needs a reason, names the nurse and is audited.
3. Recording without a scan (patient or medicine) needs a reason from a closed list, and the administration carries `scanned: false` for each.
4. An administration names its order; there is no administration of a prescription-only medicine without an order, except an emergency dose under a recorded protocol that the prescriber must acknowledge by a set time.
5. An omitted dose is `not-done` with a reason and shows as not given, never as given and never as blank.
6. A due dose with no record by its time plus the ward's window is shown as "overdue, not recorded", distinct from "not given".
7. A time-critical dose is flagged overdue at its own shorter window, and the flag is not cleared until a dose or a reason is recorded.
8. A PRN dose beyond the order's 24-hour maximum is blocked unless overridden with a reason.
9. A dose recorded at a time more than the ward's limit before now needs a reason; a future time is refused.
10. A controlled-drug administration needs a witness who is a different person from the giver; with no witness it is refused, or recorded with a reason as policy allows, and listed.
11. A second administration of the same scheduled dose is refused unless the first is marked entered in error.
12. An administration is append-only; correction is a new version with a reason, and entered in error is final.
13. An expired batch scanned at the bedside blocks the dose.
14. An unverified order is labelled unverified wherever it is shown.
15. An order stopped after a dose was due shows the due dose as "stopped", not omitted, and not overdue.
16. Every refusal and override writes an audit row holding ids and codes, never a free-text reason (the audit rule for content).

## Hazards to add when built

For the Clinical Safety Officer; not added to the hazard log now. Each goes to `governance/clinical-safety/hazard-log.csv` (AS-1) in the commit that builds it.

- MA-H1 A dose given to the wrong patient (scan bypassed, two charts open, a wristband on the wrong patient).
- MA-H2 Wrong drug, strength, form or route given (look-alike, sound-alike, scan matched on ingredient only).
- MA-H3 A dose recorded as given that was not, or not given and recorded as given; a double dose.
- MA-H4 A time-critical dose omitted or late and not seen (Parkinson's, insulin, anticoagulants, anti-infectives, anti-epileptics) [the list is the hospital's].
- MA-H5 A PRN maximum exceeded across nurses or shifts.
- MA-H6 Override habituation: scanning bypassed so often that the check is meaningless.
- MA-H7 A controlled drug given without witness, or a stock balance wrong.
- MA-H8 An infusion rate, volume or concentration wrong, or a pump value taken as true without validation.
- MA-H9 An order stopped or changed after dispense, and the old dose given.
- MA-H10 A downtime (no network or no scanners) with no paper fallback and no back-entry path.

## First slice

- **Store.** Orders before administrations: `medication_request_versions` (append-only by trigger) with a coded medicine, dose, route, timing, PRN, maximum, time-critical flag; then `medication_administration_versions` with order id, status, reason code, time, giver, witness, scanned flags. No delete path. Schema map regenerated.
- **Pure rules.** `mar.rs`: given the orders, administrations, a clock and the ward windows, compute each dose's state (`due`, `overdue-not-recorded`, `given`, `not-given(reason)`, `stopped`, `prn-available`) and the refusals in Rules 1 to 11. No I/O; a fake clock in tests.
- **Routes.** `GET /api/patients/{id}/medication-requests`, `POST` (prescriber role); `GET /api/patients/{id}/mar` (the due list); `POST /api/patients/{id}/medication-administrations` (given or not given). FHIR read only at first: `GET /fhir/R4/MedicationAdministration?patient=`. No version segment in a route.
- **Screen.** A read-only drug chart on `/patients/[id]` by day, each dose's state in words and colour, overdue time-critical doses at the top; then the record-a-dose form with typed identifiers. Scanning comes after (OQ-M3).
- **Tests.** The EM-2 acceptance (wrong patient and wrong drug block; override needs a reason and is audited; omitted shows as omitted); a property test that no sequence of events makes a `not-done` dose read as given; a fake-clock test that an unrecorded time-critical dose is overdue at its window; a test that a witness equal to the giver is refused. Hazards MA-H1, MA-H3 and MA-H4 in the same commit.

## What this repo has and lacks

| Need | This repo today | Gap |
|---|---|---|
| Allergies for checks | Built (RC-5, partly) | Class-level matching via terminology (OR-6) |
| Weight and height | Built as vital signs | Dose calculation not built |
| Medicine terminology | Fixture only (RC-7) | dm+d or RxNorm, formulary |
| Orders | None (OR-1 for services) | MedicationRequest, verification |
| Dispensing | None | MedicationDispense, stock |
| Administration | None | EM-2 |
| Positive patient ID by scan | None | OQ-M3: build or integrate |
| Append-only, entered in error, audit | Built for other records | Same pattern to reuse |
| Medicines reconciliation | None | RC-10 |
| Pump and device feeds | Designed (WL-10) | EM-11, post-1.0 |
| Omitted-dose report | None | Needs administrations |

## Not verified

- Poon 2010, Koppel 2008, Westbrook 2010 and Elliott and Liu 2010 were read as abstracts and secondary summaries; the UCL 2025 scanning study, the override study, the pump review, the NPSA alert, SPS and Parkinson's UK pages as snippets.
- The controlled-drug witness rule rests on two local policies, not on the Misuse of Drugs Regulations text, which was not read.
- MedicationRequest and MedicationDispense element lists are from my reading of R4 and were not fetched this pass.
- Whether a closed-loop eMAR is a regulated medical device where it is used is OQ-6.
- No nurse, pharmacist or Clinical Safety Officer has read this page.

## Sources

- <a id="s-poon"></a>Poon et al., effect of bar-code technology on the safety of medication administration, NEJM 2010;362:1698–1707: https://pubmed.ncbi.nlm.nih.gov/20445181/
- <a id="s-koppel"></a>Koppel et al., workarounds to barcode medication administration systems, JAMIA 2008;15:408–423: https://pmc.ncbi.nlm.nih.gov/articles/PMC2442264
- <a id="s-ucl-scan"></a>Barcode scanning rates on five London wards, BMJ Health & Care Informatics 2025 (snippet; repository page, address inferred from the export links [uncertain]): https://discovery.ucl.ac.uk/id/eprint/10204088/
- <a id="s-override2"></a>Sustained barcode override rates below 2% in a health system (snippet, record page): https://katalog.lib.cas.cz/KNAV/EdsRecord/edsair,edsair.doi...........e9f1a5f99e15d5fb93a8e896d1f4bbdb
- <a id="s-westbrook"></a>Westbrook et al., interruptions and medication administration errors, Arch Intern Med 2010;170:683–690: https://researchers.mq.edu.au/en/publications/association-of-interruptions-with-an-increased-risk-and-severity-/
- <a id="s-elliott"></a>Elliott and Liu, the nine rights of medication administration, Br J Nurs 2010;19:300–305: https://pubmed.ncbi.nlm.nih.gov/20335899/
- <a id="s-npsa"></a>NPSA, Rapid Response Report RRR009, reducing harm from omitted and delayed medicines (2010): https://www.england.nhs.uk/wp-content/uploads/2014/04/psa-omitted-delayed-meds.pdf
- <a id="s-sps"></a>Specialist Pharmacy Service, safer use of time critical medicines: https://www.sps.nhs.uk/articles/safer-use-of-time-critical-medicines-programme/
- <a id="s-parkinsons"></a>Parkinson's UK, Get It On Time: https://www.parkinsons.org.uk/get-it-on-time
- <a id="s-cd-borders"></a>Second checking of controlled drugs, a Scottish board's guideline: https://rightdecisions.scot.nhs.uk/nhs-borders-clinical-guidelines/drugs-and-therapeutics/second-checking-of-controlled-drugs-in-community-hospitals-and-mental-health-units
- <a id="s-cd-wwl"></a>Preparation and administration of controlled drugs, an English trust SOP: https://wwl.nhs.uk/media/FOI/October%202023/9248%20-%20PREPARATION%20AND%20ADMINISTRATION%20OF%20CONTROLLED%20DRUGS%20TW%2010-037%20SOP%2012.pdf
- <a id="s-nice-ng46"></a>NICE NG46, controlled drugs: safe use and management: https://www.nice.org.uk/guidance/NG46/chapter/recommendations
- <a id="s-pump-review"></a>Smart infusion pump interoperability review, Medical Devices: Evidence and Research 2025: https://www.dovepress.com/evaluating-the-impact-of-smart-infusion-pump-interoperability-on-reduc-peer-reviewed-fulltext-article-MDER
- <a id="s-fhir-ma"></a>HL7 FHIR R4 MedicationAdministration: https://hl7.org/fhir/R4/medicationadministration.html
- <a id="s-fhir-mr"></a>HL7 FHIR R4 MedicationRequest: https://hl7.org/fhir/R4/medicationrequest.html
- <a id="s-fhir-md"></a>HL7 FHIR R4 MedicationDispense: https://hl7.org/fhir/R4/medicationdispense.html
- <a id="s-fhir-med"></a>HL7 FHIR R4 Medication: https://hl7.org/fhir/R4/medication.html
- <a id="s-gs1-patient"></a>GS1, patient and caregiver identification: https://www.gs1.org/sites/default/files/docs/healthcare/broch_a4_patient_caregiverid.pdf
- <a id="s-gs1uk"></a>GS1 UK, Scan4Safety: https://www.gs1uk.org/knowledge-hub/standards/how-gs1-uk-enables-scan4safety-for-the-nhs
- <a id="s-bahmni-mar"></a>Bahmni, medication administration and non-medication tasks: https://bahmni.atlassian.net/wiki/spaces/BAH/pages/3719561222/Medication+Administration+And+Non+Medication+Tasks
