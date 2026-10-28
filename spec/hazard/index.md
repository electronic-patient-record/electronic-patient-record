# Hazard removal

Who may take a hazard off the active list, and what they must show. Related: [Clinical safety](../clinical-safety/index.md),
the [hazard log](../../governance/clinical-safety/hazard-log.csv), the
[risk management plan](../../governance/clinical-safety/risk-management-plan.md). Tasks AS-1, AS-13, AS-14, AS-15, AS-16.

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
