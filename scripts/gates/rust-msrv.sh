#!/bin/sh
# Gate rust-msrv (QA-8, LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
# Each crate builds with the oldest Rust its Cargo.toml's `rust-version` claims (1.98, which axa 0.17
# needs for the service), overriding the pin in rust-toolchain.toml. Library, binary and test code, with the
# lockfile as committed. A toolchain that is not installed is installed (minimal profile).
set -eu
root="$(cd "$(dirname "$0")/../.." && pwd)"
for crate in electronic-patient-record-matcher-with-rust electronic-patient-record-service-with-rust; do
  version="$(sed -n 's/^rust-version *= *"\(.*\)"/\1/p' "$root/$crate/Cargo.toml")"
  if [ -z "$version" ]; then
    echo "rust-msrv: $crate/Cargo.toml has no rust-version" >&2
    exit 1
  fi
  if ! rustup run "$version" rustc --version >/dev/null 2>&1; then
    rustup toolchain install "$version" --profile minimal --no-self-update
  fi
  echo "rust-msrv: $crate on Rust $version"
  (cd "$root/$crate" && cargo "+$version" check --locked --workspace --all-targets)
done
