#!/bin/sh
# Gate service-load-smoke (LC-3): the one definition, run by .github/workflows/ci.yml and scripts/ci-local.sh.
# Every load scenario briefly against a release build on a scratch database: each works and none fails. Latency
# budgets are not checked here, since a shared machine's numbers are noise (spec/performance-testing.md).
set -eu
cd "$(dirname "$0")/../.."
sh scripts/load/run-local.sh --profile smoke --out target/load/smoke.json
