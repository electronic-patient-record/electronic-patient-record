#!/bin/sh
# Gate matcher-test (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-matcher-with-rust"
# cargo-nextest when installed (LC-8: the same tests, faster); it runs no doc tests, so `cargo test --doc` follows.
if command -v cargo-nextest >/dev/null 2>&1; then
  cargo nextest run --workspace --locked --no-fail-fast
  cargo test --workspace --locked --doc
else
  cargo test --workspace --locked --no-fail-fast
fi
