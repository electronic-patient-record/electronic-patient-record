#!/bin/sh
# Print an access token for a development user (password grant through the DEV-ONLY `epr-dev-cli` client).
#
#   DEV_USER_PASSWORD=... sh scripts/keycloak-dev-token.sh [username]
#
# Use it:  curl -H "Authorization: Bearer $(sh scripts/keycloak-dev-token.sh)" http://127.0.0.1:5150/...
set -eu
user=${1:-dev.clinician}
: "${DEV_USER_PASSWORD:?set DEV_USER_PASSWORD}"
base=${KEYCLOAK_URL:-http://127.0.0.1:8081}
curl -fsS "$base/realms/epr/protocol/openid-connect/token" \
  -d grant_type=password -d client_id=epr-dev-cli -d "username=$user" --data-urlencode "password=$DEV_USER_PASSWORD" \
  | sed -n 's/.*"access_token":"\([^"]*\)".*/\1/p'
