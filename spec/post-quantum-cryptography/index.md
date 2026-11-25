# Post-quantum cryptography

Researched 2026-11-25 from public sources and from this repository. **A proposal: nothing here is built, and no cryptographer, security officer, lawyer or data protection officer has reviewed it.** It says which rules exist and how strong they are, what cryptography this project uses, what is at risk, and an order of work. Task: SE-12. It is not legal advice, and it is not a claim that this product complies with anything.

Related: [security](../../docs/security/index.md), [security policy](../../docs/security/policy.md), [persistence](../10-persistence.md), [audit](../audit/index.md), [non-functional requirements](../07-non-functional-requirements.md), [licences](../licenses/index.md).

## The short answer

1. **The one risk that grows today is recorded traffic.** Someone may record encrypted connections now and read them when a large quantum computer exists. A health record stays sensitive for a lifetime, so for this project key exchange comes first, and signatures after.
2. **No connection that the Rust service makes or accepts can use a post-quantum key exchange today.** It builds TLS with `rustls` on the `ring` provider; `rustls` documents the hybrid group `X25519MLKEM768` for the `aws-lc-rs` provider [RUSTLS].
3. **What is stored and hashed is mostly symmetric**, which NIST and the UK's cyber security centre say can stay [IR8547] [NCSC24].
4. **No source read for this page requires post-quantum cryptography of a health record system.** The dates are guidance, or bind others. Planning by 2028 is still the sensible reading.

## What the brief said, checked

The request came with a summary. Each claim was checked against the source it rests on.

| Claim in the brief | Found | Verdict |
|---|---|---|
| The UK sets 2028 (inventories and plans) and 2035 (full transition) | Yes, and **2031** between them: the highest-priority migration done and the plan refined [NCSC25] | Right, incomplete |
| These are "strict deadlines" and "mandates" | The UK page is guidance, calls the dates "indicative", and says organisations "should work towards these key dates". It is aimed mainly at large organisations, critical national infrastructure and bespoke IT. It does not name health [NCSC25] | Overstated for the UK |
| The UK recommends ML-KEM-768 | Yes, and ML-DSA-65, "for most use cases" [NCSC24] | Right |
| SLH-DSA, LMS or XMSS for software and firmware signing | Yes, "not suitable for general purpose use"; LMS and XMSS only where state can be managed in a trusted manner [NCSC24] | Right, with a condition left out |
| AES-256 is quantum-safe | Yes. But 256 bits is not needed: the UK says symmetric algorithms with at least 128-bit keys can continue, and NIST puts approved symmetric primitives of at least 128 bits in category 1 or above [NCSC24] [IR8547] | Right, and narrower than the rule |
| Hybrids are a temporary bridge | The UK: if a hybrid is chosen, as "an interim measure" [NCSC24]. NIST allows one when at least one part is approved [IR8547] | Right |
| NIST deprecates RSA and ECC after 2030 and disallows them after 2035 | Yes. The report is **final since 27 October 2026**; the 2024 draft deprecated only the 112-bit ones. Read in NIST's own text [IR8547] | Right |
| Executive Order 14412 "mandates" it for US federal contractors by late 2030 | The order (22 June 2026) tells the procurement council to **propose a rule** within 180 days that would require covered contractors to comply by 31 December 2030. Agencies: key establishment by 31 December 2030 and **signatures by 31 December 2031**, for high value assets and high impact systems [EO14412]. Whether the rule has been proposed was not checked | Overstated; the rule is not made |
| EU: strategies by 2026, high-risk systems by 2030, all by 2035 | The Commission's page says Member States "should start" by the end of 2026 and high-risk use cases should move no later than the end of 2030; the roadmap is a set of recommendations aimed at Member States. **2035 is not on the two Commission pages read**; other sources give it for medium risk [EU] | Mostly right; not binding on a supplier |
| DORA requires cryptographic agility, and critical infrastructure must move by 2030 | Two different things. DORA's technical standard, for **financial entities**, requires provisions for changing cryptography as cryptanalysis develops (Article 6(4)), and a recital names quantum threats. It names no post-quantum algorithm and no date [DORA]. The 2030 date is the roadmap's | Merged; corrected |
| PCI DSS 4.0 requires inventories and "quantum readiness plans" | Requirement 12.3.3 asks for an inventory of cipher suites and protocols, a review at least every 12 months and a plan for changes. Two secondary sources agree that it does not name quantum; the standard itself was not read [PCI]. This product holds no cardholder data | Half right; does not apply today |
| "Regulators expect" a DPIA that addresses quantum risk, vendor attestations and pilot results | No source was found that says so | Not supported; sensible as practice |
| "A point of consensus on Reddit" about vendor contracts | Not checked, and not evidence. The UK guidance says to tell suppliers what you need by 2028 [NCSC25] | Replaced by a source |

