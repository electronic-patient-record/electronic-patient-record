# Security policy

The rules this project holds itself to while it builds the software, and what enforces each one. How to report a
vulnerability, and what happens to a report, is the [vulnerability disclosure policy](../../SECURITY.md). What security
means for an EPR, and how it is measured, is [security: what it means here](index.md).

| | |
|---|---|
| Policy version | 1.0, 2026-11-17 |
| Owner | The maintainer ([MAINTAINERS.md](../../MAINTAINERS.md)) |
| Approved by | The maintainer, 2026-11-17 |
| Next review | 2027-05-17, and after any security incident or any change of maintainer |

**This software is not for clinical use.** This policy covers making the software. It is not an information security
management system for a deployment: whoever runs the software needs their own (section 10).

## 1. Purpose and scope

The purpose is that every change to this repository keeps patient data confidential, the record and its audit trail
intact, and the service available, and that a weakness found later is fixed in a known time.

In scope: the service, the matcher, the clinician UI, their tests, scripts, workflows, container images and
documentation in this repository, and the accounts, keys and machines used to change them.

## 2. Roles

| Role | Who | Responsible for |
|---|---|---|
| Security owner | The maintainer | This policy, its exceptions, security reports, releases, and the keys that sign them |
| Clinical Safety Officer | Not appointed (OQ-2) | Hazards a vulnerability raises; only the CSO accepts or closes a hazard |
| Contributor (person or AI agent) | Anyone making a change | Following sections 3 to 8; saying plainly what was not checked |

With one maintainer there is no separation of duties: the person who writes a change also approves it. CI's gates
and the census tests are the second check (section 4), and a deployer must not rely on review depth this project
cannot give ([MAINTAINERS.md](../../MAINTAINERS.md)).

## 3. Principles

1. **Secure by default.** A feature that is off is safe; turning one on is explicit, and a setting that is present but
   wrong stops the service starting rather than running without it.
2. **Deny by default.** A route, a role and a log field are refused unless listed. A census test fails when something
   unlisted appears.
3. **Fail closed.** A write whose audit row cannot be written is refused; a token that cannot be verified is refused.
4. **Least data.** No personal data in logs, metrics, error text or test fixtures; real personal data never enters the
   repository.
5. **Never weaken a guard to get green.** A failing security check is fixed at its cause, or carried as a recorded,
   expiring exception (section 9).
6. **Say what is not done.** Every change records what was not built or not verified (`CHANGELOG.md`).

## 4. Secure development

| Rule | Enforced by |
|---|---|
| Every route has a policy entry and an authentication decision | `tests/route_census.rs`, `tests/openapi_responses_roles.rs` |
| Login is on by default; `production` with login off refuses to start | `auth::tests` |
| The clinical record and the audit trail are append-only | Database triggers; `tests/audit_immutable.rs` and the append-only census tests |
| No `unsafe` Rust in the service | `#![forbid(unsafe_code)]` in `src/lib.rs` (the matcher does not forbid it yet) |
| No message value from an HL7 feed in a log line, an audit row or a metric | The marker tests in `tests/mllp_feed.rs` (IH-4) |
| The UI sends a strict Content Security Policy and security headers | `tests/unit/security-headers.test.ts`, `tests/e2e/smoke.spec.ts` |
| Code is formatted, linted with warnings as errors, documented and tested | The gates of `scripts/ci-local.sh` and CI ([`spec/local-ci.md`](../../spec/local-ci.md)) |
| Static analysis | CodeQL in CI |
| A change that could alter what a clinician sees or signs updates the hazard log | CI's `hazard-log` job on a `clinical-impact` pull request; `AGENTS.md` |

A security-relevant change (authentication, authorisation, consent, audit, cryptography, input parsing, a new
unauthenticated path) needs a test that fails without it. Where it matters most, check that test by breaking the code
on purpose and seeing it fail (a mutation check), and say so in the changelog.

## 5. Dependencies and supply chain

| Rule | Enforced by |
|---|---|
| No dependency with a known vulnerability, a banned licence or an unknown source | `cargo deny` (advisories, bans, licences, sources) and `pnpm audit` in CI |
| An advisory may be carried only with a reason and an expiry at most about six months ahead | `deny.toml` comments; `tests/advisory_expiry.rs` |
| A new release is not taken until it is two days old (the cooldown); an exception is approved by the maintainer and recorded | The maintainer's cargo and pnpm settings; `minimumReleaseAgeExclude` in `pnpm-workspace.yaml`; the exception's `CHANGELOG.md` entry |
| Lockfiles are committed and builds use them | `--locked` in every Rust gate; `pnpm install --frozen-lockfile` in CI |
| Each crate builds on the oldest Rust it claims | `rust-msrv` gate (QA-8) |
| Releases carry build provenance and CycloneDX SBOMs | `.github/workflows/release.yml` |
| Dependabot proposes updates | `.github/dependabot.yml` |

