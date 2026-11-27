#!/bin/sh
# Which CI jobs a set of changed files needs (LC-7). Reads paths, one per line, on stdin and prints
#
#   rust=true|false    the matcher, the service and everything that builds or tests them
#   ui=true|false      the clinician UI
#   swift=true|false   the iOS app (MP-4)
#   kotlin=true|false  the Android app (MP-4)
#
# A path it does not know sets all four, so a new kind of file runs everything until it is listed here. Documents set
# neither, except the ones a project's tests read: those set that project. `scripts/test_ci_changes.sh` scans the
# tests for files they read outside their own directory and fails if one would set nothing, so this list cannot
# fall behind a new census test. The docs, secret-scan, toolchain and hazard-log jobs run on every push regardless,
# and the nightly run (`schedule` in ci.yml) runs everything.
set -eu
rust=false
ui=false
swift=false
kotlin=false
while IFS= read -r path; do
  [ -n "$path" ] || continue
  case "$path" in
    # Read by the service's tests, or built into the service.
    governance/personal-data-inventory.csv | packs/* | ci/* | rust-toolchain.toml | Cargo.lock)
      rust=true ;;
    # Read by the service's licence census (tests/licence_files.rs).
    LICENSE.md | LICENSES/*)
      rust=true ;;
    # Read by the service's documentation census and the UI's security.txt test.
    SECURITY.md)
      rust=true
      ui=true ;;
    # Read by the UI's security.txt test (its review date).
    docs/security/policy.md)
      ui=true ;;
    # Read by the UI's tests.
    governance/production-readiness.md | governance/translation-review.md | operations/keycloak/*)
      ui=true ;;
    # Read by the UI's tests (adjustments.test.ts keeps the UI's list equal to the service's).
    electronic-patient-record-service-with-rust/electronic-patient-record-domain/src/adjustments.rs)
      rust=true
      ui=true ;;
    # Read by the UI's tests (decision-ability.test.ts keeps the UI's gap words equal to the service's).
    electronic-patient-record-service-with-rust/electronic-patient-record-domain/src/decision_ability.rs)
      rust=true
      ui=true ;;
    # Read by the UI's tests (summary.test.ts holds what the summary says it does not show to the service's routes).
    electronic-patient-record-service-with-rust/src/routes.rs)
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
    # The mobile apps (spec/mobile-apps): each app's own files and gates; the pinned Lily checkout is both apps'.
    electronic-patient-record-mobile-app-with-swift/* | scripts/gates/mobile-swift-*)
      swift=true ;;
    electronic-patient-record-mobile-app-with-kotlin/* | scripts/gates/mobile-kotlin-*)
      kotlin=true ;;
    scripts/fetch-lily.sh)
      swift=true
      kotlin=true ;;
    # Documents that no build or test reads: the docs job checks them.
    *.md | spec/* | agents/* | docs/* | locales/* | governance/* | llms.txt | llms.json | CITATION.cff | CODEOWNERS)
      ;;
    *)
      rust=true
      ui=true
      swift=true
      kotlin=true ;;
  esac
done
echo "rust=$rust"
echo "ui=$ui"
echo "swift=$swift"
echo "kotlin=$kotlin"
