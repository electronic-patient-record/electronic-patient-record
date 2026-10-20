# Security policy

## Reporting a vulnerability

Please report a suspected vulnerability **privately**, not in a public issue.

- Email **joel@joelparkerhenderson.com** with the subject `SECURITY: electronic-patient-record`.
- Say what you found, how to reproduce it, which version or commit, and what you
  think the impact is. A proof of concept helps; please do not use it against
  anyone's live system.
- If the report involves personal data, describe it without attaching real
  personal data.

There is one maintainer (see [MAINTAINERS.md](MAINTAINERS.md)). The intent is
to acknowledge a report within **5 business days** and to tell you what will be
done within **15 business days**; these are targets, not guarantees. A fix is
released as a patch version and noted in the changelog, crediting you if you
wish. No bug bounty is offered.

## Supported versions

Only the latest release, and the `main` branch, receive security fixes. The
project is pre-1.0 and changes quickly.

## What the software already does

Checked by tests where a test is named (this is a pre-release scaffold, see
[`spec/14-implementation-status.md`](spec/14-implementation-status.md)):

- Login enforcement is **on** by default; the service refuses to start without
  token verification keys, and refuses `production` with login off
  (`auth::tests`).
- Health probes are the only unauthenticated paths (`auth::tests`).
- The audit table refuses `UPDATE`, `DELETE` and `TRUNCATE`
  (`tests/audit_immutable.rs`, needs Postgres).
- Security headers and a strict Content Security Policy on the UI
  (`tests/e2e/smoke.spec.ts`).
- The UI's backend-for-frontend proxy refuses any path outside `/api`.
- `cargo deny` runs in CI; Dependabot watches the dependencies.

## What it does not claim

**This software is not for clinical use and holds no clinical record yet.** No
third party has audited it, and no certification (ONC, DTAC, DSPT, DCB0129,
HIPAA) is claimed. Authorization beyond "has a valid token" (roles, consent,
break-the-glass), audit writing on every access, and sign-in are planned and
not built (`tasks.md`: SE-*, AU-*). A deployer is responsible for their own
configuration, keys, backups and assessment.

## Scope

In scope: the code in this repository and its default configuration. Out of
scope: your deployment's infrastructure, third-party services (an identity
provider, a database host), and denial-of-service by volume.
