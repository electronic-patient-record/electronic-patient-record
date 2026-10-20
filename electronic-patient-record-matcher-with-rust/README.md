# electronic-patient-record-matcher

Explainable patient identity matching. Pure, deterministic, symmetric.

```sh
cargo run -- a.json b.json   # prints the MatchResult as JSON
cargo test
```

```rust
use electronic_patient_record_matcher::{MatchingEngine, Patient};
let result = MatchingEngine::default().compare(&a, &b);
// result.decision: AutoLink | Review | NonMatch; result.fields explains why
```

Not for clinical use. Thresholds are unvalidated. See [`spec/index.md`](spec/index.md).
