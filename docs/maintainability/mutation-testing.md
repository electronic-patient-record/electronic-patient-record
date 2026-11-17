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

## Service (2026-11-17)

The examined files: consent, policy, permission, death, the hazard and referral rules (now in the domain crate), the
route gate, break-glass, merge and unmerge, the audit chain, the HL7 reader and its redactor: 773 mutants. The unit
tests only (`--lib`): the integration tests need Postgres.

**Not finished.** The first run was interrupted after 159 mutants (40 caught, 43 missed, 28 unviable, 48 timed out on a
machine at a load average near 100); many of its misses are in `audit.rs`, whose behaviour the database tests pin, not
the unit tests. Nothing from it is acted on. A full run, and the judging of each miss, is still to do.
