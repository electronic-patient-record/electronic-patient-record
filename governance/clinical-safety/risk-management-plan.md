# Clinical risk management plan (template)

**Status: a DRAFT template, written by an AI coding agent. It is not signed, not reviewed by a clinician, and not a
claim of compliance with DCB0129 or any other standard.** The Clinical Safety Officer (CSO) is **not named** (open
question OQ-2); until one is, nothing in this folder can be accepted or closed.

## What this plan covers

The software in this repository, version 0.1.0, which is a scaffold (**not for clinical use**). Today it holds patient
identity, consent, care relationships, an audit trail and a clinician and patient interface. It holds no clinical content.

## Roles

| Role | Who | Notes |
|---|---|---|
| Clinical Safety Officer | *not named (OQ-2)* | A registered clinician trained in clinical risk management. Owns this plan, accepts or rejects each hazard, signs the safety case report. |
| Maintainer | the repository owner | Keeps the hazard log current in the same change that alters behaviour. |
| Privacy officer | *not named* | Reviews the permission table, purposes and consent rules. |

## Method

1. **Identify.** A hazard is anything that could lead a clinician or patient to be harmed, or a record to be wrongly
   disclosed, changed or lost. Add it to [`hazard-log.csv`](hazard-log.csv) with its cause and effect.
2. **Rate.** Severity 1 (minor) to 5 (catastrophic) and likelihood 1 (very low) to 5 (very high), before the controls are
   counted on; `score = severity x likelihood`. The ratings in the log are **proposed by the author and not validated**.
3. **Control.** Name each control and point at the evidence (a test or a file). A control with no evidence is not a control.
4. **Residual.** Say what is left: what the controls do not cover. This column is never empty.
5. **Review.** Only the CSO sets a hazard to `accepted` or `closed`, and records their name in `reviewed_by`.
   `scripts/check-hazard-log.py` refuses either status without a name.
6. **Keep current.** A change that could alter what a clinician sees, decides or signs is labelled `clinical-impact` and
   must change the hazard log (`scripts/check-clinical-impact.sh`). When unsure, treat it as clinical.

## Not done

No clinical risk assessment workshop, no external review, no CSO, no safety case report beyond the
[template](safety-case-report-template.md), no link to a clinical advisory group, no post-market surveillance process.
