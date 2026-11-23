#!/bin/sh
# Checks for the pre-push hook (LC-5) and ci-local.sh's EPR_CI_ONLY. The hook's choice of projects is tested in a
# throwaway repository whose commits touch known paths, with EPR_PRE_PUSH_DRY_RUN=1, so no gate runs.
# The real run (a badly formatted Rust file stopped before the push) is in spec/local-ci.md, done by hand.
set -eu
# It runs inside the hook too (as the docs-hooks gate), where git and the hook have set these.
unset EPR_CI_ONLY EPR_PRE_PUSH_DRY_RUN GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE 2>/dev/null || true
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
fails=0

check() { # what, expected, actual
  if [ "$2" = "$3" ]; then
    echo "ok: $1"
  else
    echo "FAIL: $1: expected '$2', got '$3'" >&2
    fails=$((fails + 1))
  fi
}

repo="$work/repo"
git init -q -b main "$repo"
cd "$repo"
git config user.email test@example.invalid
git config user.name test
git config commit.gpgsign false
mkdir -p .githooks
cp "$root/.githooks/pre-push" .githooks/pre-push

commit() { # path, message
  mkdir -p "$(dirname "$1")"
  echo "$2" >>"$1"
  git add "$1"
  git commit -q -m "$2"
  git rev-parse HEAD
}

base="$(commit README.md base)"
git update-ref refs/remotes/origin/main "$base"
docs="$(commit tasks.md docs)"
matcher="$(commit electronic-patient-record-matcher-with-rust/src/lib.rs matcher)"
ui="$(commit electronic-patient-record-ui-with-svelte/src/app.ts ui)"
service="$(commit electronic-patient-record-service-with-rust/src/app.rs service)"
toolchain="$(commit rust-toolchain.toml toolchain)"
swift="$(commit electronic-patient-record-mobile-app-with-swift/Package.swift swift)"
kotlin="$(commit electronic-patient-record-mobile-app-with-kotlin/app/build.gradle.kts kotlin)"
lily="$(commit scripts/fetch-lily.sh lily)"
zero=0000000000000000000000000000000000000000

projects() { # stdin lines for the hook
  EPR_PRE_PUSH_DRY_RUN=1 sh .githooks/pre-push origin https://example.invalid/repo.git
}

check "documents only" "docs" "$(echo "refs/heads/main $docs refs/heads/main $base" | projects)"
check "matcher" "matcher docs" "$(echo "refs/heads/main $matcher refs/heads/main $docs" | projects)"
check "matcher and ui" "matcher ui docs" "$(echo "refs/heads/main $ui refs/heads/main $docs" | projects)"
check "service" "service docs" "$(echo "refs/heads/main $service refs/heads/main $ui" | projects)"
check "the toolchain pin is both Rust projects'" "matcher service docs" \
  "$(echo "refs/heads/main $toolchain refs/heads/main $service" | projects)"
check "a new branch is compared with origin/main" "matcher service ui docs" \
  "$(echo "refs/heads/topic $toolchain refs/heads/topic $zero" | projects)"
check "the iOS app" "swift docs" "$(echo "refs/heads/main $swift refs/heads/main $toolchain" | projects)"
check "the Android app" "kotlin docs" "$(echo "refs/heads/main $kotlin refs/heads/main $swift" | projects)"
check "the Lily pin is both apps'" "swift kotlin docs" "$(echo "refs/heads/main $lily refs/heads/main $kotlin" | projects)"
check "a deleted branch checks nothing" "" "$(echo "(delete) $zero refs/heads/topic $service" | projects)"
check "nothing new checks nothing" "" "$(echo "refs/heads/main $service refs/heads/main $service" | projects)"
check "an unknown remote commit checks everything" "matcher service ui swift kotlin docs" \
  "$(echo "refs/heads/main $service refs/heads/main 1111111111111111111111111111111111111111" | projects)"
check "two refs at once" "matcher service docs" \
  "$(printf '%s\n' "refs/heads/a $matcher refs/heads/a $docs" "refs/heads/b $service refs/heads/b $ui" | projects)"

cd "$root"
check "EPR_CI_ONLY narrows --list" "matcher-fmt matcher-clippy matcher-test matcher-doc matcher-bench coverage-matcher" \
  "$(EPR_CI_ONLY=matcher sh scripts/ci-local.sh --list | paste -sd' ' -)"
check "EPR_CI_ONLY unset lists every project's gates" "matcher-fmt service-fmt ui-lint mobile-swift-test mobile-kotlin-test docs-links" \
  "$(sh scripts/ci-local.sh --list | grep -E '^(matcher-fmt|service-fmt|ui-lint|mobile-swift-test|mobile-kotlin-test|docs-links)$' | paste -sd' ' -)"
check "EPR_CI_ONLY narrows --list to the iOS app" "mobile-swift-test mobile-swift-build" \
  "$(EPR_CI_ONLY=swift sh scripts/ci-local.sh --list | paste -sd' ' -)"
check "EPR_CI_ONLY narrows --list to the Android app" "mobile-kotlin-test mobile-kotlin-build" \
  "$(EPR_CI_ONLY=kotlin sh scripts/ci-local.sh --list | paste -sd' ' -)"
status=0
EPR_CI_ONLY=bogus sh scripts/ci-local.sh --fast >/dev/null 2>&1 || status=$?
check "an unknown project is refused" "64" "$status"

if [ "$fails" -ne 0 ]; then
  echo "$fails check(s) failed" >&2
  exit 1
fi
echo "all pre-push checks passed"
