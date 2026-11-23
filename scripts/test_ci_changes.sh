#!/bin/sh
# Checks for scripts/ci-changes.sh (LC-7).
#
# 1. Fixed cases: documents skip the build jobs, each project's files run its jobs, an unknown file runs everything.
# 2. A census of what the tests read: every file outside a project's own directory that its tests or source name
#    (a "../path" literal, or a join("dir/...") from the repository root) must run that project's jobs when it
#    changes. A new test that reads a document fails here until ci-changes.sh lists the document.
set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
fails=0

classify() { printf '%s\n' "$@" | sh scripts/ci-changes.sh | paste -sd' ' -; }

check() { # what, expected, paths...
  what="$1"
  expected="$2"
  shift 2
  got="$(classify "$@")"
  if [ "$got" = "$expected" ]; then
    echo "ok: $what"
  else
    echo "FAIL: $what: expected '$expected', got '$got'" >&2
    fails=$((fails + 1))
  fi
}

check "nothing changed" "rust=false ui=false swift=false kotlin=false"
check "documents only" "rust=false ui=false swift=false kotlin=false" tasks.md plan.md spec/16-open-questions.md governance/hazard-log/x.md \
  locales/en-us/communications/github-discussions/general.txt llms.txt
check "the hazard log alone" "rust=false ui=false swift=false kotlin=false" governance/clinical-safety/hazard-log.csv
check "the matcher" "rust=true ui=false swift=false kotlin=false" electronic-patient-record-matcher-with-rust/src/lib.rs
check "a service document" "rust=true ui=false swift=false kotlin=false" electronic-patient-record-service-with-rust/spec/index.md
check "the UI's tests" "rust=false ui=true swift=false kotlin=false" electronic-patient-record-ui-with-svelte/tests/e2e/smoke.spec.ts
check "the UI's source (read by the service's censuses)" "rust=true ui=true swift=false kotlin=false" \
  electronic-patient-record-ui-with-svelte/src/lib/strings.ts
check "the workflow itself" "rust=true ui=true swift=true kotlin=true" .github/workflows/ci.yml
check "a script" "rust=true ui=true swift=true kotlin=true" scripts/ci-local.sh
check "an unknown file" "rust=true ui=true swift=true kotlin=true" something-new.yaml
check "a document among code" "rust=true ui=false swift=false kotlin=false" README.md electronic-patient-record-service-with-rust/src/app.rs
check "the iOS app" "rust=false ui=false swift=true kotlin=false" electronic-patient-record-mobile-app-with-swift/Sources/EPRCore/Banner.swift
check "the Android app" "rust=false ui=false swift=false kotlin=true" electronic-patient-record-mobile-app-with-kotlin/app/build.gradle.kts
check "an iOS gate" "rust=false ui=false swift=true kotlin=false" scripts/gates/mobile-swift-test.sh
check "an Android gate" "rust=false ui=false swift=false kotlin=true" scripts/gates/mobile-kotlin-build.sh
check "the Lily pin is both apps'" "rust=false ui=false swift=true kotlin=true" scripts/fetch-lily.sh
check "the mobile contract is a document" "rust=false ui=false swift=false kotlin=false" spec/mobile-apps/index.md

# The census: (project, path read) for every read outside the project's directory.
reads="$(python3 - <<'EOF'
import os, re
root = os.getcwd()
projects = {
    "electronic-patient-record-matcher-with-rust": "rust",
    "electronic-patient-record-service-with-rust": "rust",
    "electronic-patient-record-ui-with-svelte": "ui",
}
up = re.compile(r'"\.\./([^"]+)"')
joined = re.compile(r'join\(\s*"([^"]+)"\s*\)')
found = set()
for project, flag in projects.items():
    for base in ("src", "tests"):
        top = os.path.join(root, project, base)
        for dirpath, _, files in os.walk(top):
            for name in files:
                if not name.endswith((".rs", ".ts")):
                    continue
                text = open(os.path.join(dirpath, name), encoding="utf-8").read()
                for path in up.findall(text):
                    if not path.startswith("../") and os.path.exists(os.path.join(root, path)):
                        found.add((flag, path))
                for path in joined.findall(text):
                    first = path.split("/")[0]
                    if (first and os.path.exists(os.path.join(root, first))
                            and not os.path.exists(os.path.join(root, project, first))
                            and os.path.exists(os.path.join(root, path))):
                        found.add((flag, path))
for flag, path in sorted(found):
    if os.path.isdir(os.path.join(root, path)):
        path = path.rstrip("/") + "/any"
    print(flag, path)
EOF
)"
count=0
while read -r flag path; do
  [ -n "$flag" ] || continue
  count=$((count + 1))
  case " $(classify "$path") " in
    *" $flag=true "*) ;;
    *)
      echo "FAIL: $path is read by the $flag tests, and a change to it would not run them (list it in scripts/ci-changes.sh)" >&2
      fails=$((fails + 1))
      ;;
  esac
done <<EOF
$reads
EOF
if [ "$count" -lt 5 ]; then
  echo "FAIL: the census found only $count reads outside a project; the scan is broken" >&2
  fails=$((fails + 1))
else
  echo "ok: $count files read across projects each run the projects that read them"
fi

if [ "$fails" -ne 0 ]; then
  echo "$fails check(s) failed" >&2
  exit 1
fi
echo "all ci-changes checks passed"
