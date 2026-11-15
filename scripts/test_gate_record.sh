#!/bin/sh
# Checks for scripts/gate-record.sh (LC-6), in a throwaway repository with a throwaway key: a record is written and
# verified on a clean tree, refused on a dirty one, says it is self-attested in its own text, and fails to verify
# when changed, when moved to another commit, or when signed by a key not on the allowed list.
set -eu
unset EPR_CI_SIGNING_KEY EPR_CI_ALLOWED_SIGNERS 2>/dev/null || true
root="$(cd "$(dirname "$0")/.." && pwd)"
script="$root/scripts/gate-record.sh"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
fails=0

ok() { echo "ok: $1"; }
fail() {
  echo "FAIL: $1" >&2
  fails=$((fails + 1))
}

ssh-keygen -q -t ed25519 -N '' -C test -f "$work/key"
ssh-keygen -q -t ed25519 -N '' -C other -f "$work/other"
echo "test@example.invalid $(cat "$work/key.pub")" >"$work/allowed_signers"
export EPR_CI_SIGNING_KEY="$work/key" EPR_CI_ALLOWED_SIGNERS="$work/allowed_signers"

repo="$work/repo"
git init -q -b main "$repo"
cd "$repo"
git config user.email test@example.invalid
git config user.name test
git config commit.gpgsign false
echo one >file.txt
git add file.txt
git commit -q -m one
first="$(git rev-parse HEAD)"
printf 'matcher-fmt\tpass\t1\t\nui-unit\tpass\t3\t\n' >"$work/pass.tsv"
printf 'matcher-fmt\tpass\t1\t\nui-unit\tFAIL\t3\t\nui-browser\tSKIP\t-\tno browser\n' >"$work/fail.tsv"
record="target/ci-local/$first.json"

if sh "$script" write "$work/pass.tsv" >/dev/null 2>&1 && [ -f "$record" ] && [ -f "$record.sig" ]; then
  ok "a clean tree gets a signed record"
else
  fail "a clean tree gets a signed record"
fi
out="$(sh "$script" verify "$first" 2>&1)" && case "$out" in *"all passed"*) ok "it verifies, all passed" ;; *) fail "verify said: $out" ;; esac ||
  fail "a fresh record verifies: $out"
grep -q '"statement": "Self-attested, not a hosted build.' "$record" && ok "the record says it is self-attested" ||
  fail "the record says it is self-attested"
grep -q 'not evidence for the hazard log' "$record" && ok "the record says it is not safety evidence" ||
  fail "the record says it is not safety evidence"
python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); sys.exit(0 if r["commit"]==sys.argv[2] and r["host"]["arch"] and r["tools"]["python"] else 1)' \
  "$record" "$first" && ok "it names the commit, the host and the tools" || fail "it names the commit, the host and the tools"

cp "$record" "$work/saved.json"
sed -i.bak 's/"passed": true/"passed": false/' "$record" && rm -f "$record.bak"
sh "$script" verify "$first" >/dev/null 2>&1 && fail "a changed record still verifies" || ok "a changed record fails"
cp "$work/saved.json" "$record"

echo two >>file.txt
if sh "$script" write "$work/pass.tsv" >"$work/out" 2>&1; then
  fail "a dirty tree is refused"
else
  grep -q 'tracked files differ' "$work/out" && ok "a dirty tree is refused" || fail "a dirty tree is refused: $(cat "$work/out")"
fi
git commit -q -am two
second="$(git rev-parse HEAD)"
cp "$record" "target/ci-local/$second.json"
cp "$record.sig" "target/ci-local/$second.json.sig"
sh "$script" verify "$second" >"$work/out" 2>&1 && fail "a record copied to another commit verifies" ||
  { grep -q 'names commit' "$work/out" && ok "a record copied to another commit fails" || fail "copied record: $(cat "$work/out")"; }

EPR_CI_SIGNING_KEY="$work/other" sh "$script" write "$work/fail.tsv" >/dev/null 2>&1
sh "$script" verify "$second" >"$work/out" 2>&1 && fail "a key not on the allowed list verifies" ||
  { grep -q 'not in' "$work/out" && ok "a key not on the allowed list fails" || fail "other key: $(cat "$work/out")"; }

sh "$script" write "$work/fail.tsv" >/dev/null 2>&1
out="$(sh "$script" verify "$second" 2>&1)" || true
case "$out" in
  *"not passed: ui-unit FAIL, ui-browser SKIP"*) ok "a failed run is recorded as not passed, naming the gates" ;;
  *) fail "a failed run: $out" ;;
esac

if EPR_CI_SIGNING_KEY="$work/missing" sh "$script" write "$work/pass.tsv" >"$work/out" 2>&1; then
  fail "a signing failure is an error"
else
  out="$(sh "$script" verify "$second" 2>&1)" || true
  case "$out" in
    *"not passed"*) ok "a signing failure leaves the earlier record, still verifying" ;;
    *) fail "after a signing failure: $out" ;;
  esac
  ls target/ci-local/.draft-* >/dev/null 2>&1 && fail "a signing failure leaves a draft behind" || ok "and no draft is left"
fi

if [ "$fails" -ne 0 ]; then
  echo "$fails check(s) failed" >&2
  exit 1
fi
echo "all gate record checks passed"
