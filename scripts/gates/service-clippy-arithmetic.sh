#!/bin/sh
# Gate service-clippy-arithmetic (LC-3): the arithmetic and conversion lints of spec/rust-checked-arithmetic
# (RCA-ENF-1), denied on production code: the libraries and the binary. Tests and benchmarks may use plain operators
# (RCA-SCOPE-4, RCA-EXC-5), and clippy has no way to allow these lints in tests only, so they are not in
# `[workspace.lints]`, which `--all-targets` would apply to tests too. Run by .github/workflows/ci.yml and
# scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-service-with-rust"
cargo clippy --workspace --lib --bins --all-features -- \
  -D clippy::arithmetic_side_effects -D clippy::integer_division -D clippy::modulo_arithmetic \
  -D clippy::default_numeric_fallback \
  -D clippy::as_conversions -D clippy::cast_lossless -D clippy::cast_possible_truncation \
  -D clippy::cast_possible_wrap -D clippy::cast_precision_loss -D clippy::cast_sign_loss \
  -D clippy::checked_conversions -D clippy::lossy_float_literal -D clippy::float_cmp \
  -D clippy::string_slice -D clippy::allow_attributes -D clippy::allow_attributes_without_reason \
  -D warnings
