#!/bin/sh
# One command for the gates (LC-2). See spec/local-ci.md.
#
#   sh scripts/ci-local.sh --fast    fmt, clippy, docs, database-free tests, UI lint and check, doc checks
#   sh scripts/ci-local.sh --full    adds the database tests, UI browser tests, the FHIR validator, containers
#   sh scripts/ci-local.sh --list    the gates, without running them
#
# Prints a table of gate, result and seconds, and keeps each gate's output in target/ci-local/<gate>.log.
# Every gate runs even after a failure, so one run shows everything that is wrong. Exit status: 0 all passed,
# 1 a gate failed, 2 a gate could not run. A gate that cannot run says SKIP and why, and that fails the run:
# nothing is skipped silently. EPR_CI_ALLOW_SKIP=1 lets skips exit 0, and the table still shows them.
#
# It bypasses the kache compiler cache, which has served stale libraries and fails under load. Set
# EPR_CI_KEEP_CACHE=1 to keep it. The database gates use a scratch database made for the run and dropped after:
# on the server DB_URL names, if set, or in a throwaway Postgres 18 on a free port.
#
# This proves the gates passed on this machine. It is not the CI run, and it does not replace it: cargo deny,
# CodeQL, the secret scan, the coverage floor and the fresh clone stay on GitHub.
set -u

root="$(cd "$(dirname "$0")/.." && pwd)"
service="$root/electronic-patient-record-service-with-rust"
matcher="$root/electronic-patient-record-matcher-with-rust"
ui="$root/electronic-patient-record-ui-with-svelte"
logs="$root/target/ci-local"

mode="${1:-}"
case "$mode" in
  --fast | --full | --list) ;;
  *)
    echo "usage: sh scripts/ci-local.sh --fast | --full | --list" >&2
    exit 64
    ;;
esac

if [ "${EPR_CI_KEEP_CACHE:-}" != "1" ]; then
  PATH="$(printf '%s' "$PATH" | tr ':' '\n' | grep -v 'kache/shims' | paste -sd: -)"
  RUSTC_WRAPPER=
  CC=/usr/bin/cc
  CXX=/usr/bin/c++
  HOST_CC=/usr/bin/cc
  HOST_CXX=/usr/bin/c++
  export PATH RUSTC_WRAPPER CC CXX HOST_CC HOST_CXX
  unset CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER CC_KNOWN_WRAPPER_CUSTOM
fi

rows=""
failed=0
skipped=0

record() { # name, result, seconds, note
  rows="$rows$(printf '%-24s %-5s %6s  %s' "$1" "$2" "$3" "$4")
"
}

gate() { # name, directory, command
  if [ "$mode" = "--list" ]; then
    printf '%s\n' "$1"
    return
  fi
  printf '%s ... ' "$1" >&2
  start=$(date +%s)
  if (cd "$2" && sh -c "$3") >"$logs/$1.log" 2>&1; then
    result=pass
  else
    result=FAIL
    failed=$((failed + 1))
  fi
  seconds=$(($(date +%s) - start))
  printf '%s (%ss)\n' "$result" "$seconds" >&2
  record "$1" "$result" "$seconds" ""
}

skip() { # name, why
  if [ "$mode" = "--list" ]; then
    printf '%s\n' "$1"
    return
  fi
  printf '%s ... SKIP: %s\n' "$1" "$2" >&2
  skipped=$((skipped + 1))
  record "$1" SKIP - "$2"
}

[ "$mode" = "--list" ] || mkdir -p "$logs"

gate matcher-fmt "$matcher" 'cargo fmt --check'
gate matcher-clippy "$matcher" 'cargo clippy --all-targets --all-features -- -D warnings'
gate matcher-test "$matcher" 'cargo test --locked'
gate matcher-doc "$matcher" 'RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items'
gate service-fmt "$service" 'cargo fmt --check'
gate service-clippy "$service" 'cargo clippy --all-targets --all-features -- -D warnings'
gate service-test "$service" 'cargo test --workspace --locked --no-fail-fast'
gate service-doc "$service" 'RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --workspace'
gate ui-lint "$ui" 'pnpm run lint'
gate ui-check "$ui" 'pnpm run check'
gate ui-check-tests "$ui" 'pnpm run check:tests'
gate ui-unit "$ui" 'pnpm exec vitest run'
gate docs-links "$root" 'python3 scripts/check-links.py .'
gate docs-status "$root" 'sh scripts/generate-status.sh --check'
gate docs-llms "$root" 'python3 scripts/generate-llms.py --check'
gate docs-schema-map "$root" 'python3 scripts/generate-schema-map.py --check'
gate docs-hazard-log "$root" 'python3 scripts/check-hazard-log.py --report . && python3 scripts/test_check_hazard_log.py'

# The database gates run against a scratch database made for this run and dropped after it (LC-4).
. "$root/scripts/scratch-db.sh"

if [ "$mode" != "--fast" ]; then
  trap clean_up EXIT
  trap 'clean_up; exit 130' INT TERM
  ready=yes
  if [ "$mode" = "--full" ]; then
    : >"$logs/postgres.log"
    scratch_server && create_scratch || ready=no
  fi
  if [ "$ready" = yes ]; then
    gate service-migrate "$service" 'cargo run --locked --quiet -- db migrate'
    gate service-database "$service" \
      'EPR_CHAOS_SEED=20261028 cargo test --workspace --locked --no-fail-fast -- --ignored --test-threads=1 --skip fhir_validator_'
  else
    skip service-migrate "$why"
    skip service-database "$why"
  fi
  clean_up

  if [ "$mode" = "--list" ] || find_postgres || command -v psql >/dev/null 2>&1; then
    gate scratch-db-test "$root" 'sh scripts/test_scratch_db.sh'
  else
    skip scratch-db-test "no Postgres tools found"
  fi

  if [ "$mode" = "--list" ] || [ -d "$ui/node_modules/.bin" ]; then
    gate ui-browser "$ui" "PW_PORT=${PW_PORT:-4183} CI=true pnpm exec playwright test"
  else
    skip ui-browser "run pnpm install first"
  fi

  if [ "$mode" = "--list" ] || command -v java >/dev/null 2>&1; then
    gate fhir-validator "$service" \
      'FHIR_VALIDATOR_JAR="$(sh ../scripts/fhir-validator.sh)" cargo test --test fhir_validator -- --ignored --test-threads=1'
  else
    skip fhir-validator "no java on PATH"
  fi

  if [ "$mode" = "--list" ] || podman info >/dev/null 2>&1; then
    gate container-api "$root" 'podman build -f electronic-patient-record-service-with-rust/Containerfile -t epr-api:local .'
    gate container-ui "$root" 'podman build -f electronic-patient-record-ui-with-svelte/Containerfile -t epr-ui:local .'
  else
    skip container-api "podman is not answering"
    skip container-ui "podman is not answering"
  fi
fi

[ "$mode" != "--list" ] || exit 0

printf '\n%-24s %-5s %6s  %s\n' gate result secs note
printf '%s' "$rows"
if [ "$failed" -gt 0 ]; then
  printf '\n%s failed, %s could not run. Logs: target/ci-local/\n' "$failed" "$skipped"
  exit 1
fi
if [ "$skipped" -gt 0 ] && [ "${EPR_CI_ALLOW_SKIP:-}" != "1" ]; then
  printf '\n%s could not run, so this is not a full pass.\n' "$skipped"
  exit 2
fi
printf '\nAll gates passed on this machine. CI is still the independent check.\n'
