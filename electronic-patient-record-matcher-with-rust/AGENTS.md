# AGENTS.md — Matcher

Pure Rust library: patient identity matching for a master patient index. Read [`spec/index.md`](spec/index.md) first.

- **Pure**: no IO, no clocks, no randomness, no panics on any input. `compare(a, b)` is symmetric and deterministic.
  The dependency list is serde, serde_json, strsim, unicode-normalization, thiserror; keep it that way.
- **Explainable**: every `MatchResult` carries a per-field breakdown. Absent evidence is `similarity: None` with a
  note, never zero.
- **Refuses before it guesses**: automatic linking needs given name, family name and birth date present and
  agreeing; a missing field blocks `AutoLink` (it goes to `Review`). Two different valid NHS numbers are
  `NonMatch`. Do not loosen a guard to make a test pass; a wrong automatic link is a patient safety incident.
- `Patient` is also the service's API DTO. Change it here, and update the entity spec §5 and the UI types in one commit.
- Gates: `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test`.
- Not built: the labelled corpus (ID-1), fuzz target, benches, phonetic matching, address-line matching.
