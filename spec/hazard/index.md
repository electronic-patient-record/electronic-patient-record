# Hazard removal

Who may take a hazard off the active list, and what they must show. Related: [Clinical safety](../clinical-safety/index.md),
the [hazard log](../../governance/clinical-safety/hazard-log.csv), the
[risk management plan](../../governance/clinical-safety/risk-management-plan.md). Tasks AS-1, AS-13 to AS-18.

Planned unless stated. **NOT FOR CLINICAL USE.** No Clinical Safety Officer is named yet (OQ-2), so today nobody can remove
a hazard, and that is the intended state until one is.

## The rule

1. **Only the Clinical Safety Officer (CSO) can remove a hazard.** Not a maintainer, not an agent, not a pull request's
   author unless they are the CSO. (The role is called the Clinical Safety Officer in DCB0129 and in this repository;
   "chief safety officer" in the request that started this page means the same role.)
2. **The CSO must put in evidence that the hazard has been removed.** A removal with no evidence is refused, whoever makes it.
3. **Removal means retirement, not deletion.** The row stays in `hazard-log.csv` with its history; it leaves the active
   list by taking a closing status. No one, the CSO included, deletes a row.
4. **Reopening is open to anyone.** Any contributor may set a closed hazard back to `open` with a reason. Closing is hard
   and reopening is easy on purpose: a doubt should never need the CSO's permission to be recorded.

## What counts as evidence of removal

The evidence must show that the hazard **no longer applies** or that its **controls were verified**, not that someone says so.
It goes in the log row and every path must exist (the checker already refuses a path that does not).

| The hazard is removed because | Evidence the CSO must attach |
|---|---|
| The feature or data flow that caused it is gone | The commit or spec change that removed it, and a test that shows the route, column or screen is absent |
| A control now stops it | The test that fails when the control is removed (a planted failure, not only a passing run), with the run's output |
| The cause cannot occur any more (for example a data migration) | The migration, and a query or census that finds none left |
| A clinician reviewed it | A dated review record naming the reviewer and what they looked at (a file in `governance/clinical-safety/`) |

Not evidence: "no incident reported", a green build alone, the author's own rating, or a link that cannot be opened.
Each closure also records **who** (the CSO's name), **when**, **what was checked** and **what is left over** (the residual
column is never empty, even for a closed hazard).

## What the code enforces today, and what it does not

| Part of the rule | Today |
|---|---|
| Only the CSO may set `accepted` or `closed` | **Enforced on pull requests**: `scripts/check-hazard-log.py --base` fails unless the author's login is in `reviewers.txt` on the base branch. The list is empty, so every such change fails |
| `closed` needs a named reviewer | Enforced (`reviewed_by` must be non-empty) |
| No row is deleted | Enforced on pull requests (a removed row fails) |
| `closed` needs evidence of removal | **Not enforced.** `evidence` may be empty, and it describes controls, not closure |
| Reopening is recorded with a reason | **Not built** |
| The CSO is who they say they are | **Weak**: a GitHub login in a text file; no signature |
| The check runs on every change | **No**: the sign-off job runs on pull requests only (`.github/workflows/ci.yml`), and a direct push to `main` skips it |

## Weaknesses found, and advice

1. **Direct pushes bypass the sign-off.** The job has `if: github.event_name == 'pull_request'`, and this repository's work goes
   straight to `main`. Until branch protection requires pull requests (a repository setting only the owner can change), a
   closed hazard can be committed without the check. Advice: require pull requests for `main`, and also run the base
   comparison on `push` (against the previous commit). This is task AS-13.
2. **A login is not a signature.** Anyone who controls the CSO's account, or a bot with its token, can close a hazard.
   Advice: require the closing commit to carry a signature by a key listed for the CSO, not just a login (AS-14).
3. **No closure evidence is required**, so a CSO can close with an empty field and the checker passes. Advice: a closed
   hazard needs `closed_on`, `closure_evidence` (paths that exist) and `closure_note`; and `closed` with the old columns fails
   (AS-13).
4. **Evidence rots.** A path that existed on the day of closure can be deleted later and the hazard stays closed. Advice: the
   checker fails when a closed hazard's evidence path disappears, and a change under a hazard's watched code paths asks for
   a review (AS-15).
