#!/bin/sh
# Gate coverage-matcher (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-matcher-with-rust"
# Measured by cargo-tarpaulin with its LLVM engine, as on CI. The floor sits just under the figure measured on 2026-11-17
# (96.16 percent of lines, 551 of 573; cargo-llvm-cov measured 97.65 on 2026-10-30). A floor may rise; lowering one needs a
# written reason. Its own target directory, so a coverage build does not throw away the ordinary one.
cargo tarpaulin --locked --engine llvm --skip-clean --target-dir target/tarpaulin \
  --out Lcov --output-dir . --fail-under 95