## Rules, and what they mean for a deployment

| Place | Who it binds | Dates | For this product |
|---|---|---|---|
| United Kingdom, NCSC [NCSC25] | Nobody by law; guidance to large organisations, critical infrastructure and those with bespoke software | 2028, 2031, 2035 | A deploying hospital should expect to plan on it. This product is "custom or specialised software", for which an update or replacement has to be found on the same timetable |
| United States, EO 14412 and IR 8547 [EO14412] [IR8547] | Federal agencies; covered contractors if the rule is made | 2030, 2031, 2035; TLS 1.3 for agencies by 2 January 2030 | Only for a federal deployment |
| European Union, roadmap [EU] | Member States, as recommendations | end 2026, end 2030, (2035) | A country's own plan decides. "High-risk use cases" are not defined on the pages read |
| European Union, DORA [DORA] | Financial entities | In force; no post-quantum date | Only if a customer is one |
| Health law (HIPAA and others) | Those it covers | None found | A 2026 review says HIPAA does not prescribe post-quantum algorithms and that covered entities should document quantum risk where records live long [FRONT] (read in part, through a summary) |

**A deploying organisation in another country has its own rules. None was looked for beyond these.**

## Standards

| Need | Standard | Notes |
|---|---|---|
| Key exchange | ML-KEM, FIPS 203 (13 August 2024); ML-KEM-768 as default [NCSC24] [IR8547] | Public key 1,184 bytes, ciphertext 1,088. HQC is selected by NIST as an alternative and not yet published |
| Key exchange in TLS 1.3 | Hybrid `X25519MLKEM768`, RFC 10024 (August 2026, Proposed Standard, the one marked Recommended) [RFC10024] | TLS 1.2 has none |
| Signatures | ML-DSA, FIPS 204; ML-DSA-65 as default [NCSC24] | Public key 1,952 bytes, signature 3,309, against 32 and 64 for Ed25519 [RFC9964] |
| Signatures in tokens | RFC 9964 (May 2026, Proposed Standard): `ML-DSA-44`, `-65`, `-87`, key type `AKP` [RFC9964] | Browsers do not verify ML-DSA in web certificates yet [CF] |
| Firmware and software signing | SLH-DSA (FIPS 205); LMS, XMSS (SP 800-208) | State must be managed for LMS and XMSS |
| Symmetric, hash, MAC | AES with 128 bits or more, SHA-256 and up, HMAC | No change needed |

## What this project uses

Read from the code on 2026-11-25. "At risk" is risk from a quantum computer only.

