# Arithmetic figures and their declared fallbacks

The declarations that [`../rust-checked-arithmetic/index.md`](../rust-checked-arithmetic/index.md) asks for (RCA-DECL-1, RCA-DECL-4): one row per figure whose arithmetic recovers with a fallback instead of returning `None` or an error. Code cites the ID in a comment at the call (RCA-NONE-3). A figure with no row here has no fallback: its failure is `None` or an error. Task QA-17.

| ID | Figure | Type and unit | Failure | Fallback | Why it is correct |
|---|---|---|---|---|---|
| ARITH-1 | `FixedClock` time (a test double, `electronic-patient-record-domain/src/clock.rs`) | `jiff::Timestamp` | advancing past the representable range | the clock stays where it was | a test clock that cannot move leaves the test to fail on the unchanged time; production code never reads it |
| ARITH-2 | Labels hidden by a decision's `Mask` obligations (`policy.rs`, `masked`) | `usize`, a count | the sum overflows `usize` | `usize::MAX` | the count only ranks decisions, preferring the one that hides least; a count too large to hold hides more than any other |
| ARITH-3 | Sum of hidden small-cell counts (`privacy.rs`, `suppress`) | `u64`, a count of people | the sum overflows `u64` | `u64::MAX` | the loop stops hiding more cells once the hidden sum reaches `k`; a sum too large for `u64` is certainly at least `k` |

## Figures that return unknown, not a fallback

| Figure | Failure | Result |
|---|---|---|
| A task or amendment due time (`clock::add_days`, `Request::due`) | the date is out of the calendar's range | `None`; `overdue` is `None` too, shown as unknown, never as "not overdue" |
| A suspicion pathway's target due day (`pathways::plan`) | the date is out of range | no plan (`None`) |
| The published total of a suppressed table (`privacy::suppress`) | the sum overflows `u64` | no total (`None`) |
| An observation's late-entry and before-birth limits (`observations_store::timing_problems`) | the limit date is out of the calendar's range (invalid data) | a refusal with code `effective_at_out_of_range`; no verdict is guessed either way (RCA-DECL-5) |
