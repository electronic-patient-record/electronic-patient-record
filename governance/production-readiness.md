# Production readiness checklist (PR-7, DRAFT)

**NOT FOR CLINICAL USE.** An item is ticked only with evidence named next to it. On 2026-11-04 **no item is ticked**: this is a scaffold (milestone M0). The right-hand column says what exists today, which is not the same as done. A tick needs a person to look at a real deployment.

| Area | Question | Evidence needed | What exists today |
|---|---|---|---|
| Safety | Is each hazard reviewed, and is every open one accepted or closed by the Clinical Safety Officer? | `hazard-log.csv` statuses and `reviewers.txt` | 55 hazards on 2026-11-09, 34 open, none accepted or closed; no reviewer is named |
| Safety | Does each control name a test that still exists? | `scripts/check-hazard-log.py --report` | Named for H-044 only |
| Privacy | DPIA signed, retention periods set, data-subject routes tested | [dpia](dpia.md), [retention](retention-schedule.md) | Drafts; the periods are not set |
| Identity | Second factor required for staff | [mfa-plan](mfa-plan.md) | Not enforced or checked |
| Transport | TLS everywhere, including in front of MLLP, and `DATABASE_URL` sets `sslmode` to `require`, `verify-ca` or `verify-full` | A scan of the deployed endpoints; `axa` refuses to start in production without `sslmode`, unless the address is on the same machine or `DB_ALLOW_INSECURE=true` | Not built here; MLLP is plaintext |
| Secrets | Keys held outside the image and rotated; `EPR_INTEGRITY_MAC_KEY` set | The deployment's secret store | Variables exist; rotation untested end to end |
| Data | Backups taken and a restore tried; recovery time known | A restore record | **No backup procedure exists** |
| Audit | Chain verified on a schedule; archive anchors checked | A run of the verify route on the live data | Verify route and tests exist; no schedule |
| Operations | Metrics scraped, alerts set, logs kept | A dashboard and an alert test | `/metrics` exists (off by default); no alerts |
| Operations | One replica runs each background job | `audit_retention::run_once` lock | Built (PR-4); not tried with two real processes |
| Workforce | Workforce planning holds the rota and keeps an `on-duty-clinician` on every shift, and the role changes in the identity provider at each handover | The workforce system's own record; the identity provider | No link built; the role is given by hand, and the service cannot see a gap |
| Referrals | Timeout periods set per priority; an owner for the unmatched queue | `EPR_REFERRAL_TIMEOUTS`; a rota | Off by default; no owner named |
| Interop | Each feed and message type tested with the real partner | A partner test log | Only the synthetic and unit cases |
| Accessibility | Screen-reader and keyboard pass on the real screens | A written audit | Automated checks only |
| Language | Any non-English text reviewed by a speaker | [translation-review](translation-review.md) | Machine-written; none reviewed |
| Release | Supply chain, SBOM, licences, vulnerability scan green on the release commit | CI run | CI runs these on each push |
| Support | A named on-call, an incident route, a rollback | A runbook | Not written |

## "Live", per site (WL-11)

A deployment never reports that it is "live" in one word. The Auditor General for Wales found that on WCCIS "'Live' meant
different things" from one organisation to the next, while key functions were delayed and others were deployed
differently ([spec/nhs-wales](../spec/nhs-wales/index.md), [AGL22]). So the readiness report for a deployment holds one
row per **site and function**, and a function is in use at a site only with the evidence in the row.

| Column | What it holds |
|---|---|
| Site | The ward, clinic, practice or team, as the deploying organisation names it |
| Function | One line from the list below |
| State | `not in use`, `trial` (named users, a stated end date), `in use`, or `withdrawn` (with the date and why) |
| Since | The date the state began |
| Who | The roles using it there, and roughly how many people |
| Settings | The settings that change its behaviour at that site, as set (for example `EPR_REQUIRE_PERMISSION_BASIS`, `EPR_REFERRAL_TIMEOUTS`, the suspicion pathway catalogue's version) |
| Evidence | What shows it is used as stated: an audit query, a training record, a named person's sign-off |
| Not used there | What of the function that site does not use, and what it does instead (paper, another system) |

The functions, as this repository holds them on 2026-11-09 (the service spec lists what each does and does not do):

- patient identity: registration, search, matching, merge and unmerge;
- consent and care relationships, break-the-glass and its review;
- allergies and the no-known-allergies statement;
- observations;
- results and sign-off (entered by hand; no feed);
- assessments under versioned forms (one example form only);
- referrals and their closed loop, the referrer's worklist, timeouts, the referral queue;
- suspicion on a referral, its pathway catalogue, and its outcome;
- watch lists;
- reasonable adjustments, amendments, death assertions;
- the permission basis on clinical reads;
- the HL7 v2 ADT feed (MLLP) and the FHIR R4 and R5 facades;
- the outbox and its sinks;
- the clinician UI, and each locale in use (Welsh is machine-written and unreviewed).

Rules for the report:

- **A function with no row at a site is `not in use` there**, never assumed live.
- **A row is per site, never per organisation**, because two wards of one health board can use different functions.
- **Settings are part of the state.** A function behaves differently with a setting on or off, so the row records it;
  two sites with different settings are two different deployments of the function.
- **The status this repository generates (`spec/14-implementation-status.md`) is not this report.** It counts code and
  tests; it says nothing about any site. Nothing in this repository can tick a row.

**Not done (WL-11):** no deployment exists, so no report has been written; there is no template file or tool that
produces one, and nothing checks that a deployment keeps it current.

## Not checked

Written from this repository alone. It is not a complete list for any country's rules, and it does not replace the clinical safety standard that applies where the system is used.
