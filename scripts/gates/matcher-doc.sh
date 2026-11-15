#!/bin/sh
# Gate matcher-doc (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../electronic-patient-record-matcher-with-rust"
# Private items included: a broken link or an undocumented public item fails.
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
