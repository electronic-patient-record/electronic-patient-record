#!/bin/sh
# Gate container-api (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../."
# The tag is the first argument: CI passes epr-api:ci.
podman build -f electronic-patient-record-service-with-rust/Containerfile -t "${1:-epr-api:local}" .