| Use | Where | Algorithm | At risk | Harm if broken |
|---|---|---|---|---|
| HL7 feed, TLS listener | `mllp.rs` (`EPR_MLLP_TLS_*`) | `rustls` 0.23, **ring**, TLS 1.2 and 1.3; certificates as configured | **Key exchange, now** | Recorded messages are read later: names, numbers, admissions, deaths |
| Service to PostgreSQL | `sqlx`, `tls-rustls-ring-webpki`; production must set `sslmode` to `require`, `verify-ca` or `verify-full` | `rustls`, **ring** | **Key exchange, now** | The whole record in transit |
| Browser to UI, UI to service | The deployment's proxy; Node 26 in the UI image | The deployment's | **Key exchange, now**, unless the proxy offers the hybrid | Everything a screen shows |
| Keycloak and key-set fetches | `authentication-verifier` (`fetch`), UI OIDC | TLS of the library | Key exchange | Keys are public; tokens in transit are short-lived |
| Sign-in tokens, PASETO | `auth.rs`, `v4.public` | Ed25519 | Later | Forged sign-in, once a machine exists. Not retrospective |
| Sign-in tokens, Keycloak | `authentication-verifier` on `jsonwebtoken` 9 | RSA or elliptic curve, as the realm signs | Later | The same |
| HL7 client certificates | `mllp.rs` | As the hospital's authority issues | Later | A false sender |
| Integration tokens, link tokens | `api_tokens_store.rs`, `link_token.rs` | Random, stored as SHA-256 | No | |
| Audit chain | `audit.rs`, `integrity-mac` | HMAC-SHA-256 with HKDF, or SHA-256 unkeyed | No | Each value names its scheme and key, and an unknown scheme is said to be unverifiable and not wrong |
| Sessions of the UI | `session-crypto.ts` | AES-256-GCM; SHA-256 ids | No | |
| OIDC sign-in | `oidc.ts` | PKCE with SHA-256 | No | |
| Retried-write fingerprints, photos, tokens | several | SHA-256 | No | |
| Commit signatures | git, SSH | Ed25519 | Later | A forged commit |
| Release provenance | `release.yml`, GitHub attestations | **Not established** | Later, probably | A forged build |
| Package registries, container images | crates.io, npm, quay.io, docker.io | Theirs | Theirs | |
| Data at rest, backups | The deployment | The deployment's | If keys are wrapped with RSA or elliptic curves, **now** | The whole record |

**Not looked at:** the mobile apps (a branch), what the base images' OpenSSL offers, the languages' package managers, and any hardware.

**What the parts can do today.** PostgreSQL 18 takes `ssl_groups`, default `X25519:prime256v1`, so the hybrid has to be named [PG]; mailing-list posts report it working with OpenSSL 3.5, and whether the `postgres:18-alpine` image has that was not checked. `sqlx` 0.9 has `tls-rustls-aws-lc-rs` [SQLX]. Current Chrome, Edge, Firefox and Safari offer the hybrid by default, Safari from version 26, and Node from 24.5 [CF]. PASETO has versions 1 to 4; a post-quantum one is named only as a possibility [PASETO]. Keycloak merged an ML-DSA key provider on 16 November 2026, so in no release that this project pins (26.3.0); whether any release has it was not established [KC].

## What lives longest

A common rule of thumb, in the author's words: if the years that data must stay secret, and the years a migration takes, together exceed the years until a machine exists, it is already late. The last number is not known, and this page does not guess it.

| Data | Must stay secret for | Note |
|---|---|---|
| The record, above all HIV, mental health, violence, custody, genetics | A lifetime; genetics longer, for relatives | Labels limit who reads it here, and do nothing for recorded traffic |
| Identity (names, numbers, addresses) | A lifetime, and longer for someone hiding from a person | |
| Audit trail | As long as it is kept; no period is set | Integrity rests on HMAC, which is not at risk |
| Tokens and sessions | Minutes to hours | Forging matters; reading an old one does not |
| Releases | While anyone runs them | |

## Rules proposed for this project

1. **No cryptography of our own**, as the security policy says and the UK guidance repeats [NCSC25]. Only a maintained library, in a protocol that has a standard.
2. **Key exchange before signatures.** A recorded connection is lost for good; a signature fails only from the day a machine exists.
3. **Hybrid, and then ask again.** `X25519MLKEM768` in TLS 1.3, preferred and not only offered. Not pure ML-KEM: a fault found in a young algorithm or its code would then be the whole protection.
4. **Every signed or keyed value names its algorithm and key**, and an unknown one is refused or said to be unverifiable, never passed. The audit chain does this; tokens do by their own headers.
5. **A fallback is seen.** If a connection is made without the hybrid, the service can say how many. An offer that a middlebox removes quietly is the likeliest failure, and the UK guidance says to check what is really used [NCSC25].
6. **Availability is safety.** Nothing here may refuse a hospital's connection because it is old before that hospital has agreed a date. A larger first message can meet old equipment that fails on it. Any refusal is a setting, off by default, with a hazard row.
7. **An inventory that cannot go stale.** A test that fails when a cryptographic dependency is added that the inventory does not name.
8. **No claim.** Until it is measured and reviewed, no document says "quantum-safe", "post-quantum compliant" or "quantum-ready".

