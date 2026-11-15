#!/bin/sh
# Gate service-test (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-service-with-rust"
# Every test that needs no database, in every crate and test binary, and the doc tests: a new test file runs
# without a workflow edit (CQ-24). Database tests are `#[ignore]`d and run in service-database.
# cargo-nextest when installed (LC-8: the same tests, faster); it runs no doc tests, so `cargo test --doc` follows.
if command -v cargo-nextest >/dev/null 2>&1; then
  cargo nextest run --workspace --locked --no-fail-fast
  cargo test --workspace --locked --doc
else
  cargo test --workspace --locked --no-fail-fast
fi
