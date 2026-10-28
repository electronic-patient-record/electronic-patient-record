# Hazard removal

Who may take a hazard off the active list, and what they must show. Related: [Clinical safety](../clinical-safety/index.md),
the [hazard log](../../governance/clinical-safety/hazard-log.csv), the
[risk management plan](../../governance/clinical-safety/risk-management-plan.md). Tasks AS-1, AS-13, AS-14, AS-15.

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

## Sources and how far to trust them

DCB0129 vendor guides (not the standard itself) say closed hazards stay in the log for auditability, that each log version
is approved by the CSO, and that control evidence should trace to a requirement, design decision, test or code change:
[Presencis hazard-log article](https://presencis.com/regulations/dcb0129/article-hazard-log/),
[Presencis CSO guide](https://cdn.presencis.com/regulations/dcb0129/guide/clinical-safety-officer). I did **not** read the
NHS England standard text, so the closure criteria and retention period are **[uncertain]** and go to AS-11 for checking.
This page is a proposal by an AI agent: it is not clinically reviewed and claims no compliance.
