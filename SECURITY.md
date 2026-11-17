# Security policy

This is the project's vulnerability disclosure policy: how to report a vulnerability, what happens next, and what you
can expect. It follows the shape of ISO/IEC 29147 (disclosure) and ISO/IEC 30111 (handling), and the machine-readable
contact file of RFC 9116 is served by the clinician UI at `/.well-known/security.txt`. The rules the project holds
itself to while it builds the software are in the [security policy](docs/security/policy.md).

| | |
|---|---|
| Policy version | 2.0, 2026-11-17 |
| Owner | The maintainer ([MAINTAINERS.md](MAINTAINERS.md)) |
| Next review | 2027-05-17, or sooner after any report that the policy handled badly |

**This software is not for clinical use and holds no clinical record yet.** A report is still welcome and is handled
as below: the point of building the process now is that it works before anyone depends on it.

## 1. Reporting a vulnerability

Report a suspected vulnerability **privately**, never in a public issue, pull request, discussion or commit.

- **Email:** **joel@joelparkerhenderson.com**, subject `SECURITY: electronic-patient-record`.
- **Include:** what you found; how to reproduce it; the version, tag or commit; what an attacker could do with it; and
  how you would like to be credited, if at all. A proof of concept helps.
- **Personal data:** describe it, never attach it. If you came across real personal data, stop, tell us, and keep no
  copy.
- **Encryption:** if you need to send something sensitive, say so in a first email and a key will be agreed.

Or use GitHub's **private vulnerability reporting**: [report a vulnerability](https://github.com/electronic-patient-record/electronic-patient-record/security/advisories/new) (switched on 2026-11-17). The
report is seen only by the maintainers, and the advisory can be drafted, fixed in a private fork and published from it.
Either channel gets the same targets below.

## 2. What happens next

| Stage | Target | What you get |
|---|---|---|
| Acknowledgement | 5 business days | A reply saying the report arrived and who is handling it |
| Triage | 15 business days | Whether it is accepted as a vulnerability, its severity, and the plan |
| Fix | By severity, below | A fix or a mitigation in a release, or a reason it will not be fixed |
| Disclosure | With the fix, or at 90 days | A published advisory, crediting you if you wish |

Severity is rated with CVSS v4.0, then raised where the harm could be clinical: a flaw that could show a clinician the
wrong patient's record, or change what they decide or sign, is treated as at least **high** whatever its score, and
goes to the Clinical Safety Officer as a hazard ([`governance/clinical-safety/`](governance/clinical-safety/)).

| Severity | CVSS v4.0 | Fix or mitigation target |
|---|---|---|
| Critical | 9.0 to 10.0 | 7 days |
| High | 7.0 to 8.9 | 30 days |
| Medium | 4.0 to 6.9 | 90 days |
| Low | 0.1 to 3.9 | The next planned release |

There is one maintainer. These are **targets, not guarantees**, and a report that misses one is told so, with a new
date, rather than left waiting.

## 3. Coordinated disclosure

- We ask you to keep the vulnerability private until a fix is released or 90 days have passed since your report,
  whichever comes first. Either side may ask to change that date; a fix that needs a deployer's action may need longer,
  and a vulnerability being exploited may need less.
- The advisory is published as a GitHub security advisory, with a CVE requested for anything rated medium or above,
  and the fix is noted in the [changelog](CHANGELOG.md).
- You are credited by the name you give, or not at all if you prefer. No bug bounty is offered.

## 4. Safe harbour

Research done in good faith under this policy is authorised: we will not pursue legal action or a complaint against
you for it. Good faith means you:

- test only against your own installation of this software, never anyone else's live system;
- do not access, keep, change or destroy data that is not yours, and stop as soon as you reach any;
- do not degrade a service for others (no denial of service, no spam, no social engineering of people);
- report promptly and keep the details private as in section 3.

This cannot authorise testing of a deployment that someone else runs: ask its operator.

## 5. Supported versions

Only the latest release and the `main` branch receive security fixes. The project is pre-1.0 and changes quickly; an
older tag is not patched.

## 6. Scope

**In scope:** the code in this repository (the service, the matcher, the clinician UI, the scripts and the workflows)
and its default configuration, including the container images it builds.

**Out of scope:** a deployment's own infrastructure and settings; third-party services (the identity provider, the
database host, an HL7 sender); denial of service by volume; findings that need a compromised maintainer account or
machine; reports from automated scanners with no shown impact; missing hardening on paths that are documented as off
by default (for example the HL7 listener).

## 7. What the software already does

Checked by tests where a test is named (this is a pre-release scaffold, see
[`spec/14-implementation-status.md`](spec/14-implementation-status.md)):

- Login enforcement is **on** by default; the service refuses to start without token verification keys, and refuses
  `production` with login off (`auth::tests`).
- Health probes and `/metrics` are the only unauthenticated paths (`auth::tests`, `tests/route_census.rs`).
- Keycloak access tokens are accepted only with the right signature, issuer, audience, expiry and verified email, and
  a half-configured Keycloak stops the service starting (`tests/keycloak_guard.rs`).
- The audit table refuses `UPDATE`, `DELETE` and `TRUNCATE` (`tests/audit_immutable.rs`, needs Postgres); audit rows
  are hash-chained, keyed when `EPR_INTEGRITY_MAC_KEY` is set.
- The HL7 listener is off unless configured, loopback unless allowed, and with TLS settings speaks only TLS, can
  require a client certificate and check its name (`tests/mllp_feed.rs`).
- Security headers and a strict Content Security Policy on the UI (`tests/e2e/smoke.spec.ts`).
- The UI's backend-for-frontend proxy refuses any path outside `/api`.
- `cargo deny` with expiring advisory ignores, a secret scan of the full history, CodeQL and `pnpm audit` run in CI;
  Dependabot watches the dependencies.

## 8. What it does not claim

No third party has audited it, and no certification (ONC, DTAC, DSPT, DCB0129, HIPAA) is claimed. Roles, consent,
break-the-glass and a hash-chained audit trail are built for the routes listed in
`spec/14-implementation-status.md` and refuse a write whose audit row cannot be written; a per-subject request limit
exists (`src/rate_limit.rs`, in the process only); a clinical record, a sign-in lockout, a limit shared between
replicas and a completed hazard review are not built (`tasks.md`: SE-*, AU-*, OS-2). A deployer is responsible for
their own configuration, keys, backups and assessment.
