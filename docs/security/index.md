# Security: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

## Definition

ISO/IEC 25010:2023 defines security as the degree to which a product protects information and data so that persons or systems have the degree of data access appropriate to their types and levels of authorisation. Per secondary summaries (Sonar, arc42; search snippets and one page, the ISO text was not read) the sub-characteristics are confidentiality, integrity, non-repudiation, accountability, authenticity and resistance ([Sonar](https://www.sonarsource.com/resources/library/iso-iec-25010-explained/), [arc42](https://quality.arc42.org/standards/iso-25010)). Resistance (withstanding, responding to and recovering from attacks) is described as new in 2023; the 2011 list had the first five. I could not confirm the exact 2023 wording of each definition.

Cross-checks used here: OWASP ASVS for verification requirements ([OWASP ASVS](https://owasp.org/www-project-application-security-verification-standard/)), NIST SP 800-53 for controls ([NIST](https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final)), NIST SP 800-63 for identity assurance ([NIST](https://pages.nist.gov/800-63-4/)), the NCSC Cyber Assessment Framework ([NCSC](https://www.ncsc.gov.uk/collection/cyber-assessment-framework)), the DSPT ([England's national health body](https://www.dsptoolkit.nhs.uk/)) and DTAC ([England's national health body](https://transform.england.nhs.uk/key-tools-and-info/digital-technology-assessment-criteria-dtac/)). Bass, Clements and Kazman's security scenarios ask who attacks, with what, what is protected, and how fast it is detected and recovered.

## Why it matters for an EPR

- Health data is both sensitive and persistent: it cannot be changed after a leak the way a password can.
- The 2022 Advanced ransomware attack disrupted England's 111 urgent care phone service and other services; the ICO found, among other things, a customer account without multi-factor authentication and fined the processor about 3.07 million pounds ([Computing](https://computing.co.uk/news/2025/security/ico-fines-advanced), snippet only).
- The 2017 WannaCry attack hit the UK's public health service through unpatched systems ([NAO](https://www.nao.org.uk/reports/investigation-wannacry-cyber-attack-and-the-nhs/), from memory).
- US breaches of unsecured protected health information of 500 or more people are listed publicly by the HHS Office for Civil Rights ([HHS breach portal](https://ocrportal.hhs.gov/ocr/breach/breach_report.jsf)); I did not count entries.
- The HIPAA Security Rule NPRM of January 2025 proposed mandatory encryption, MFA, asset inventory and segmentation ([Federal Register](https://www.federalregister.gov/documents/2025/01/06/2024-30983/hipaa-security-rule-to-strengthen-the-cyber-security-of-electronic-protected-health-information), proposed, not final).
- Insider misuse (snooping on records) is a standing threat, which is why access control and audit are one story ([auditability](../auditability/index.md)).

## What the project says today

- NFR-6 (`spec/07-non-functional-requirements.md`): login enforced by default, TLS 1.2 or higher, AES-256 at rest, MFA, per-tenant keys, no secrets in code, non-root containers, cargo-deny, SBOM, ZAP.
- Built (`SECURITY.md`): login on by default; refusal to start without keys or with `production` and login off (`auth::tests`); only health probes and `/metrics` unauthenticated (`tests/route_census.rs`); Keycloak token checks of signature, issuer, audience, expiry, verified email (`tests/keycloak_guard.rs`); PASETO v4 tokens; strict CSP and security headers on the UI (`tests/e2e/smoke.spec.ts`); the UI proxy refuses paths outside `/api`; `cargo deny`, Dependabot, secret scanning (OS-11) and expiring advisory ignores (OS-12) in CI.
- Authorisation (SE-1, `spec/access-control/index.md`): a policy table with a route census; roles plus care relationship, purpose of use and consent; `404` rather than `403` where existence would leak; break-glass with a 4-hour grant, a reason and auditor review (SE-3, no real-time notification). Multi-tenancy (SE-5), sensitive masking (SE-4), and per-tenant keys are not built.
- Rate limits (OS-2, NFR-6): per token subject, per process, counted by `axa-ratelimit`, which holds a subject only as a keyed hash and lets a new subject through when its table is full; not per address; no sign-in lockout; not shared between replicas.
- MFA: the service reads no `acr` or `amr` claim and cannot demand a second factor; the development realm requires none; no test signs in with one (`governance/mfa-plan.md`).
- Known weak points: HL7 MLLP has no authentication or encryption (off unless configured); `/metrics` is public when on; TLS and disk encryption are the deployer's job (`governance/dpia.md`, `governance/data-flow.md`).
- Integrity: audit table triggers refuse UPDATE, DELETE and TRUNCATE; hash chain with keyed MAC (`EPR_INTEGRITY_MAC_KEY`); no SQL delete path for clinical data; a seeded chaos test sends hostile input to every route.
- Not done: penetration test, third-party audit, any certification. A reporting route and response targets (5 and 15 business days) exist in `SECURITY.md`; one maintainer.

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
| --- | --- | --- | --- |
| ASVS coverage | Count of ASVS requirements by level, marked met, not met or not applicable with evidence | Pick a level (L2 is a common choice for sensitive data, my suggestion); publish the table | None yet; idea 1 |
| Route census completeness | Every route has a policy entry and a test | 100 per cent, enforced | `tests/route_census.rs` (built) |
| Known vulnerable dependencies | `cargo deny`, advisory age | Zero unignored; ignores expire | CI `deny` job (OS-12) |
| Secrets in repo | Secret scan findings | Zero | CI `secrets` job (OS-11) |
| Staff accounts with a second factor | Share of active staff with MFA enrolled | 100 per cent, deployer-measured | Not collectable here; Keycloak |
| Token acceptance faults | Count of refused tokens by reason | Alert on spikes | `/metrics` (partly) |
| Time to patch critical advisory | Advisory date to release | Set by policy; SECURITY.md has acknowledgement targets only | Changelog, Dependabot |
| DAST findings | ZAP baseline results | No high findings | CI `zap` job |
| Penetration test findings | Independent test before real use | Open critical findings zero at go-live | Not done |
| Break-glass reviews overdue | Grants past review limit | Zero | EC-11 list (partly built) |

## Trade-offs with other attributes

- Security versus interaction capability: stronger sign-in, short sessions and lockouts slow a clinician; a ward shared terminal needs fast switching (SE-10). See [interaction-capability](../interaction-capability/index.md).
- Security versus reliability: limits can refuse care (H-039); see [reliability](../reliability/index.md).
- Security versus interoperability: plaintext MLLP is common in hospital feeds; secure options cost partner effort ([interoperability](../interoperability/index.md)).
- Security versus privacy: logging more for detection collects more personal data ([privacy](../privacy/index.md)). Versus safety: a refusal in an emergency is a hazard ([safety](../safety/index.md)).

## Gaps and risks

- MFA cannot be enforced or checked by the service; an operator can switch it off unseen.
- No threat model is in the repository (not found by grep of the files read).
- No independent assessment; ASVS, DSPT and DTAC are not mapped.
- At-rest encryption and TLS are left to the deployer without a tested reference deployment.
- Per-process limits mean N replicas allow N times the limit; a front-end server hides client addresses.
- Post-quantum and key rotation are only planned (SE-12; rotation untested end to end).
- Sensitive-label masking (SE-4) and tenant isolation (SE-5) are not built, so a multi-tenant deployment is not safe yet.

## Ideas

1. Write `docs/security` ASVS L2 mapping as a table with evidence links (extends OS-4, AS-11).
2. Read `acr` and `amr` and refuse tokens lacking a second factor for staff roles, behind a setting (PR-7 follow-up; SE-10).
3. Write a short STRIDE threat model per data-flow row in `governance/data-flow.md`.
4. Put `/metrics` behind a token or a separate port by default (SE-7).
5. Provide a reference Podman deployment with TLS, a non-default realm with WebAuthn, and a script that checks it.
6. Add a shared rate-limit store and per-address limits behind the UI's trusted proxy header (OS-2).
7. Commission a penetration test before any real data; record scope and findings openly.
8. Map controls to DSPT assertions and the DTAC security section (AS-2, AS-11) without claiming compliance.

## Open questions

- Which ASVS level and which NIST 800-63 assurance level apply for staff, patients and proxies (mfa-plan item 1)?
- Who runs the identity provider in a small deployment, and does the equity rule allow a simpler authenticator where hardware keys are unaffordable?
- Should integration accounts use mutual TLS or short-lived tokens?

## Not verified

- ISO 25010:2023 definitions come from secondary summaries; wording not confirmed.
- Every URL here was either shown in a search result or cited from memory; apart from the one search snippet on the ICO fine, none of the pages was read in this session.
- No scan, penetration test or token test was run by the author; test names are as the repository states them.

## Sources

- [Sonar on ISO/IEC 25010](https://www.sonarsource.com/resources/library/iso-iec-25010-explained/) and [arc42](https://quality.arc42.org/standards/iso-25010): sub-characteristics, secondary.
- [OWASP ASVS](https://owasp.org/www-project-application-security-verification-standard/): verification standard.
- [NIST SP 800-53 Rev 5](https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final): control catalogue.
- [NIST SP 800-63-4](https://pages.nist.gov/800-63-4/): digital identity guidelines.
- [NCSC CAF](https://www.ncsc.gov.uk/collection/cyber-assessment-framework): UK assessment framework.
- [DSPT](https://www.dsptoolkit.nhs.uk/) and [DTAC](https://transform.england.nhs.uk/key-tools-and-info/digital-technology-assessment-criteria-dtac/): requirements of England's national health body.
- [Computing, ICO fines Advanced](https://computing.co.uk/news/2025/security/ico-fines-advanced): 2022 ransomware fine.
- [NAO, WannaCry](https://www.nao.org.uk/reports/investigation-wannacry-cyber-attack-and-the-nhs/): 2017 attack.
- [HHS OCR breach portal](https://ocrportal.hhs.gov/ocr/breach/breach_report.jsf): public breach list.
- [Federal Register, HIPAA Security Rule NPRM](https://www.federalregister.gov/documents/2025/01/06/2024-30983/hipaa-security-rule-to-strengthen-the-cyber-security-of-electronic-protected-health-information): proposed rule.
