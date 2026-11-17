# Mutation testing

A mutation run changes the code one small way at a time (a `<` becomes `<=`, a match arm is deleted, a function
returns a constant) and runs the tests against each change. A change the tests do not notice is a **missed** mutant:
behaviour no test pins. Every missed mutant here is killed by a new test, or listed below with the reason it cannot be.

**How to run:** `sh scripts/mutants.sh matcher` or `sh scripts/mutants.sh service` (cargo-mutants; the examined files
are each crate's `.cargo/mutants.toml`, chosen by hazard weight); `--in-diff main` tests only the lines a branch
changed. Not a CI gate: a full service run takes hours, and a scheduled run was declined (PR-8). Run it before a release
and after changing an examined file.

## Matcher (2026-11-17)

Every file except `main.rs`: 348 mutants, 303 caught, 9 unviable, **36 missed** (48 minutes, four jobs, on a machine
shared with other builds). Pins added the same day (`src/ihi.rs`, `src/normalize.rs`, `tests/fields.rs`) are meant to kill 31; the other 5 are listed below:

| Where | Missed | What the tests lacked | Pin |
|---|---|---|---|
| `ihi.rs` Luhn check | 8 of 9 | The tests derived the check digit with the function under test, so a broken algorithm agreed with itself | Five IHIs whose check digits were worked out outside the code, and every one-digit change of each refused |
| `engine.rs` best given-name pair | 8 | No record with two name lists, so which pair wins, and the tie rule, were never seen | Both orders of a better pair and of a tie between a pair with and without an extra name |
| `engine.rs` extra given name | 3 | No case where every name on the longer side matches | `Ann` against `Ann Ann` (nothing extra) and `Ann Mary` (extra) |
| `engine.rs` swapped names | 1 of 4 | The penalty's value was never read | A swapped pair scores 0.9 on both names |
| `engine.rs` thin evidence | 4 of 5 | The share of evidence was never read | The reported share equals the present weight over the counted weight |
| `engine.rs` config | 2 | Equal thresholds; a config without a calling code | Equal thresholds validate; a config that leaves the code out gets 44 |
| `normalize.rs` | 5 | `ä` and `ö`, a non-Latin letter with a mark, a date with four parts, `l` in Soundex | One assertion each |

**Not killable, with the reason:**

- `ihi.rs`: `x > 9` to `x >= 9`. `x` is a digit doubled, so it is even and never 9: the two are the same function.
- `engine.rs`: the `<` to `<=` at the swap floor (`f < SWAP_FLOOR && g < SWAP_FLOOR`), which differs only for a name
  similarity of exactly 0.85; and the `&&` to `||` there, which differs only when one name matches and the swapped pair
  still matches both ways, which the similarity measure does not produce for real names (three mutants). Listed
  rather than pinned with a contrived pair.
- `engine.rs`: the `<` to `<=` in `present / total < min_evidence_weight`, which differs only when the share equals
  the floor exactly.

Not re-run after the pins: the 31 are expected killed, not confirmed by a second run.

## Task rules (2026-11-17)

`electronic-patient-record-domain/src/task_rules.rs` (TL-3): 100 mutants. After the pins added the same day (event
words, an empty id or title, the inclusive limits, a calendar of 1,500 holidays), every viable mutant is caught but four,
none killable:

- `add_working_days`: `guard < 366` to `<=` (twice). The guard only bounds a calendar with a year of holidays in a row;
  366 and 367 days cannot be told apart by any real calendar.
- `latest_versions`: `>` to `>=`. Versions of one rule are distinct by the time this runs (`parse` refuses a repeat).
- `sub_working_days`: `day > from` to `>=`. The result is clamped to `from` afterwards, so one more step back changes
  nothing.

Confirmed by a second run of the 15 working-day mutants after the last pin.

## The domain crate's other examined files (2026-11-17)

A run with the domain crate's own tests only (no service tests) on the six examined domain files: 348 caught,
91 missed (including the task rules' before their pins), 36 unviable, 5 timed out. About 60 of the misses are deleted
arms of `policy::permitted`, the role table: the domain crate's unit tests do not exercise the whole table, and the
service's route census and HTTP tests, which do, did not run. These are to be judged in the service run, with the
service's tests, before any is called a gap. The rest (consent and death boundaries, hazard and policy word functions,
purpose parsing) are listed in that run's output and not yet judged.

## Service (2026-11-17)

The examined files: consent, policy, permission, death, the hazard and referral rules (now in the domain crate), the
route gate, break-glass, merge and unmerge, the audit chain, the HL7 reader and its redactor: 773 mutants. The unit
tests only (`--lib`): the integration tests need Postgres.

**Not finished.** The first run was interrupted after 159 mutants (40 caught, 43 missed, 28 unviable, 48 timed out on a
machine at a load average near 100); many of its misses are in `audit.rs`, whose behaviour the database tests pin, not
the unit tests. Nothing from it is acted on. A full run, and the judging of each miss, is still to do.
