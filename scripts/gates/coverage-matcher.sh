#!/bin/sh
# Gate coverage-matcher (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-matcher-with-rust"
# The floor sits just under the figure measured on 2026-10-30 (97.65 percent of lines). A floor may rise;
# lowering one needs a written reason.
cargo llvm-cov --locked --lcov --output-path lcov.info --fail-under-lines 95
