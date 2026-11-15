#!/bin/sh
# Gate docs-gate-record (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../.."
sh scripts/test_gate_record.sh