5. **The CSO and the author may be one person.** On a one-maintainer project the maintainer would also be the CSO, which
   removes the point of a second pair of eyes. Advice: DCB0129 expects the CSO to be a registered clinician trained in
   clinical risk management; the maintainer may not hold both roles for a release.
6. **Closed is not the same as accepted.** Accepting says "this residual risk is tolerable and stays on the log"; closing says
   "this no longer applies". Only closing removes it from the active list. The CSO must say which, in the note.
7. **The log is a file.** A row's history lives in git. Advice: never rewrite that history (no force-push, no squash of the
   log's commits); a signed tag at each release freezes the log as the release saw it.

## In the EPR system itself

Everything above governs the hazard log **file** in this repository, which the manufacturer keeps (DCB0129). The request
"in the EPR system" is a different thing: a **hazard register inside the running product**, where a deploying organisation
keeps its own hazards (DCB0160, the deployer's duty, with its own CSO). **Nothing of it is built:** the service has no
`ClinicalSafetyOfficer` role (`src/policy.rs` lists ten roles, none of them this one), no hazard table and no hazard route.
`src/gate.rs` already names the CSO as an example approver, and that is the only trace.

**Built 2026-10-28 (AS-17, in part):** the `clinical-safety-officer` role and the `Hazard` resource in the policy table
(CSO and clinician read, raise, record controls and reopen; the auditor reads; nobody else); the append-only
`hazard_versions` table with view `hazard_current` (migration 16, triggers refuse update, delete and truncate); the pure
rules (`src/hazard.rs`); the store (`src/hazard_store.rs`, no delete function); `GET` and `POST /api/hazards` and
`PUT /api/hazards/{hazard_id}` with the steps `control`, `retire` and `reopen`; the 403 for a retirement without the role,
which does not switch off when login is off; the 422 without evidence; four eyes (whoever recorded a control cannot
retire); an audit row per step holding the evidence hashes and no free text; the table in the personal-data inventory;
the role in the Keycloak realm and role map; hazard H-036. **Not built:** a person holding the role (OQ-2), the deployer
baseline, the gate link and the re-hash that reopens (AS-18), any screen in the UI, a copy of the evidence itself (only its
hash and reference are stored), the repository-file items (AS-13 to AS-15).

### The rule, in product terms

1. **Only a person holding the `clinical-safety-officer` role can retire a hazard.** The role is a new `Role` variant with
   one new permission, `hazard:retire`, in the policy table, so the route census (CQ-16) fails if any other role gets it.
   No clinical role has it, and a service, script or agent identity never has it.
2. **A retirement without evidence is refused with a 422**, whoever asks: at least one evidence item, each with a kind
   (`removal-change`, `test-run`, `review-record`, `census`), a reference, the time it was taken, and the **SHA-256 of the
   artefact as it was then**. The hash means later edits to the evidence can be detected.
3. **The register is append-only.** Events (`raised`, `rated`, `control-added`, `retired`, `reopened`) are rows that are never
   updated or deleted; a database trigger refuses both, as for `audit_events`. A hazard's state is the last event. There is
   no delete route and no migration may add one.
4. **Any authenticated clinical-safety reader may reopen**, with a reason; the system itself may reopen when a re-check of the
   evidence fails (see below). Retiring needs the CSO; reopening never does.
5. **Every event is audited** in the hash-chained trail, with the actor, the role used and the evidence hashes.
6. **The register holds no patient data.** Free text is length-limited and the table is listed in
   `governance/personal-data-inventory.csv`, so the census test forces its columns to be declared.

### Design questions I could not settle alone

| Question | Advice |
|---|---|
| Two registers (the repository's CSV and the product's) | Keep both, with different owners: the CSV is the **manufacturer's** file; the product register is each **deployer's**. A deployer should receive the manufacturer's hazards as a shipped baseline it **cannot retire**, only mark "does not apply here" with evidence. **[uncertain]**: I have not checked how DCB0160 splits these |
| The CSO is also a clinician | Retiring needs the CSO role **in that session's token**, not the clinician role: a separate login or a step-up, so a clinician's session cannot retire by accident |
| Four eyes | The person who recorded the control or the removal cannot also retire it. On a one-person site this fails; allow it only with a recorded override and flag it on the safety case |
| A hazard stuck open when the CSO leaves | Safe, but a lock-out: require **two** named CSOs (a deputy) and record who holds the role |
| Evidence rots | Store the hash and a copy of small artefacts; a scheduled re-check re-hashes the evidence and **reopens** the hazard if it no longer matches, then raises an alert |
| Retiring a hazard that gates a feature | Link a hazard to a gate (`src/gate.rs`): a feature with an open linked hazard may not reach `Show`. Retiring is then clinical, because it can turn a feature on |
| Time of day and clock | Use the server clock; the Podman clock-drift note in `AGENTS.md` applies to the test setup, not the rule |

### Risks

- **Clinical impact.** The register itself shows clinicians nothing, but a linked gate makes retirement able to switch a
  feature on. Treat any change that links them as `clinical-impact` and add a hazard row (AS-1).
- **Evidence can be forged by the CSO.** Hashes prove it did not change afterwards, not that it was true. Only an independent
  reviewer or the audit trail narrows that; do not claim more.
- **Nothing here is clinically reviewed.** It is an AI agent's proposal, with no compliance claim.

Tasks AS-17 (role, table, route, the 422 and the trigger) and AS-18 (the deployer baseline and the gate link).

## ISO 14971 (medical device risk management)

ISO 14971:2019 is the risk management process for medical devices; ISO/TR 24971:2020 is its guidance. It applies **only if a
module is a device** (the MHRA boundary in [`../12-compliance.md`](../12-compliance.md)). Today nothing here is claimed to be one,
so this is a mapping to prepare with, not a claim of conformity. The clause list below is from my general knowledge of the
standard, which is paywalled and **was not read**: every row is **[uncertain]** until someone with the text checks it (AS-11).

| 14971 activity | Where it lives here | Gap |
|---|---|---|
| Risk management plan, with the manufacturer's **criteria for acceptable risk** | [`risk-management-plan.md`](../../governance/clinical-safety/risk-management-plan.md) | The plan rates severity x likelihood but sets **no acceptability matrix**, so "accepted" has no yardstick |
| Intended use and foreseeable misuse | `spec/01`, `spec/02` (scope) | No written intended-use statement or misuse list |
| Hazard identification, hazardous situations, harms | hazard log columns `hazard`, `cause`, `effect` | No separate "hazardous situation" or "harm" field; one row mixes them |
| Risk estimation (severity, probability of harm) | `severity`, `likelihood`, `score` | Likelihood is of the hazard, not of harm following it; ratings are the author's, unvalidated |
| Risk control, in order: safe design, protective measures, information for safety | `controls` | The column does not say which kind each control is; a warning in the UI is the weakest kind |
| Verification of control implementation and effectiveness | `evidence` | Two checks in one column; only existence of the path is machine-checked |
| Residual risk evaluation, and overall residual risk | `residual` | No overall residual-risk judgement across all hazards |
| Benefit-risk analysis where residual risk is not acceptable | none | Not started |
| Risk management review before release; report | [`safety-case-report-template.md`](../../governance/clinical-safety/safety-case-report-template.md) | Unsigned template |
| Production and post-production information | none | No complaint, incident or field-feedback loop |

Two differences matter for the removal rule above. ISO 14971 has no "closed" state: a hazard stays until verified and its
residual risk is judged acceptable, and the file is kept as a **risk management file** for the product's life. And
DCB0129 (the NHS standard) is a separate duty that sits beside it; meeting one does not meet the other.

## Sources and how far to trust them

DCB0129 vendor guides (not the standard itself) say closed hazards stay in the log for auditability, that each log version
is approved by the CSO, and that control evidence should trace to a requirement, design decision, test or code change:
[Presencis hazard-log article](https://presencis.com/regulations/dcb0129/article-hazard-log/),
[Presencis CSO guide](https://cdn.presencis.com/regulations/dcb0129/guide/clinical-safety-officer). I did **not** read the
NHS England standard text, so the closure criteria and retention period are **[uncertain]** and go to AS-11 for checking.
This page is a proposal by an AI agent: it is not clinically reviewed and claims no compliance.
