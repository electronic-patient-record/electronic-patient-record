#!/bin/sh
# Gate service-bench (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
# Each benchmark run once (criterion's --test): it still builds and runs. It measures nothing.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-service-with-rust"
cargo bench --locked --bench service -- --test
