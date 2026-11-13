#!/bin/sh
# Tests scripts/scratch-db.sh (LC-4): two runs at once get separate databases, a leftover row in the shared test
# database does not reach a run, and every scratch database is dropped after. With DB_URL set it uses that
# server; without, it starts throwaway Postgres 18 servers, two of them at once. Exit 0 pass, 1 fail.
#
#   sh scripts/test_scratch_db.sh
set -u

root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/epr-scratch-test.XXXXXX")"
logs="$work"
failures=0

fail() {
  printf 'FAIL: %s\n' "$1"
  failures=$((failures + 1))
}

. "$root/scripts/scratch-db.sh"

# One run, as ci-local.sh does it: a scratch database, a table, a row, a pause so the other run overlaps, then a
# count. Writes its database name and its count to $work/<tag>.
one_run() { # tag
  (
    logs="$work/$1"
    mkdir -p "$logs"
    . "$root/scripts/scratch-db.sh"
    trap clean_up EXIT
    if ! { scratch_server && create_scratch; }; then
      printf 'setup failed: %s\n' "$why" >"$work/$1.err"
      exit 1
    fi
    printf '%s\n' "$scratch" >"$work/$1.name"
    "$psql" -v ON_ERROR_STOP=1 -qtA -d "$DB_URL" \
      -c "CREATE TABLE epr_ci_probe (who text)" \
      -c "INSERT INTO epr_ci_probe VALUES ('$1')" >>"$logs/psql.log" 2>&1 || exit 1
    sleep 2
    "$psql" -v ON_ERROR_STOP=1 -qtA -d "$DB_URL" \
      -c "SELECT count(*) || ':' || string_agg(who, ',') FROM epr_ci_probe" >"$work/$1.count" 2>>"$logs/psql.log"
    "$psql" -qtA -d "$DB_URL" -c "SELECT to_regclass('epr_ci_leftover') IS NULL" >"$work/$1.leftover" 2>&1
  )
}

check_run() { # tag
  [ ! -s "$work/$1.err" ] || fail "$1: $(cat "$work/$1.err")"
  [ "$(cat "$work/$1.count" 2>/dev/null)" = "1:$1" ] ||
    fail "$1 saw another run's rows: $(cat "$work/$1.count" 2>/dev/null)"
  [ "$(cat "$work/$1.leftover" 2>/dev/null)" = t ] || fail "$1 saw the shared database's leftover table"
}

# 1. Against one shared server: the one DB_URL names, or a throwaway one started here.
if ! scratch_server; then
  printf 'could not run: %s\n' "$why"
  rm -rf "$work"
  exit 1
fi
shared="$admin_url"
psql="${pg_bin:+$pg_bin/}psql"
"$psql" -v ON_ERROR_STOP=1 -qtA -d "$shared" \
  -c "CREATE TABLE IF NOT EXISTS epr_ci_leftover (who text)" \
  -c "INSERT INTO epr_ci_leftover VALUES ('left by an earlier run')" >>"$work/psql.log" 2>&1 ||
  fail "could not write the leftover row to the shared database"

DB_URL="$shared" one_run a &
DB_URL="$shared" one_run b &
wait
check_run a
check_run b
[ "$(cat "$work/a.name")" != "$(cat "$work/b.name")" ] || fail "two runs were given the same database"
left="$("$psql" -qtA -d "$shared" -c "SELECT count(*) FROM pg_database WHERE datname IN ('$(cat "$work/a.name")', '$(cat "$work/b.name")')")"
[ "$left" = 0 ] || fail "$left scratch database(s) were not dropped"
"$psql" -qtA -d "$shared" -c "DROP TABLE IF EXISTS epr_ci_leftover" >>"$work/psql.log" 2>&1
clean_up

# 2. With no DB_URL, two runs at once each start their own server on a free port.
if find_postgres; then
  (unset DB_URL; one_run c) &
  (unset DB_URL; one_run d) &
  wait
  check_run c
  check_run d
  leaked="$(find "${TMPDIR:-/tmp}" -maxdepth 1 -name 'epr-ci-pg.*' -newer "$work" 2>/dev/null | wc -l | tr -d ' ')"
  [ "$leaked" = 0 ] || fail "$leaked throwaway server directories were left behind"
else
  printf 'not run: two throwaway servers at once (no Postgres 18 found)\n'
fi

if [ "$failures" -gt 0 ]; then
  printf '%s failed; logs in %s\n' "$failures" "$work"
  exit 1
fi
rm -rf "$work"
printf 'scratch database: all checks passed\n'
