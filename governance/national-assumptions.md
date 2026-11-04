# National assumptions: a second pass (PR-7, DRAFT)

**NOT FOR CLINICAL USE.** [`spec/equity-audit.md`](../spec/equity-audit.md) (IO-19, 2026-10-29) is the main list under the equity rule in [`spec/02-scope.md`](../spec/02-scope.md). This is a second pass over what was built **after** it: referrals, the referral queue and worklist, timeouts, the page cut, retention jobs, the clock and the stored-word checks. It does not repeat the thirteen findings there. Nothing was changed.

## Found in the newer code

| # | Finding | Where | Who it disadvantages | Pack boundary |
|---|---|---|---|---|
| N1 | Referral **priority** words are `routine`, `urgent`, `asap`, `stat`; they are fixed in the state table, the database `CHECK` and the timeout settings. A country's own priority scheme (for example a two-week cancer pathway, or triage categories 1 to 5) needs mapping to these four | `src/referrals_store.rs`, `src/referral_timeouts.rs`, migration 22 | Any health system whose referral urgency classes differ | A referral-priority table in a pack; the four words kept as the neutral core |
| N2 | The referral **states** (12) come from one closed-loop model; a country's referral pathway may have states or steps this lacks (for example a mandatory triage or a booking-system hand-off) | `src/referral.rs` | Systems with a national e-referral service | Pack-selected state tables are not designed; the table is code |
| N3 | **Timeout periods** are counted in whole days from the database clock and are not tied to a working calendar or public holidays | `src/referral_timeouts.rs` | Places where "days" means working days | A site calendar, not built |
| N4 | **Retention** classes name no period; the schedule cites the NHS Records Management Code of Practice as one example | `governance/retention-schedule.md` | Everywhere else | Per-country retention packs |
| N5 | Regulatory references in governance documents are UK and US first (DCB0129, DSPT, HIPAA) | `governance/*.md` | Deployers elsewhere | Already named in `spec/02-scope.md` as that country's rule |
| N6 | The **referral worklist text** is English only and says "Referral replies"; the locale set has ten entries but no translation of these screens | UI `strings.ts` | Non-English readers | Translation, unreviewed (see [translation-review](translation-review.md)) |
| N7 | The audit **archive** and `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` assume one retention horizon for all audit rows | `src/audit_retention.rs` | Regimes with different horizons per record type | Configuration per record class, not built |

Nothing found that hard-codes a national identifier in the newer code: the referral code never mentions an NHS number.

## Not checked

- A keyword and reading pass, not a census: a finding that matched no keyword is missing.
- No review by a clinician or a person from another health system.
- Whether the closed-loop model fits a given country's pathway was not tested against any real pathway.
- The earlier audit's 13 rows were not re-verified here.
