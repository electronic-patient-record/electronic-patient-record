#!/bin/sh
# Create (or reset) a DEVELOPMENT user in the `epr` realm of the compose `keycloak` profile.
#
#   KEYCLOAK_ADMIN_PASSWORD=... DEV_USER_PASSWORD=... sh scripts/keycloak-dev-user.sh [username [role]]
#
# No password is committed anywhere: you choose both. The user gets a verified email (the API
# requires one) and, if you name one, a realm role such as clinician, nurse or auditor (the roles
# in operations/keycloak/role-map.json). With no role the user can sign in but the API refuses
# every request (403). Fictitious people only; never run this against a real Keycloak.
set -eu
user=${1:-dev.clinician}
role=${2:-}
: "${KEYCLOAK_ADMIN_PASSWORD:?set KEYCLOAK_ADMIN_PASSWORD (the password the keycloak service started with)}"
: "${DEV_USER_PASSWORD:?set DEV_USER_PASSWORD (any value; this is a throwaway development user)}"
kc() { podman compose exec -T keycloak /opt/keycloak/bin/kcadm.sh "$@"; }
kc config credentials --server http://localhost:8080 --realm master --user admin --password "$KEYCLOAK_ADMIN_PASSWORD" >/dev/null
if [ -z "$(kc get users -r epr -q "username=$user" --fields id --format csv --noquotes 2>/dev/null)" ]; then
  kc create users -r epr -s "username=$user" -s "email=$user@example.test" -s emailVerified=true -s enabled=true \
     -s firstName=Dev -s lastName=User >/dev/null
fi
kc set-password -r epr --username "$user" --new-password "$DEV_USER_PASSWORD"
if [ -n "$role" ]; then
  kc add-roles -r epr --uusername "$user" --rolename "$role"
  echo "user $user ready in realm epr with role $role"
else
  echo "user $user ready in realm epr with no role (the API will answer 403)"
fi
