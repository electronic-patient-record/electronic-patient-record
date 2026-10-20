#!/bin/sh
# Run Redis 7 under Podman, standalone (no compose). It is the API's cache; it holds nothing that cannot be rebuilt,
# so there is no volume.
#
#   EPR_REDIS_PASSWORD=... sh scripts/podman-redis.sh up
#   sh scripts/podman-redis.sh url                      print the EPR_REDIS_URL
#   sh scripts/podman-redis.sh down
#
# Settings (environment): EPR_REDIS_PASSWORD (required for `up`), EPR_REDIS_PORT (default 6379, bound to 127.0.0.1
# only), EPR_REDIS_NAME (default epr-redis).
set -eu
name=${EPR_REDIS_NAME:-epr-redis}
port=${EPR_REDIS_PORT:-6379}

case "${1:-}" in
  up)
    : "${EPR_REDIS_PASSWORD:?set EPR_REDIS_PASSWORD (any value for local use)}"
    podman run -d --name "$name" -e REDIS_PASS="$EPR_REDIS_PASSWORD" -p "127.0.0.1:$port:6379" \
      --health-cmd 'redis-cli -a "$REDIS_PASS" --no-auth-warning ping | grep -q PONG' --health-interval 2s --health-retries 30 \
      docker.io/library/redis:7-alpine \
      redis-server --requirepass "$EPR_REDIS_PASSWORD" --save "" --appendonly no --maxmemory 128mb --maxmemory-policy allkeys-lru >/dev/null
    until [ "$(podman inspect -f '{{.State.Health.Status}}' "$name")" = healthy ]; do sleep 1; done
    echo "Redis 7 is up on 127.0.0.1:$port"
    ;;
  url) echo "redis://:${EPR_REDIS_PASSWORD:-<password>}@127.0.0.1:$port/0" ;;
  down) podman rm -f "$name" >/dev/null && echo "removed $name" ;;
  *) echo "usage: $0 up|url|down" >&2; exit 2 ;;
esac
