# Nursing flowsheets: vital signs, NEWS2, fluid balance, assessments and care rounds

Public-source research read 2026-11-14 plus this repository's own files. A proposal and a reading, not clinical advice: nothing here is built, and no clinician has reviewed it.

Method: web searches and fetches (the Royal College of Physicians NEWS2 page and its 2020 implementation guidance, the FHIR R4 vital-signs profile, read in full; the rest mostly abstracts and search snippets, marked "snippet") and the project's own files: [domain model 5.2.1](../05-domain-model.md), [device readings](../device-readings/index.md), [virtual ward](../virtual-ward/index.md) (for shape), [implementation status](../14-implementation-status.md), the [service spec](../../electronic-patient-record-service-with-rust/spec/index.md), `src/observations.rs` and `src/assessment_forms.toml`, and tasks DO-5, RC-6, RC-11, EM-3, EM-4, EM-11, EM-13, WL-3 and WL-10 in [tasks.md](../../tasks.md). The companion page is [medication administration](../medication-administration/index.md); both draw on the same grid and the same "absent is not zero" rule.

## What it is

A **flowsheet** is the nurse's time-by-row chart: rows are things measured or assessed (pulse, respiratory rate, pain, intake, a turn, a risk score), columns are times, and each cell is one recorded value or a stated reason there is none. On paper it is the observation chart, the fluid balance chart and the care-round sheet; in an electronic record it is usually one configurable grid.

