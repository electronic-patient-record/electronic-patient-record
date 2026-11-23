#!/bin/sh
# One command for the gates (LC-2). See spec/local-ci.md.
#
#   sh scripts/ci-local.sh --fast    fmt, clippy, docs, database-free tests, UI lint and check, doc checks
#   sh scripts/ci-local.sh --full    adds the database tests, UI browser tests, the FHIR validator, containers
#   sh scripts/ci-local.sh --list    the gates, without running them
#   sh scripts/ci-local.sh --verify <commit>   check the signed record a --full run wrote for that commit (LC-6)
#
# Prints a table of gate, result and seconds, and keeps each gate's output in target/ci-local/<gate>.log.
# Every gate runs even after a failure, so one run shows everything that is wrong. Exit status: 0 all passed,
# 1 a gate failed, 2 a gate could not run, 3 the preflight (--full only, scripts/preflight.sh) found a problem.
# A gate that cannot run says SKIP and why, and that fails the run: nothing is skipped silently. EPR_CI_ALLOW_SKIP=1 lets skips exit 0, and the table still shows them.
#
# It bypasses the kache compiler cache, which has served stale libraries and fails under load. Set
# EPR_CI_KEEP_CACHE=1 to keep it. The database gates use a scratch database made for the run and dropped after:
# on the server DB_URL names, if set, or in a throwaway Postgres 18 on a free port.
#
# EPR_CI_ONLY="matcher service ui swift kotlin docs" (any of the six, space-separated) runs only those projects' gates; the
# others show in the table as "off", so a narrowed run never reads as a whole one. The pre-push hook sets it (LC-5).
#
# This proves the gates passed on this machine. It is not the CI run, and it does not replace it: cargo deny,
# CodeQL, the secret scan, the coverage floor and the fresh clone stay on GitHub.
set -u

# Run from a git hook (pre-push), GIT_DIR and the like name this repository; a gate that makes a throwaway repository,
# or works in another one, would act on this one instead (on 2026-11-23 a test's `git init` set core.bare=true here).
unset $(git rev-parse --local-env-vars) 2>/dev/null || true

root="$(cd "$(dirname "$0")/.." && pwd)"
service="$root/electronic-patient-record-service-with-rust"
matcher="$root/electronic-patient-record-matcher-with-rust"
ui="$root/electronic-patient-record-ui-with-svelte"
logs="$root/target/ci-local"

mode="${1:-}"
case "$mode" in
  --fast | --full | --list) ;;
  --verify) cd "$root" && exec sh scripts/gate-record.sh verify "${2:?usage: sh scripts/ci-local.sh --verify <commit>}" ;;
  *)
    echo "usage: sh scripts/ci-local.sh --fast | --full | --list | --verify <commit>" >&2
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

only="${EPR_CI_ONLY:-}"
for project in $only; do
  case "$project" in
    matcher | service | ui | swift | kotlin | docs) ;;
    *)
      echo "EPR_CI_ONLY: unknown project '$project' (use matcher, service, ui, swift, kotlin, docs)" >&2
      exit 64
      ;;
  esac
done

project_of() { # gate name -> the project it checks
  case "$1" in
    matcher-* | coverage-matcher) echo matcher ;;
    service-* | scratch-* | fhir-* | container-api | coverage-service | rust-msrv) echo service ;;
    ui-* | container-ui) echo ui ;;
    mobile-swift-*) echo swift ;;
    mobile-kotlin-*) echo kotlin ;;
    *) echo docs ;;
  esac
}

selected() { # gate name
  [ -z "$only" ] && return 0
  case " $only " in *" $(project_of "$1") "*) return 0 ;; esac
  return 1
}

rows=""
failed=0
skipped=0
off=0

record() { # name, result, seconds, note
  rows="$rows$(printf '%-24s %-5s %6s  %s' "$1" "$2" "$3" "$4")
"
  [ "$mode" = "--list" ] || printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" >>"$logs/results.tsv"
}

