# Production readiness checklist (PR-7, DRAFT)

**NOT FOR CLINICAL USE.** An item is ticked only with evidence named next to it. On 2026-11-04 **no item is ticked**: this is a scaffold (milestone M0). The right-hand column says what exists today, which is not the same as done. A tick needs a person to look at a real deployment.

| Area | Question | Evidence needed | What exists today |
|---|---|---|---|
| Safety | Is each hazard reviewed, and is every open one accepted or closed by the Clinical Safety Officer? | `hazard-log.csv` statuses and `reviewers.txt` | 44 hazards, none accepted or closed; no reviewer is named |
| Safety | Does each control name a test that still exists? | `scripts/check-hazard-log.py --report` | Named for H-044 only |
| Privacy | DPIA signed, retention periods set, data-subject routes tested | [dpia](dpia.md), [retention](retention-schedule.md) | Drafts; the periods are not set |
| Identity | Second factor required for staff | [mfa-plan](mfa-plan.md) | Not enforced or checked |
| Transport | TLS everywhere, including in front of MLLP | A scan of the deployed endpoints | Not built here; MLLP is plaintext |
| Secrets | Keys held outside the image and rotated; `EPR_INTEGRITY_MAC_KEY` set | The deployment's secret store | Variables exist; rotation untested end to end |
| Data | Backups taken and a restore tried; recovery time known | A restore record | **No backup procedure exists** |
| Audit | Chain verified on a schedule; archive anchors checked | A run of the verify route on the live data | Verify route and tests exist; no schedule |
| Operations | Metrics scraped, alerts set, logs kept | A dashboard and an alert test | `/metrics` exists (off by default); no alerts |
| Operations | One replica runs each background job | `audit_retention::run_once` lock | Built (PR-4); not tried with two real processes |
| Referrals | Timeout periods set per priority; an owner for the unmatched queue | `EPR_REFERRAL_TIMEOUTS`; a rota | Off by default; no owner named |
| Interop | Each feed and message type tested with the real partner | A partner test log | Only the synthetic and unit cases |
| Accessibility | Screen-reader and keyboard pass on the real screens | A written audit | Automated checks only |
| Language | Any non-English text reviewed by a speaker | [translation-review](translation-review.md) | Machine-written; none reviewed |
| Release | Supply chain, SBOM, licences, vulnerability scan green on the release commit | CI run | CI runs these on each push |
| Support | A named on-call, an incident route, a rollback | A runbook | Not written |

## Not checked

Written from this repository alone. It is not a complete list for any country's rules, and it does not replace the clinical safety standard that applies where the system is used.
