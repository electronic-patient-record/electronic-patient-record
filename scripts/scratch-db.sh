# Sourced, not run: a scratch database per run (LC-4, spec/local-ci.md). Used by scripts/ci-local.sh and
# tested by scripts/test_scratch_db.sh. The caller sets `root` (the repository) and `logs` (a directory for
# postgres.log).
#
# A scratch database is named epr_ci_<UTC time>_<pid>_<random>_test (the chaos test runs only on a database named like *_test), so two runs at once never share one, and a row left in a
# shared test database cannot reach a run. DB_URL, if set, names only the server: its database is used to
# create and drop the scratch one, and nothing is written to it. Without DB_URL a throwaway Postgres 18 is
# started on a free port (EPR_CI_PG_PORT to choose one) and removed after.
#
#   scratch_server && create_scratch   sets DB_URL to a new, empty scratch database, or sets why and fails
#   clean_up                           drops the scratch database, stops the throwaway server if any, and puts
#                                      DB_URL back as it was

pg_data=""
pg_bin=""
why=""
admin_url=""
scratch=""
given_url="${DB_URL-}"
given_set="${DB_URL+set}"

find_postgres() { # sets pg_bin to a directory holding Postgres 18's tools
  for dir in "$(dirname "$(command -v pg_ctl 2>/dev/null || echo /none/pg_ctl)")" \
    "$HOME/.local/share/mise/installs/postgres/18/bin"; do
    if [ -x "$dir/pg_ctl" ] && "$dir/pg_ctl" --version | grep -q ' 18'; then
      pg_bin="$dir"
      return 0
    fi
  done
  return 1
}

free_port() {
  python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])'
}

start_postgres() { # sets admin_url, or sets why
  if ! find_postgres; then
    why="no Postgres 18 found"
    return 1
  fi
  port="${EPR_CI_PG_PORT:-$(free_port)}"
  pg_data="$(mktemp -d "${TMPDIR:-/tmp}/epr-ci-pg.XXXXXX")"
  if ! { "$pg_bin/initdb" -D "$pg_data" -U loco --auth=trust -E UTF8 &&
    "$pg_bin/pg_ctl" -D "$pg_data" -w -o "-p $port -c unix_socket_directories=''" -l "$pg_data/log" start; } \
    >"$logs/postgres.log" 2>&1; then
    stop_postgres
    why="Postgres did not start: see $logs/postgres.log"
    return 1
  fi
  admin_url="postgres://loco:loco@127.0.0.1:$port/postgres"
}

stop_postgres() {
  [ -n "$pg_data" ] || return 0
  "$pg_bin/pg_ctl" -D "$pg_data" -m fast stop >/dev/null 2>&1
  rm -rf "$pg_data"
  pg_data=""
}

create_scratch() { # from admin_url: creates the scratch database and sets DB_URL to it, or sets why
  psql="${pg_bin:+$pg_bin/}psql"
  if ! command -v "$psql" >/dev/null 2>&1; then
    why="no psql found to create the scratch database"
    return 1
  fi
  scratch="epr_ci_$(date -u +%Y%m%d%H%M%S)_$$_$(od -An -N2 -tu2 /dev/urandom | tr -d " ")_test"
  query=""
  case "$admin_url" in *\?*) query="?${admin_url#*\?}" ;; esac
  base="${admin_url%%\?*}"
  url="${base%/*}/$scratch$query"
  if ! { "$psql" -v ON_ERROR_STOP=1 -d "$admin_url" -c "CREATE DATABASE \"$scratch\"" &&
    "$psql" -v ON_ERROR_STOP=1 -d "$url" -f "$root/ci/postgres-init/00-extensions.sql"; } \
    >>"$logs/postgres.log" 2>&1; then
    drop_scratch
    why="the scratch database could not be made: see $logs/postgres.log"
    return 1
  fi
  DB_URL="$url"
  export DB_URL
}

drop_scratch() {
  [ -n "$scratch" ] || return 0
  "$psql" -v ON_ERROR_STOP=1 -d "$admin_url" -c "DROP DATABASE IF EXISTS \"$scratch\" WITH (FORCE)" \
    >>"$logs/postgres.log" 2>&1 ||
    printf 'could not drop the scratch database %s: see %s/postgres.log\n' "$scratch" "$logs" >&2
  scratch=""
  if [ -n "$given_set" ]; then
    DB_URL="$given_url"
  else
    unset DB_URL
  fi
}

scratch_server() { # sets admin_url from DB_URL, or starts a throwaway server; or sets why
  if [ -n "${DB_URL:-}" ]; then
    find_postgres || true
    admin_url="$DB_URL"
  else
    start_postgres
  fi
}

clean_up() {
  drop_scratch
  stop_postgres
}
