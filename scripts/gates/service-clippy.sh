#!/bin/sh
# Gate service-clippy (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-service-with-rust"
cargo clippy --workspace --all-targets --all-features -- -D warnings