## Clinical risk

No hazard row changes with this page: nothing is built. Candidates for the Clinical Safety Officer when it is:

- **A connection fails after the change**, and a feed, a screen or the database stops: care is delayed. Cause: a larger first message, an old TLS library, a certificate too big.
- **A sign-in token grows past a limit.** An ML-DSA-65 signature is 4,412 characters in base64url, against 86 for Ed25519. A proxy with an 8 KiB header limit may refuse it and lock every clinician out.
- **Recorded traffic is read years later**, and a person is harmed by what it says.
- **False reassurance**: the hybrid is thought to be in use and is not.

## Order of work, proposed

Each is small unless marked. None is started, and **PQ-3 is not to be started yet** (the maintainer, 2026-11-25, OQ-PQ2). SE-12 is the parent and keeps its acceptance.

| Step | What | Acceptance | By |
|---|---|---|---|
| PQ-1 | The inventory above as a file, and a test over the lock files | The test fails on a planted dependency | 2026 |
| PQ-2 | Measure first: what each connection negotiates today (feed, database, proxy, Keycloak) | A table in this page, from a run | 2026 |
| PQ-3 (M) | `aws-lc-rs` in place of `ring`, with the hybrid preferred, for the feed and the database | A test that a client offering the hybrid gets it, and that one that does not still connects. The licence, build time, image size and the targets it builds for are recorded | 2027 |
| PQ-4 | The database and the proxy: `ssl_groups`, and an example for the proxy | `compose.yaml` negotiates it, shown by a check | 2027 |
| PQ-5 | A count of connections made without the hybrid, and TLS 1.2 ones | A metric, and a row in [measures](../clinical-safety/measures.md) | 2027 |
| PQ-6 | The audit chain verifies across a change of scheme (SE-12's own acceptance) | A test | 2027 |
| PQ-7 | Token size: a sign-in with an ML-DSA-65 sized token, through the proxy, the UI and the service | A test that passes, or the limit that fails, named | 2027 |
| PQ-8 (M) | Keycloak ML-DSA, when a release has it and the verifier can read it | A sign-in under it in CI, beside the old | after a release |
| PQ-9 | A migration plan for a deploying hospital: what is theirs, what is ours, what to ask suppliers | Reviewed by a security officer | 2028 |
| PQ-10 | Commit and release signing | Follows the forge and the registries | watch |

`aws-lc-rs` is C and assembly, needs more to build than `ring`, and has its own licence terms: the licence gate decides first. `sqlx` itself says its build requirements are "more onerous" [SQLX].

**A cryptography bill of materials.** Releases already carry CycloneDX component lists (`release.yml`). CycloneDX has a form for cryptographic assets: algorithms, keys, certificates and what uses them [CBOM]. Proposed: make it from PQ-1's file and not from a scanner, since a scanner sees what is linked and not what is used.

## What a deploying organisation has to do

The product cannot do these: certificates and their authority; the proxy; disks, backups and how their keys are wrapped; Keycloak; the network devices between; the senders of HL7 messages; other suppliers; the risk register, the DPIA and the date.

## Open questions

Summarised as OQ-PQ1 to OQ-PQ5 in [open questions](../16-open-questions.md).

1. Which country's dates does the first deployment work to, and does its contract say so? (OQ-PQ1)
2. May `aws-lc-rs` replace `ring`: the licence, the build, and the targets of a small site? **Answered 2026-11-25 by the maintainer: not yet.** No step is started without their word. (OQ-PQ2)
3. Is the hybrid required or only preferred, and from when may a connection without it be refused? Recommendation: preferred, and refused only where a deployment says so. (OQ-PQ3)
4. Does PASETO stay, with no post-quantum version, or do tokens move to a standard that has one? Recommendation: wait; nothing is lost by waiting for signatures. (OQ-PQ4)
5. Who reviews this: is there a cryptographer, and who is the security officer? (OQ-PQ5)

## Not done, not verified

- No code, no measurement, no test. What any connection negotiates was read from configuration and documentation and not observed.
- The EU roadmap's own PDF, the PCI standard, OMB M-26-15, the FIPS texts, the DORA text in the Official Journal and the GitHub attestation algorithm were not read. The review [FRONT] was read in part. The NIST, UK, US order, RFC, PostgreSQL, `rustls` and Keycloak points were read at their source.
- Whether the US procurement rule has been proposed, and any UK health-specific or other country's rule, was not established. Absence of a finding is not absence of a rule.
- These are the author's reasoning and have no source: that key exchange should come first **for this product**; hybrid and not pure ML-KEM; that old equipment may fail on a larger first message; the header limit and large tokens; how long each kind of data must stay secret; and the "By" years.

## Sources

Read 2026-11-25.

- [NCSC25] UK National Cyber Security Centre, Timelines for migration to post-quantum cryptography, v1.0, 20 March 2025 (file of August 2026): https://www.ncsc.gov.uk/guidance/pqc-migration-timelines
- [NCSC24] UK National Cyber Security Centre, Next steps in preparing for post-quantum cryptography, v2.0, 14 August 2024: https://www.ncsc.gov.uk/whitepaper/next-steps-preparing-for-post-quantum-cryptography
- [IR8547] NIST IR 8547, Transition to Post-Quantum Cryptography Standards, final, October 2026: https://nvlpubs.nist.gov/nistpubs/ir/2026/NIST.IR.8547.pdf
- [EO14412] Executive Order 14412, 22 June 2026: https://www.govinfo.gov/content/pkg/DCPD-202600417/html/DCPD-202600417.htm
- [EU] European Commission, Post-quantum cryptography, and the coordinated implementation roadmap: https://digital-strategy.ec.europa.eu/en/policies/post-quantum-cryptography and https://digital-strategy.ec.europa.eu/en/library/coordinated-implementation-roadmap-transition-post-quantum-cryptography
- [DORA] Delegated Regulation (EU) 2024/1774, Article 6 and recital 9, in an unofficial copy: https://www.springlex.eu/en/packages/dora/rts-rmf-regulation/article-6/
- [PCI] Two secondary accounts of PCI DSS 12.3.3: https://postquantum.com/security-pqc/pci-dss-v4-quantum-cryptographic-requirements/ and https://www.halock.com/are-you-keeping-an-inventory-of-cipher-suites-and-certificates-for-the-new-pci-dss
- [FRONT] Wu and others, a review in Frontiers in Health Services, 16 July 2026: https://www.frontiersin.org/journals/health-services/articles/10.3389/frhs.2026.1901282/full
- [RFC10024] https://www.rfc-editor.org/info/rfc10024
- [RFC9964] https://www.rfc-editor.org/info/rfc9964
- [RUSTLS] The `rustls` manual, defaults, 0.23.46: https://docs.rs/rustls/latest/rustls/manual/_05_defaults/index.html
- [SQLX] https://docs.rs/sqlx/latest/sqlx/
- [PG] https://www.postgresql.org/docs/18/runtime-config-connection.html
- [PASETO] https://github.com/paseto-standard/paseto-spec/tree/master/docs/01-Protocol-Versions
- [KC] Keycloak pull requests 55099 and 55395 and issue 52814: https://github.com/keycloak/keycloak/issues?q=ML-DSA
- [CF] Cloudflare, post-quantum support, 24 June 2026: https://developers.cloudflare.com/ssl/post-quantum-cryptography/pqc-support/
- [CBOM] https://cyclonedx.org/capabilities/cbom/