gate() { # name, directory, command
  if [ "$mode" = "--list" ]; then
    selected "$1" && printf '%s\n' "$1"
    return
  fi
  if ! selected "$1"; then
    off=$((off + 1))
    record "$1" off - "not in EPR_CI_ONLY"
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

gate_script() { # name: the gate's one definition, scripts/gates/<name>.sh, also run by ci.yml (LC-3)
  gate "$1" "$root" "sh scripts/gates/$1.sh"
}

skip() { # name, why
  if [ "$mode" = "--list" ]; then
    selected "$1" && printf '%s\n' "$1"
    return
  fi
  if ! selected "$1"; then
    off=$((off + 1))
    record "$1" off - "not in EPR_CI_ONLY"
    return
  fi
  printf '%s ... SKIP: %s\n' "$1" "$2" >&2
  skipped=$((skipped + 1))
  record "$1" SKIP - "$2"
}

[ "$mode" = "--list" ] || { mkdir -p "$logs" && : >"$logs/results.tsv"; }
# A --full run on a clean tree writes a signed record of its gates (LC-6): HEAD now, and whether tracked files match it.
start_head="$(git -C "$root" rev-parse HEAD 2>/dev/null || true)"
start_clean="$([ -z "$(git -C "$root" status --porcelain --untracked-files=no 2>/dev/null)" ] && echo yes || echo no)"

# The traps that have failed a full run here, checked before any gate (LC-12): ports, the validator jar, the Podman
# VM's clock, memory and disk. Exit 3 names each problem and its fix.
if [ "$mode" = "--full" ] && [ "${EPR_CI_SKIP_PREFLIGHT:-}" != "1" ]; then
  sh "$root/scripts/preflight.sh" || exit 3
fi

gate_script matcher-fmt
gate_script matcher-clippy
gate_script matcher-test
gate_script matcher-doc
gate_script service-fmt
gate_script service-clippy
gate_script service-checked-arithmetic
gate_script service-clippy-arithmetic
gate_script service-test
gate_script service-doc
gate_script ui-lint
gate_script ui-check
gate_script ui-check-tests
gate_script ui-unit

# The mobile apps (spec/mobile-apps, MP-4): tests here, builds in --full. Each needs its platform's tools, so a machine
# without them says SKIP and why. The iOS app needs Xcode (macOS); the Android app a JDK and the Android SDK.
have_xcode() { command -v xcodebuild >/dev/null 2>&1 && xcodebuild -version >/dev/null 2>&1; }
have_android() {
  command -v java >/dev/null 2>&1 || return 1
  [ -n "${ANDROID_HOME:-}" ] && [ -d "$ANDROID_HOME" ] && return 0
  [ -d "$HOME/Library/Android/sdk" ] || [ -d "$HOME/Android/Sdk" ]
}
if [ "$mode" = "--list" ] || have_xcode; then
  gate_script mobile-swift-test
else
  skip mobile-swift-test "Xcode is not installed (the iOS app builds on macOS only)"
fi
if [ "$mode" = "--list" ] || have_android; then
  gate_script mobile-kotlin-test
else
  skip mobile-kotlin-test "no java on PATH, or no Android SDK (set ANDROID_HOME)"
fi
gate_script docs-links
gate_script docs-status
gate_script docs-llms
gate_script docs-schema-map
gate_script docs-hazard-log
gate_script docs-hooks
gate_script docs-preflight
gate_script docs-ci-changes
gate_script docs-gate-record
gate_script docs-gates
gate_script docs-load-harness

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
    gate_script service-database
  else
    skip service-migrate "$why"
    skip service-database "$why"
  fi
  clean_up

  if [ "$mode" = "--list" ] || find_postgres || command -v psql >/dev/null 2>&1; then
    gate_script scratch-db-test
  else
    skip scratch-db-test "no Postgres tools found"
  fi

  # Performance (spec/performance-testing.md): every load scenario briefly on a scratch database, and every
  # benchmark once. They check that it works and nothing fails; they measure nothing worth keeping.
  if [ "$mode" = "--list" ] || find_postgres || command -v psql >/dev/null 2>&1; then
    gate_script service-load-smoke
  else
    skip service-load-smoke "no Postgres tools found"
  fi
  gate_script matcher-bench
  gate_script service-bench
  # Each crate on the oldest Rust it claims (QA-8): another toolchain's build, so --full only.
  if [ "$mode" = "--list" ] || command -v rustup >/dev/null 2>&1; then
    gate_script rust-msrv
  else
    skip rust-msrv "rustup is not installed"
  fi

  if [ "$mode" = "--list" ] || have_xcode; then
    gate_script mobile-swift-build
  else
    skip mobile-swift-build "Xcode is not installed (the iOS app builds on macOS only)"
  fi
  if [ "$mode" = "--list" ] || have_android; then
    gate_script mobile-kotlin-build
  else
    skip mobile-kotlin-build "no java on PATH, or no Android SDK (set ANDROID_HOME)"
  fi

  if [ "$mode" = "--list" ] || [ -d "$ui/node_modules/.bin" ]; then
    gate ui-browser "$root" "PW_PORT=${PW_PORT:-4183} sh scripts/gates/ui-browser.sh"
  else
    skip ui-browser "run pnpm install first"
  fi

  if [ "$mode" = "--list" ] || command -v cargo-tarpaulin >/dev/null 2>&1; then
    gate_script coverage-matcher
    gate_script coverage-service
  else
    skip coverage-matcher "cargo-tarpaulin is not installed (cargo install cargo-tarpaulin --locked)"
    skip coverage-service "cargo-tarpaulin is not installed (cargo install cargo-tarpaulin --locked)"
  fi

  if [ "$mode" = "--list" ] || command -v java >/dev/null 2>&1; then
    gate_script fhir-validator
  else
    skip fhir-validator "no java on PATH"
  fi

  if [ "$mode" = "--list" ] || podman info >/dev/null 2>&1; then
    gate_script container-api
    gate_script container-ui
  else
    skip container-api "podman is not answering"
    skip container-ui "podman is not answering"
  fi
fi

[ "$mode" != "--list" ] || exit 0

if [ "$mode" = "--full" ]; then
  if [ "$start_clean" != yes ]; then
    printf '\nNo gate record: tracked files differed from HEAD when the run started.\n'
  elif [ "$(git -C "$root" rev-parse HEAD)" != "$start_head" ]; then
    printf '\nNo gate record: HEAD moved during the run.\n'
  else
    printf '\n'
    (cd "$root" && sh scripts/gate-record.sh write "$logs/results.tsv") || true
  fi
fi

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
if [ "$off" -gt 0 ]; then
  printf '\nThe gates run passed on this machine; %s were off (EPR_CI_ONLY="%s"). CI is still the independent check.\n' "$off" "$only"
  exit 0
fi
printf '\nAll gates passed on this machine. CI is still the independent check.\n'
