#!/bin/sh
# Gate matcher-clippy (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-matcher-with-rust"
cargo clippy --all-targets --all-features -- -D warnings
