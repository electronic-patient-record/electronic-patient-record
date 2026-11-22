# Arithmetic

Maths in this record must not panic and must not wrap. Where a result cannot be represented, the code says so with `None` (or an error the caller can refuse with); it never aborts the request and never returns a wrong number. Task QA-17. Companion to the rule that production code has no `unwrap`, `expect`, `panic!` or unchecked indexing (the `[workspace.lints.clippy]` table in the service's `Cargo.toml`).

**Built today:** the panic lints for `unwrap`, `expect`, `panic!`, indexing and slicing (denied in production code). **Not built:** the arithmetic rule below is not yet enforced by a lint, and the service still contains plain `+`, `-`, `*` and `as` casts on integers. A first run of clippy's `arithmetic_side_effects` found 94 such sites in the service and domain crates (2026-11-22); none has been judged yet as safe or unsafe.

## The rule

**Use the `checked_*` functions on integers and handle the `None`.** `checked_add`, `checked_sub`, `checked_mul`, `checked_div`, `checked_rem`, `checked_neg`, `checked_abs`, `checked_pow`, `checked_shl` and `checked_shr` return `Option<T>`: `Some(result)` when the operation fits the type, `None` when it would overflow (or divide by zero). The function that called them returns that `None` upward, or turns it into a refusal with a reason.

```rust
// An integer type has a top. One past it is not 0 and is not a crash: it is None.
assert_eq!(255_u8.checked_add(1), None);          // Option<u8>
assert_eq!(250_u8.checked_add(5), Some(255));
assert_eq!(0_u8.checked_sub(1), None);            // not 255
assert_eq!(i32::MIN.checked_abs(), None);         // there is no +2_147_483_648 in i32
assert_eq!(10_i32.checked_div(0), None);          // not a panic
```

### What is not allowed

| Instead of | Because | Use |
|---|---|---|
| `a + b`, `a - b`, `a * b` on integers | panics in a debug build, **wraps silently** in a release build: a count, a dose or a day number that is wrong and looks right | `a.checked_add(b)` and so on, then handle `None` |
| `a / b`, `a % b` | panics on a zero divisor, and on `MIN / -1` | `checked_div`, `checked_rem` |
| `-x`, `x.abs()` | panics (or wraps) on the type's minimum | `checked_neg`, `checked_abs` |
| `x as u8` and other narrowing casts | truncates without a word | `u8::try_from(x).ok()` |
| `.wrapping_*` | hides the overflow; allowed only where wrapping is the meaning (a hash, a sequence counter) and the line says so | `checked_*`, or a comment that names why wrapping is right |
| `.saturating_*` | returns a number that is not the true one | allowed only for a display cap (a count shown as "99+"), never for a clinical value, a date or a version |
| `iter().sum()` and `product()` | overflow as above | `try_fold(0_u32, \|a, b\| a.checked_add(b))` |

### Absent is `None`, never zero

This is the same rule as for evidence ([`../05-domain-model.md`](../05-domain-model.md)): when a result cannot be given, the caller is told "no result", and a missing or impossible number is never shown as `0` or as a clamped maximum. A function that adds two lengths of stay and cannot represent the sum returns `None`; the screen shows that it could not be worked out, with the reason.

- A `None` from arithmetic on a **clinical value** (a dose, a score, a result, an age, a due date) is a refusal, and the refusal says which value and why. It is not logged and skipped.
- A `None` on a **count or a duration for a metric** may drop that one sample, and the drop is counted.
- A version, a sequence number or an id that cannot be incremented stops the write (a `409`-style refusal or a `500` with no data stored), because a record that cannot be numbered cannot be appended.

### Floating point

Floating-point arithmetic does not panic, but it can give `NaN` or infinity, and a `NaN` compares unequal to itself. A float that comes from outside, or whose result is stored or shown, is checked with `is_finite()` first; a non-finite value is `None`. Money and clinical quantities are not held in floats; see the units rules in [`../05-domain-model.md`](../05-domain-model.md).

### Time and dates

Adding a duration to a time (`jiff`'s `checked_add`, `checked_sub`) can leave the supported range; use the checked forms and carry the `None` or error. Never add seconds to an epoch count with `+`.

## How it is checked

- **Lint (planned, QA-17).** `clippy::arithmetic_side_effects`, `clippy::cast_possible_truncation`, `clippy::cast_sign_loss` and `clippy::cast_possible_wrap` denied for production code, with the 94 existing sites fixed or each marked with an `#[allow]` and a one-line reason. Not switched on yet because the sites have not been triaged.
- **Test (rule for new code).** A function that does arithmetic on values from outside has a test at the type's edge: `MAX`, `MIN`, zero, and one past each. The test asserts `None` (or the refusal), not a number.
- **Review.** A change that adds `+`, `-`, `*`, `/`, `%`, `as` or `sum()` on integers in a non-test file says in its description why the operation cannot overflow, or uses `checked_*`.

## Not decided

- Whether to forbid `as` casts that widen (`u8` to `u32`) as well as the narrowing ones; they cannot lose data, and `u32::from(x)` is clearer, but the lint is noisy.
- Whether the arithmetic lint applies to the matcher crate, which is a separate library with its own gates.
