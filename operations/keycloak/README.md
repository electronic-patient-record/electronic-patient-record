# Keycloak

The API accepts two kinds of bearer token: a PASETO (`v4.…`) from the authentication service, and an access token
from a **Keycloak** realm. This page says how to run Keycloak under Podman for development, point the API at it, and
what is and is not built. **Development only**: a production Keycloak sits behind TLS and is not set up here.

## What the API checks

When `EPR_KEYCLOAK_URL` is set, any bearer token that does not start `v4.` is offered to Keycloak:

- its signature, against the realm's published keys (discovery and key refresh are automatic);
- its issuer must be exactly `{EPR_KEYCLOAK_URL}/realms/{EPR_KEYCLOAK_REALM}`;
- its `aud` must contain one of `EPR_KEYCLOAK_AUDIENCES` (the realm's audience mapper adds `epr-api`);
- it must not be expired, and an `email` claim must have `email_verified: true`
  (`EPR_KEYCLOAK_REQUIRE_VERIFIED_EMAIL=false` turns that check off).

The caller's identity is the token's `sub`, which the audit trail records. A `v4.` token is never offered to Keycloak
and a Keycloak token is never offered to the PASETO verifier.

The service **refuses to start** if the Keycloak configuration is half-done: no realm, no audience, an `http://` URL
that is not loopback, or a malformed role map. The URL must be `https` except for `127.0.0.1`/`localhost`.

| Variable | Meaning |
|---|---|
| `EPR_KEYCLOAK_URL` | Server base URL, no `/realms/…`. Unset = Keycloak off |
| `EPR_KEYCLOAK_REALM` | Realm name (`epr` in the development realm) |
| `EPR_KEYCLOAK_AUDIENCES` | Comma-separated accepted `aud` values (`epr-api`) |
| `EPR_KEYCLOAK_ROLE_MAP` / `_ROLE_MAP_FILE` | JSON role → attributes map. **Accepted but not consulted yet** (see below) |
| `EPR_KEYCLOAK_REQUIRE_VERIFIED_EMAIL` | `false`/`0`/`no`/`off` to disable the email check |

## Run it (Podman)

```sh
export POSTGRES_PASSWORD=devpw KEYCLOAK_ADMIN_PASSWORD=adminpw      # throwaway values
podman compose --profile keycloak up -d --wait postgres keycloak   # http://127.0.0.1:8081
```

Keycloak keeps its state in the `keycloak` database on the compose PostgreSQL (created by `ci/postgres-init/10-keycloak.sql`
on a **fresh** volume; on an existing volume run
`podman compose exec postgres psql -U epr -d electronic_patient_record -c 'CREATE DATABASE keycloak'`). It imports the
realm `epr` from [`epr-realm.json`](epr-realm.json): clients `epr-api` (the resource server), `epr-web` (the UI, code
flow with PKCE), and `epr-dev-cli` (**development only**, password grant for scripts), each adding `epr-api` to the
token's audience; realm roles `clinician`, `auditor`, `administrator` (reserved, see below). Admin console:
http://127.0.0.1:8081 as `admin` with `KEYCLOAK_ADMIN_PASSWORD` (default: the PostgreSQL password).

## Sign in to the UI

The UI's `epr-web` client signs readers in (task SE-9). Start the API as below, then the UI on a port the client allows (3000, 4310 or
5173 on `127.0.0.1` or `localhost`; see `epr-realm.json`):

```sh
cd electronic-patient-record-ui-with-svelte && pnpm run build
EPR_API_URL=http://127.0.0.1:5150 EPR_OIDC_ISSUER=http://127.0.0.1:8081/realms/epr EPR_PUBLIC_URL=http://127.0.0.1:4310 \
  pnpm exec vite preview --host 127.0.0.1 --port 4310 --strictPort
# end-to-end, with the dev user from below: EPR_E2E_API_URL=… EPR_E2E_PASSWORD=$DEV_USER_PASSWORD \
#   pnpm exec playwright test -c playwright.keycloak.config.ts
```

Use `127.0.0.1` for both the UI and Keycloak, and pass `--binding 127.0.0.1` to the API if `localhost` resolves to `::1` here.

No user is committed. Make one, then get a token:

```sh
export DEV_USER_PASSWORD=throwaway-pw
sh scripts/keycloak-dev-user.sh dev.clinician clinician   # verified email, realm role clinician (omit the role for none: the API answers 403)
sh scripts/keycloak-dev-token.sh                # prints an access token
```

Run the API on the **host** against it (the API refuses a plain-HTTP Keycloak URL that is not loopback, so it cannot reach
`http://keycloak:8080` from inside compose):

```sh
EPR_REQUIRE_AUTH=true EPR_KEYCLOAK_URL=http://127.0.0.1:8081 EPR_KEYCLOAK_REALM=epr EPR_KEYCLOAK_AUDIENCES=epr-api \
  EPR_KEYCLOAK_ROLE_MAP_FILE=$PWD/operations/keycloak/role-map.json DATABASE_URL=… cargo run -- start --binding 127.0.0.1
curl -X POST http://127.0.0.1:5150/api/patients/compare -H "authorization: Bearer $(sh scripts/keycloak-dev-token.sh)" \
  -H 'content-type: application/json' -d '{"a":{},"b":{}}'
```

## Standalone PostgreSQL and Redis (no compose)

```sh
EPR_PG_PASSWORD=devpw sh scripts/podman-postgres.sh up        # PostgreSQL 18 on 127.0.0.1:5432, named volume
EPR_REDIS_PASSWORD=devpw sh scripts/podman-redis.sh up        # Redis 7 on 127.0.0.1:6379, no volume
sh scripts/podman-postgres.sh url; sh scripts/podman-redis.sh url
```

Ports, names and the database are set by `EPR_PG_*` and `EPR_REDIS_*` (see the scripts' headers); both bind to loopback only.
`down` removes the container; `destroy` (PostgreSQL) also deletes its data.

## What is not built

- **Roles gate routes, nothing finer.** With `EPR_KEYCLOAK_ROLE_MAP_FILE=operations/keycloak/role-map.json`, a realm role named
  like a policy role (`clinician`, `nurse`, …) becomes the token attribute `role=<name>`, and the API's route gate (SE-1) checks it. A
  valid token with no mapped role is authenticated but gets 403 everywhere; **without the role-map variable every Keycloak user gets
  403**. Consent, break-the-glass and masking are not built (SE-2 to SE-4).
- **Organisation is a role.** The audit trail and the patient's access log (SE-6) record the caller's roles and organisations. With
  Keycloak, an organisation is a realm role that `role-map.json` maps to `{"organisation": ["riverside"]}` (the development realm has
  `org-riverside`; `keycloak-dev-user.sh dev.clinician clinician,org-riverside` gives both). It grants nothing by itself. Only short
  lowercase tokens are kept.
- **UI sign-in is limited.** The UI signs in through `epr-web` (SE-9), but keeps sessions in process memory (a restart signs
  everyone out; two replicas cannot share them), does not react to Keycloak-initiated logout, and its end-to-end run is not in CI.
- **Not tested against a Keycloak that has been upgraded, clustered, put behind TLS or brokered to Entra ID.** The development
  realm was exercised only on Keycloak 26.3.0, `start-dev`, one node.
- Production hardening of Keycloak itself (TLS, hostname, secrets, backups, realm export policy) is a deployer's job.
