#!/bin/sh
# Which CI jobs a set of changed files needs (LC-7). Reads paths, one per line, on stdin and prints
#
#   rust=true|false    the matcher, the service and everything that builds or tests them
#   ui=true|false      the clinician UI
#
# A path it does not know sets both, so a new kind of file runs everything until it is listed here. Documents set
# neither, except the ones a project's tests read: those set that project. `scripts/test_ci_changes.sh` scans the
# tests for files they read outside their own directory and fails if one would set nothing, so this list cannot
# fall behind a new census test. The docs, secret-scan, toolchain and hazard-log jobs run on every push regardless,
# and the nightly run (`schedule` in ci.yml) runs everything.
set -eu
rust=false
ui=false
while IFS= read -r path; do
  [ -n "$path" ] || continue
  case "$path" in
    # Read by the service's tests, or built into the service.
    SECURITY.md | governance/personal-data-inventory.csv | packs/* | ci/* | rust-toolchain.toml | Cargo.lock)
      rust=true ;;
    # Read by the UI's tests.
    governance/production-readiness.md | governance/translation-review.md | operations/keycloak/*)
      ui=true ;;
    # Read by the UI's tests (adjustments.test.ts keeps the UI's list equal to the service's).
    electronic-patient-record-service-with-rust/src/adjustments.rs)
      rust=true
      ui=true ;;
    electronic-patient-record-matcher-with-rust/* | electronic-patient-record-service-with-rust/*)
      rust=true ;;
    # The service's census tests read the UI's source (risk_field_census, grief_census), so that runs both.
    electronic-patient-record-ui-with-svelte/src/*)
      ui=true
      rust=true ;;
    electronic-patient-record-ui-with-svelte/*)
      ui=true ;;
    # Documents that no build or test reads: the docs job checks them.
    *.md | spec/* | agents/* | docs/* | locales/* | governance/* | llms.txt | llms.json | CITATION.cff | LICENSE | CODEOWNERS)
      ;;
    *)
      rust=true
      ui=true ;;
  esac
done
echo "rust=$rust"
echo "ui=$ui"
