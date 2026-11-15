#!/bin/sh
# Gate scratch-db-test (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
set -eu
cd "$(dirname "$0")/../../."
# The scratch database per local run (LC-4): two at once on one server, and a leftover row neither may see.
sh scripts/test_scratch_db.sh
