#!/bin/sh
# Run PostgreSQL 18 under Podman, standalone (no compose).
#
#   EPR_PG_PASSWORD=... sh scripts/podman-postgres.sh up      start (creates the volume on first use)
#   sh scripts/podman-postgres.sh url                         print the DATABASE_URL
#   sh scripts/podman-postgres.sh psql                        open psql in the container
#   sh scripts/podman-postgres.sh down                        stop and remove the container (keeps the data)
#   sh scripts/podman-postgres.sh destroy                     also delete the data volume
#
# Settings (environment): EPR_PG_PASSWORD (required for `up`), EPR_PG_PORT (default 5432, bound to 127.0.0.1 only),
# EPR_PG_NAME (container and volume prefix, default epr-postgres), EPR_PG_DB (default electronic_patient_record),
# EPR_PG_USER (default epr). The init scripts in ci/postgres-init run on the first start of a fresh volume only; they
# create the `keycloak` database too.
set -eu
name=${EPR_PG_NAME:-epr-postgres}
port=${EPR_PG_PORT:-5432}
db=${EPR_PG_DB:-electronic_patient_record}
user=${EPR_PG_USER:-epr}
here=$(cd "$(dirname "$0")/.." && pwd)

case "${1:-}" in
  up)
    : "${EPR_PG_PASSWORD:?set EPR_PG_PASSWORD (any value for local use)}"
    podman volume exists "$name-data" || podman volume create "$name-data" >/dev/null
    podman run -d --name "$name" \
      -e POSTGRES_USER="$user" -e POSTGRES_PASSWORD="$EPR_PG_PASSWORD" -e POSTGRES_DB="$db" \
      -p "127.0.0.1:$port:5432" \
      -v "$name-data:/var/lib/postgresql" \
      -v "$here/ci/postgres-init:/docker-entrypoint-initdb.d:ro,z" \
      --health-cmd "pg_isready -U $user -d $db" --health-interval 2s --health-retries 30 \
      docker.io/library/postgres:18-alpine >/dev/null
    until [ "$(podman inspect -f '{{.State.Health.Status}}' "$name")" = healthy ]; do sleep 1; done
    echo "PostgreSQL 18 is up: postgres://$user:<password>@127.0.0.1:$port/$db"
    ;;
  url) echo "postgres://$user:${EPR_PG_PASSWORD:-<password>}@127.0.0.1:$port/$db" ;;
  psql) exec podman exec -it "$name" psql -U "$user" -d "$db" ;;
  down) podman rm -f "$name" >/dev/null && echo "removed $name (data kept in volume $name-data)" ;;
  destroy) podman rm -f "$name" >/dev/null 2>&1 || true; podman volume rm -f "$name-data" >/dev/null && echo "removed $name and its data" ;;
  *) echo "usage: $0 up|url|psql|down|destroy" >&2; exit 2 ;;
esac