- **Vital signs and an early-warning score.** The bedside set (respiratory rate, oxygen saturation, supplemental oxygen, blood pressure, pulse, level of consciousness or new confusion, temperature) is scored by a track-and-trigger system; in the UK this is NEWS2 ([RCP](#s-rcp-news2)).
- **Escalation.** A score or a clinical concern triggers a response from a named responder within a set time. The score aids judgement; it does not replace it ([RCP guidance 2020](#s-rcp-2020)).
- **Fluid balance.** Intake and output by route per period, with running totals and a balance.
- **Nursing assessments and risk scores.** Falls, pressure ulcer, nutrition, VTE, delirium and others, each with a due time and a repeat interval (EM-3). Most named tools are licensed content (OQ-M2, OQ-WL2).
- **Care rounds.** Scheduled checks (repositioning, comfort, continence, hydration) recorded as done, not done with a reason, or declined.
- **Device readings.** Values from monitors and pumps arrive in the same grid but count only once a clinician validates them ([device readings](../device-readings/index.md), WL-10).

## How others do it

- **Electronic observations at the bedside.** Two English hospitals that replaced paper charts with a handheld system prompting a complete vital-sign set and computing the score reported falls in crude mortality from 7.75% to 6.42% and 7.57% to 6.15% ([Schmidt 2015](#s-schmidt)). The design is observational; a letter in the same journal argued confounding explained the "deaths avoided" (snippet).
- **Commercial systems.** Vendor pages list eObservations beside ePMA in UK records ([system C page](../system-c/index.md)); a single-site audit of a mental-health older-people ward reported NEWS documented correctly 100% versus 87% before ([Nervecentre page](../nervecentre/index.md)). These are vendor or single-site claims.
- **Open source.** Bahmni's inpatient module (built on OpenMRS) organises nursing tasks and the drug chart by shift ([Bahmni IPD](#s-bahmni)) (snippet). openEHR publishes a NEWS2 archetype that holds the component scores and total only, with the raw measurements in their own archetypes, and excludes under-16s and pregnancy ([openEHR CKM](#s-openehr)) (snippet).
- **Documentation burden.** A study of flowsheet logs counted roughly 630 to 875 manual flowsheet entries per patient per 12-hour shift in acute care and 633 to 689 in critical care, with device feeds supplying only 5 to 20% ([Collins 2018](#s-collins)) (abstract). A health system that removed 748 flowsheet groups, rows and options reported 15% (critical care) and 22% (acute) less active flowsheet time ([Vizient poster](#s-vizient)) (snippet, conference poster). The lesson for us: every row has a cost; rows are added by review, not by request.

## NEWS2: what it scores and what it does not

From the RCP chart as reproduced by secondary sources and the RCP's 2020 guidance; the RCP chart itself must be checked before any code ([uncertain] for the exact bands below until then).

| Parameter | Score bands (0 to 3) |
|---|---|
| Respiratory rate (/min) | ≤8 is 3; 9–11 is 1; 12–20 is 0; 21–24 is 2; ≥25 is 3 |
| SpO2 Scale 1 (%) | ≤91 is 3; 92–93 is 2; 94–95 is 1; ≥96 is 0 |
| SpO2 Scale 2 (%) | lower bands for a target of 88–92%, and on oxygen high saturations score up to 3 |
| Air or oxygen | oxygen is 2 |
| Systolic BP (mmHg) | ≤90 is 3; 91–100 is 2; 101–110 is 1; 111–219 is 0; ≥220 is 3 |
| Pulse (/min) | ≤40 is 3; 41–50 is 1; 51–90 is 0; 91–110 is 1; 111–130 is 2; ≥131 is 3 |
| Consciousness | Alert is 0; new confusion, Voice, Pain or Unresponsive is 3 |
| Temperature (°C) | ≤35.0 is 3; 35.1–36.0 is 1; 36.1–38.0 is 0; 38.1–39.0 is 1; ≥39.1 is 2 |

The total is 0 to 20 ([openEHR CKM](#s-openehr), snippet). The response bands (0–4 low, a single 3 low-medium, 5–6 medium, 7 or more high) come from the RCP 2017 report through secondary copies ([iatrox](#s-iatrox), [NWAS](#s-nwas)) [uncertain until read in the RCP report].

Rules the RCP states directly ([RCP guidance 2020](#s-rcp-2020), read in full):

- **New confusion scores 3**, and confusion of unknown novelty is assumed new until confirmed otherwise.
- **Scale 2 is only for confirmed hypercapnic respiratory failure** on blood gas, decided by a competent clinical decision maker and recorded in the notes; otherwise Scale 1. COPD without hypercapnia is often wrongly put on Scale 2. A known history of hypercapnia should be recorded so the right scale applies from presentation. Agreeing a target saturation is a separate decision from choosing the scale.
- **An incomplete NEWS2 is still calculated and shown as incomplete.** A patient may trigger on a single measure or on an incomplete aggregate; clinical judgement may respond at a lower threshold. A measure not obtainable despite the equipment being used should trigger an immediate response.
- **NEWS2 is an aid, not a substitute for clinical judgement.** Any concern prompts review whatever the score.

Known limits:

- **Validation.** NEWS2 changed Scale 2 thresholds without published derivation; a retrospective study found no evidence that NEWS2 identified patients better than NEWS and asked for more work before wide adoption ([Pimentel et al., Resuscitation](#s-pimentel)) (snippet; year 2018 or 2019 [uncertain]). A BMJ systematic review found early-warning-score validation studies mostly of poor method, with missing data poorly handled ([BMJ 2020 review](#s-gerry)) (snippet; the first author, Gerry, is from memory [uncertain]). Few studies test whether using the score improves outcomes (snippet, same search).
- **Pulse oximetry and skin tone.** Occult hypoxaemia (arterial saturation below 88% with oximetry 92–96%) was about three times as frequent in Black as in white patients in two US cohorts ([Sjoding 2020](#s-sjoding)). A UK independent review found extensive evidence that oximeters overestimate oxygen levels more in darker skin, and did not advise stopping their use ([Equity in Medical Devices 2024](#s-equity)) (snippet). SpO2 carries two NEWS2 parameters, so the bias reaches the score.
- **Population.** Not for under-16s or pregnancy ([openEHR CKM](#s-openehr), snippet); those need PEWS or an obstetric score. Not for critical care, which the NICE guideline also excludes ([NICE CG50](#s-nice-cg50)).
- **Virtual wards.** A remote-monitoring study found NEWS2 a poor predictor of (re)admission ([virtual ward page](../virtual-ward/index.md)).
- **Monitoring frequency and thresholds are local.** NICE asks for observations at least every 12 hours unless a senior decision changes it, a track-and-trigger system for all adults in acute settings, locally set and regularly reviewed thresholds, and a response triggered by the score or by clinical concern ([NICE CG50](#s-nice-cg50)) (snippet of the recommendations).
- **Trends and communication.** A national safety investigation recommended continued evaluation of how NEWS2 is recorded, responded to and communicated, including how electronic records present trends ([HSSIB](#s-hssib)) (snippet).

**Licence.** The RCP says "there is no copyright restriction on NEWS2" and allows reproduction if the material is not modified, charts are reproduced in colour, the RCP is acknowledged ("Reproduced from: Royal College of Physicians") and translations carry a not-approved disclaimer ([RCP](#s-rcp-news2), read). It says nothing about software. "Not modified" sits awkwardly with a computed score and an electronic chart; we should write to the RCP before shipping (OQ-NF1). The e-learning tool has its own end-user licence, which does not govern the score (snippet).

## Standards

- **FHIR R4 Observation, vital-signs profile.** Category `vital-signs`, a LOINC code, a patient, a time, and a UCUM value, or a `dataAbsentReason` when there is none; panel 85353-1; respiratory rate 9279-1, heart rate 8867-4, oxygen saturation 2708-6, temperature 8310-5, blood pressure panel 85354-9 with systolic 8480-6 and diastolic 8462-4 as components (either may carry a reason instead of a value). A system that supports any listed vital sign "SHALL conform" ([FHIR vital signs](#s-fhir-vitals), read).
- **The score itself** is a separate `Observation` whose `derivedFrom` names the measurements it used, with each component in `component` [uncertain: no published FHIR NEWS2 profile was found; the openEHR archetype has the same split]. A LOINC code for the NEWS2 total was not confirmed (OQ-NF2).
- **Questionnaire and QuestionnaireResponse** for nursing assessments; scoring stays in our code (EM-3) and the form version is kept on every response (built, WL-3). DO-7 (SDC `$extract`) would turn answers into Observations.
- **Fluid balance** as Observations with an intake or output code and volume in mL, the period as `effectivePeriod` [uncertain: no fluid-balance profile confirmed].
- **Device readings** carry `Observation.device` and the capture time ([device readings](../device-readings/index.md)).
- **Positive patient identification** at the bedside uses a GS1 DataMatrix wristband carrying a GSRN, with the location as a GLN; England's Scan4Safety programme scans the wristband before observations enter the record ([GS1 patient ID](#s-gs1-patient), [GS1 UK case studies](#s-gs1uk)) (vendor material).

## Workflows

Each step names what the nurse needs on screen.

1. **Open the ward list.** Every patient with: latest NEWS2 and its time, trend arrow, "incomplete" marker, Scale 2 flag, next observations due and overdue, assessments overdue, open escalation and who holds it. Data: ward census, latest observations, monitoring plan. Feeds EM-13 handover.
2. **Identify the patient.** Scan the wristband (or two identifiers, typed, if no scanner) before the chart opens for entry. Data: identity banner, photo if held, allergies.
3. **Record a set of observations.** One row per parameter for this time; the device value pre-filled but marked unvalidated until accepted; each blank needs a reason (not done, declined, unable to obtain despite equipment, no equipment). Consciousness as ACVPU; oxygen as air or the device and flow. The SpO2 scale shown is the one on record, with who chose it and when. Data: previous set, the patient's monitoring plan, target saturation.
4. **See the score.** Total, each component, "incomplete: n of 7 recorded" when a parameter is missing (never counted as 0), and a measure "not obtainable despite equipment" shown as an immediate-response prompt. Data: score with its rule version.
5. **Escalate.** The response band, the named responder role and the time to respond, set by the ward's policy; a nurse may escalate on concern alone, with no score. Record who was told, how, when, and the response (attended, advice given, plan changed, not attended). The escalation stays open until answered. Data: escalation record, responder rota.
6. **Set or change the monitoring plan.** Frequency (default from the ward, changed only by a senior decision with a reason), an individual threshold for chronic hypoxia (RCP), Scale 2 by a competent decision maker after confirmed hypercapnia, with the reason. Data: plan history.
7. **Fluid balance.** Enter intake and output by route; running totals for the period; a period with no entry shows as "not recorded", not 0 mL; the balance is shown only when the period is complete or is labelled partial. Data: chart for 24 hours, the target if one is set.
8. **Assessments due.** Admission assessments with their due time; repeats by interval or by trigger (a fall, a transfer, surgery); the form version shown; an unanswered item is "not assessed". Data: assessment list (built for one example form).
9. **Care rounds.** Scheduled checks listed by time; done, not done (with reason), declined; a missed round shows as missed. Data: care plan (CA-2).
10. **Validate device readings.** Accept a set over a period or reject one as artefact with a reason; only accepted values enter the score (WL-10, RC-11).
11. **Correct an entry.** A new version with a reason, or entered in error; the original stays visible in history and the score is recomputed from what now stands, keeping the old score in history.
12. **Hand over.** The list in step 1 with the open escalations and the plan; nothing open drops off (EM-13).

## Rules

Each is a sentence a test can check.

1. A missing NEWS2 parameter never scores 0; the score is returned with `complete: false` and the list of missing parameters.
2. An incomplete score is still computed and shown, and a single parameter scoring 3 still triggers its band.
3. A parameter recorded as "not obtainable despite equipment" produces an immediate-response prompt whatever the total.
4. Scale 2 is used only when the patient's record holds a Scale 2 decision with its author, time and reason; otherwise Scale 1 is used and the score says which scale.
5. Consciousness "confusion, novelty unknown" scores as new confusion (3).
6. A device reading not yet validated never changes a score, a fluid balance or a trigger (WL-10, RC-11).
7. The score is computed on read from the standing measurements and its rule version is shown; a stored score, if kept for history, names the measurement versions it used.
8. A score is never computed for a patient under 16 or recorded as pregnant or postpartum; the screen says why.
9. Every escalation names who escalated, to whom (a role or a person), when, and stays open until a response is recorded.
10. A nurse can raise an escalation with no score (clinical concern), and it is not refused.
11. A fluid-balance period with no entries shows "not recorded"; a total over a period with gaps is labelled partial.
12. A fluid entry is append-only: correction is a new version with a reason.
13. A changed monitoring frequency or individual threshold records its author, a reason and a review date; a change past its review date is listed.
14. An assessment answer left blank is "not assessed", never a score of 0 for that item, and a total over blanks is labelled incomplete.
15. Every flowsheet row added to a deployment's configuration names an approver and a reason (burden control).
16. An entry for the wrong patient is corrected by marking it entered in error on that patient and recording it again on the right one; nothing is moved silently.

## Hazards to add when built

For the Clinical Safety Officer; not added to the hazard log now. Each goes to `governance/clinical-safety/hazard-log.csv` (AS-1) in the commit that builds it.

- NF-H1 NEWS2 miscalculated (wrong band, wrong scale, boundary error); DO-5 asks for this row.
- NF-H2 A missing parameter read as normal, so the total looks low.
- NF-H3 Scale 2 applied without confirmed hypercapnia, hiding hypoxia; or Scale 1 kept when Scale 2 was decided.
- NF-H4 Oximetry overestimates saturation in darker skin and the score reassures.
- NF-H5 An escalation raised but never answered, or closed with no response.
- NF-H6 An unvalidated device artefact changes a score, or a validated real value is rejected.
- NF-H7 Observations charted on the wrong patient.
- NF-H8 A fluid-balance gap read as zero, giving a false balance.
- NF-H9 An overdue observation set or assessment not shown, or the overdue list over-full and ignored.
- NF-H10 A licensed risk tool encoded wrongly or from an old version.

## First slice

- **Store.** No new table for the score: computed on read from `observation_current`. Add two observation kinds the score needs and the record lacks: level of consciousness (ACVPU, coded) and inspired oxygen (air or oxygen, with device and flow). A `monitoring_plan_versions` table (append-only by trigger) for frequency, Scale 2 decision with author and reason, and individual thresholds. Migration and `generate-schema-map.py` with it.
- **Pure rules.** `news2.rs`: input the standing measurements, the scale decision and age and pregnancy; output each component, the total, `complete`, `missing`, `band`, and the rule version. No I/O.
- **Routes.** `GET /api/patients/{id}/news2` (the score, with the measurement versions used) and `GET /api/wards/{id}/news2` later with the ward list; `GET`/`POST /api/patients/{id}/monitoring-plan`. No `/api/v1` (AGENTS.md).
- **Screen.** A read-only grid of the last 24 hours on `/patients/[id]` with the score row under it, "incomplete" shown in words as well as colour, and the scale named.
- **Tests.** Reference cases for every band edge of every parameter (from the RCP chart once read), Scale 1 and Scale 2, every single-parameter 3, a missing parameter (score incomplete, never 0), "not obtainable despite equipment", under-16 and pregnancy refusal, a property test that adding a parameter never lowers the count of known components. Hazard row NF-H1 and NF-H2 in the same commit.

## What this repo has and lacks

| Need | This repo today | Gap |
|---|---|---|
| Vital signs with LOINC, UCUM, absent reason | Built: eight vitals, append-only, FHIR read (RC-6) | No consciousness (ACVPU), no supplemental oxygen, no grid |
| Absent evidence as a reason, never zero | Built (`AbsentReason`: not done, declined, failed, unknown) | No "unable to obtain despite equipment" |
| NEWS2 | None | Score, scale decision, bands, escalation |
| Device readings, validated | Designed (WL-10) | Source, device, validated state on `Observation` (EM-11, RC-11) |
| Assessments under versioned forms | Built for one example form (WL-3) | Licensed tools (OQ-M2, OQ-WL2), scoring, due times |
| Fluid balance | None | Whole area (EM-4) |
| Care rounds | None | Needs care plans and tasks (CA-2) |
| Escalation | None | Record, responder, open-until-answered |
| Ward list, handover | None | EM-13 |
| Positive patient ID by scan | None | Shared with eMAR (OQ-M3) |

## Not verified

- The exact NEWS2 bands above come from secondary reproductions of the RCP chart and the RCP 2017 report was not read; the response bands are [uncertain] until it is.
- Schmidt 2015, Pimentel, the BMJ 2020 review, Collins 2018, the HSSIB report, the NICE recommendations, the equity review and the openEHR archetype were read as abstracts or snippets, not in full.
- No FHIR NEWS2 profile and no LOINC code for the NEWS2 total were confirmed.
- Whether NEWS2 software is a regulated medical device where it is used is OQ-6 and not decided here.
- No nurse, Clinical Safety Officer or the RCP has read this page.

## Sources

- <a id="s-rcp-news2"></a>RCP, National Early Warning Score (NEWS) 2, page and reproduction terms (read 2026-11-14): https://www.rcp.ac.uk/improving-care/resources/national-early-warning-score-news-2/
- <a id="s-rcp-2020"></a>RCP, NEWS2 additional implementation guidance, March 2020 (read in full): https://www.rcp.ac.uk/media/umzn4ntq/news2_additional-guidance-002-_0.pdf
- <a id="s-fhir-vitals"></a>HL7 FHIR R4 vital signs profile: https://hl7.org/fhir/R4/observation-vitalsigns.html
- <a id="s-schmidt"></a>Schmidt et al., electronic physiological surveillance and hospital mortality, BMJ Qual Saf 2015;24:10–20: https://qualitysafety.bmj.com/content/24/1/10.abstract
- <a id="s-pimentel"></a>Pimentel et al., NEWS and NEWS2 in the HAVEN database, Resuscitation (accepted manuscript): https://core.ac.uk/download/161511054.pdf
- <a id="s-gerry"></a>Early warning scores systematic review summary (NIHR alert on Gerry et al., BMJ 2020): https://evidence.nihr.ac.uk/alert/early-warning-scores-used-in-hospitals-must-be-based-on-sound-science
- <a id="s-sjoding"></a>Sjoding et al., racial bias in pulse oximetry, NEJM 2020;383:2477–2478: https://pmc.ncbi.nlm.nih.gov/articles/PMC7808260
- <a id="s-equity"></a>Equity in medical devices: independent review, final report, 11 March 2024: https://www.gov.uk/government/publications/equity-in-medical-devices-independent-review-final-report
- <a id="s-nice-cg50"></a>NICE CG50, acutely ill adults in hospital, recommendations: https://www.nice.org.uk/guidance/cg50/chapter/1-Guidance
- <a id="s-hssib"></a>HSSIB, recognising and responding to critically unwell patients: https://www.hssib.org.uk/patient-safety-investigations/recognising-and-responding-to-critically-unwell-patients/
- <a id="s-openehr"></a>openEHR CKM, NEWS2 archetype openEHR-EHR-OBSERVATION.news2.v1: https://ckm.openehr.org/ckm/archetypes/1013.1.3342
- <a id="s-collins"></a>Collins et al., quantifying and visualizing nursing flowsheet documentation burden, AMIA 2018: https://pubmed.ncbi.nlm.nih.gov/30815074/
- <a id="s-vizient"></a>Vizient poster, flowsheet reduction: https://continuingeducation.vizientinc.com/sites/default/files/media/2023-10/P423.pdf
- <a id="s-bahmni"></a>Bahmni, In-Patient Department: https://bahmni.atlassian.net/wiki/spaces/BAH/pages/3721953281/In-Patient+Department
- <a id="s-gs1-patient"></a>GS1, patient and caregiver identification: https://www.gs1.org/sites/default/files/docs/healthcare/broch_a4_patient_caregiverid.pdf
- <a id="s-gs1uk"></a>GS1 UK, clinical benefits of point-of-care scanning: https://www.gs1uk.org/insights/case-studies/Harnessing-the-clinical-benefits-of-point-of-care-scanning
- <a id="s-iatrox"></a>iatrox, NEWS2 scoring and escalation (secondary): https://www.iatrox.com/blog/news2-scoring-escalation-guide
- <a id="s-nwas"></a>North West Ambulance Service, NEWS2 (secondary copy of the chart): https://www.nwas.nhs.uk/publications/news2-national-early-warning-score