Prefer a dependency already in the tree over a new one; a new direct dependency is named, with why, in the commit
that adds it.

## 6. Secrets and keys

- No secret, key or token is committed. The full history is scanned in CI (`secrets` job, gitleaks); an accepted
  finding is listed with its reason in `.gitleaksignore`. Test certificates and keys are made when a test runs
  (`tests/mllp_feed.rs`).
- Secrets reach the service only through its environment or files the deployer provides; none has a usable default.
- Commits on `main` are signed by the maintainer's key. A lost or exposed key is an incident (section 8).
- Integration tokens (TL-1, `api_tokens_store.rs`): a token is shown once, when it is issued, and never stored or
  returned again; only the SHA-256 hash of its 244-bit secret is kept (`axa::api_tokens`), so a copy of the table lets
  no one in, and it is compared in constant time. A token reads `epr_<id>_<secret>`, so a secret scanner can find a
  leaked one. Each is named (never reused), holds only scopes the integrator role already has, and expires within 365
  days; revocation is final. Issuing, changing and revoking are the clinical administrator's and are audited without
  the token. Signing in with one is not built yet.

## 7. Cryptography

- TLS through rustls only, with ring's provider chosen explicitly; TLS 1.2 or 1.3. No hand-written cryptography.
- Audit rows are hash-chained, keyed by `EPR_INTEGRITY_MAC_KEY` (HMAC-SHA-256) when set; identifiers that must not be
  readable (an HL7 control id) are kept only as a keyed digest under a subkey.
- Tokens are verified, never only decoded: signature, issuer, audience and expiry (`tests/keycloak_guard.rs`).
- Encryption at rest and key storage belong to the deployment (section 10). Key rotation and post-quantum readiness
  are planned only (SE-12).

## 8. Vulnerabilities and incidents

A reported or found vulnerability follows the [disclosure policy](../../SECURITY.md): acknowledge, triage, rate with
CVSS v4.0 raised for clinical harm, fix in the target time, disclose with an advisory.

An incident for this project is any of: a maintainer account, key or machine compromised; a secret committed; a
malicious or hijacked dependency in the tree; a release that is not what its tag says. Then the security owner:

1. contains it: revokes the key or token, removes the access, pins or removes the dependency;
2. checks what it reached: commits and releases since the start, by signature and provenance;
3. fixes and releases, and says in an advisory what happened, what was affected and what a deployer should do;
4. records it in the changelog, and reviews this policy within a month.

## 9. Exceptions

An exception to this policy is allowed only when the maintainer approves it, it has a reason and an end date, and it
is recorded where the check that it silences can see it: an advisory ignore in `deny.toml` (with `expires`), a
cooldown exception in `CHANGELOG.md` and `tasks.md`, an accepted secret-scan finding in `.gitleaksignore`. An exception
without an end date is not an exception; it is a change to this policy, and is made here.

## 10. What a deployer must do

This project cannot do these for a deployment, and nothing here is evidence that they were done: a risk assessment
and DPIA of their own ([`governance/dpia.md`](../../governance/dpia.md) is a starting point); TLS in front of the
service and the UI; encryption at rest and backups that are restored in tests; an identity provider with multi-factor
authentication for staff ([`governance/mfa-plan.md`](../../governance/mfa-plan.md)); monitoring of the audit trail,
the feed and `/metrics`; their own penetration test before any real data; their own incident response.

## 11. Not done

- No independent audit or penetration test; no certification claimed.
- No threat model per data flow yet (QA-10) and no ASVS mapping (QA-9).
- No DAST (ZAP) job in CI, although NFR-6 asks for one.
- GitHub's secret scanning and Dependabot security updates are switched off for the repository; the secret scan in CI
  is gitleaks. (Private vulnerability reporting was switched on 2026-11-17; `SECURITY.md` gives it beside email.)
- No one is the Clinical Safety Officer (OQ-2), so a vulnerability's hazard has no one to accept it.
- Nothing checks this policy's review date.
