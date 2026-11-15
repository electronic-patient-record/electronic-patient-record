#!/bin/sh
# Gate docs-gates (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../.."
python3 scripts/check-gates.py
sh scripts/test_check_gates.sh
