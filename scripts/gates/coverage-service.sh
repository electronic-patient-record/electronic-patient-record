#!/bin/sh
# Gate coverage-service (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-service-with-rust"
# Database-free tests only (the service's and the domain crate's unit tests), measured by cargo-tarpaulin with its
# LLVM engine, which CI and a Mac both use; both measured the same lines on 2026-11-17. The floor sits just under the figure measured on
# 2026-11-17 (37.71 percent of lines, 5,719 of 15,164). A floor may rise; lowering one needs a written reason.
# History: under cargo-llvm-cov the floor was 50 (52.24 percent measured on 2026-10-30, 53.44 on CI on 2026-11-17).
# The move to tarpaulin re-set it; coverage did not fall. llvm-cov counts the unit tests' own lines, which always run;
# tarpaulin leaves test code out, so it measures only the code under test. With `--include-tests` the same run gives
# 55.90 percent (2026-11-17), in line with llvm-cov's figure.
# Its own target directory, so a coverage build does not throw away the ordinary one.
cargo tarpaulin --locked --engine llvm --skip-clean --target-dir target/tarpaulin \
  --workspace --lib --out Lcov --output-dir . --fail-under 36
