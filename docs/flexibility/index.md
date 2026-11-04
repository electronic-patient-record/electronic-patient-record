# Flexibility: what it means here

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading: targets here are not commitments, and nothing was measured by the author of this note unless it says so.

## Definition

In ISO/IEC 25010:2023 the 2011 characteristic "portability" became "flexibility". Its sub-characteristics, as I understand them from summaries, are adaptability, scalability, installability and replaceability; scalability is the addition ([arc42 summary](https://quality.arc42.org/articles/iso-25010-update-2023)). A search summary also says the model's target widened to ICT products generally ([search snippet via ANSI preview list](https://webstore.ansi.org/preview-pages/ISO/preview_ISO+IEC+25010-2023.pdf)). I did not read the standard; the definitions below are the usual reading, not quotations.

- Adaptability: the product can be adapted to different or evolving hardware, software or operating environments.
- Scalability: it can handle growing or shrinking load or capacity.
- Installability: it can be installed, and uninstalled, in a specified environment.
- Replaceability: it can replace another product for the same purpose, and be replaced.

Compatibility ([compatibility](../compatibility/index.md)) and interoperability ([interoperability](../interoperability/index.md)) are neighbours and are not repeated here.

## Why it matters for an EPR

- Setting differs. Community and mental-health care need a caseload, home visits and legal-status records, where acute care has beds and wards; trusts have chosen mental-health or community systems partly for mobile access and statutory returns. The evidence is trade press and supplier case studies, not a comparison study (see [care settings](../../spec/care-settings.md) for its sources).
- Configuration is a quality risk, not only a benefit. A multi-site study found large variation in physicians' performance on common order-entry tasks, and its authors noted that vendor and health-system customisation means a deployed product may differ much from the one tested ([Ratwani et al. 2018, via AMA news](https://ama-assn.org/study-finds-need-performance-standards-ehr-usability-safety); snippet only).
- Customisation also drives upgrade cost: consultancy and trade sources say heavy local build makes upgrades unpredictable ([CSI](https://www.csicompanies.com/insights/the-hidden-cost-of-custom-builds-an-epic-ehr-optimization-guide-for-health-system-it-leaders); a consultancy with a commercial interest).
- Deployers are legally distinct from manufacturers in England: DCB0129 binds the manufacturer and DCB0160 the deploying organisation, so configuration choices carry the deployer's clinical risk ([NHS Standards](https://standards.nhs.uk/published-standards/clinical-risk-management-its-application-in-the-deployment-and-use-of-health-it-systems), snippet).

## What the project says today

- Equity rule ([scope](../../spec/02-scope.md)): a neutral core, with each country's rules in a pack that a deployment selects; today weighted to the UK and US (IO-19).
- Pack mechanism ([interoperability](../../spec/interoperability/index.md), OS-10): a design only. A pack is data, never code; one directory per pack with `pack.toml`; load order is neutral core, enabled packs, a read-only override directory, then environment; a pack is supported only when its validator and test vectors pass. Proposed, unconfirmed, not built.
- Setting profiles and specialty packs ([care settings](../../spec/care-settings.md), XP-5 to XP-7, XP-10, XP-11): a setting is configuration, not a fork. Nothing built.
- Multi-tenancy: SE-5 (`tenant_id` on every table, row-level security, per-tenant keys) is open; NFR-6 lists per-tenant keys as a target.
- Scalability (NFR-3): stateless app tier is true of the UI only with `EPR_SESSION_REDIS_URL` set (OS-8, built, not run behind two replicas). The request limit is per process, so N replicas allow N times the limit (OS-2). No load run exists (NFR-2 has first numbers only, [BENCHMARKS.md](../../BENCHMARKS.md)).
- Installability: Podman only, `Containerfile` and `compose` files, non-root containers (NFR-6); a pinned toolchain (NFR-11). Whether another container runtime works is untested.
- Replaceability: data is exportable through FHIR R4; bulk export is not built (SE-2 acceptance names it). The UI is a backend-for-frontend over the same API.
- Ten locales are listed (NFR-5); translations are machine-written until a speaker reviews them. None is claimed reviewed.

## Measures

| Measure | How to measure | Suggested target or rule for setting one | Where in our data/CI |
|---|---|---|---|
| Time to a running instance | Stopwatch from clone to first authenticated request, on a clean machine | Record a first reading, then set a budget | Not collected |
| Pack onboarding effort | Files and reviewer hours to add one country pack | Record per pack; no code change should be needed | OS-10, IO-18 (not built) |
| Pack evidence parity | Share of packs with validator pass, test vectors and a named local reviewer | Same evidence for every pack (equity rule); show unverified packs as such | Pack manifest `reviewed` field (design) |
| Configuration without code | Count of setting differences that needed a code change | Target zero for setting profiles | XP-5 list (not done) |
| Horizontal scale | Throughput and p95 at 1, 2, 4 replicas behind a load balancer | Near-linear to a stated ceiling; state where it stops | Not run (OS-5) |
| Tenant isolation | DB test: cross-tenant read with a forged id returns nothing | Pass on every table | SE-5 acceptance (not built) |
| Cold restore | Time to rebuild from backup in a new environment | Within NFR-3 RTO of 4 h | Not run |
| Export completeness | Resource types held vs exported (census) | Equal | CA/IO export task (not built) |

## Trade-offs with other attributes

- More configuration means more states to test and more places for a clinical rule to differ by site ([safety](../safety/index.md)); the pack rule that "no pack changes the clinical rules" is the guard.
- Row-level multi-tenancy costs performance and complexity ([performance-efficiency](../performance-efficiency/index.md)) but supports isolation ([security](../security/index.md), [privacy](../privacy/index.md)).
- Per-site settings weaken a uniform user experience ([interaction capability](../interaction-capability/index.md)): clinicians who rotate between sites meet different layouts.
- Podman-only simplifies support but narrows installability.

## Gaps and risks

- Every flexibility claim in the spec is a design or a plan; nothing has been run with two replicas, two tenants or two packs.
- Packs written from published guides are machine-assembled until a local person reviews them.
- UK/US weighting means other jurisdictions are less served today (IO-19).
- No migration path between tenants or between deployments has been designed.

## Ideas

1. Build the smallest real pack (one country identifier check) end to end to test the OS-10 design (OS-10, IO-18).
2. Write the XP-5 list of acute-only assumptions before any profile is coded.
3. Run the UI behind two replicas with Redis and record what breaks (OS-8 follow-up).
4. Time a clean install on a second operating system and record it in the README.
5. Decide the tenant model before more tables are added, as retrofitting `tenant_id` on append-only tables is costly (SE-5).
6. Add a "configuration differs from tested" notice to the deployer baseline hazards (AS-18).

## Open questions

- Is one deployment one tenant for 1.0, with tenancy deferred?
- Who may enable a pack, and who reviews it locally?
- Does "replaceable" include bulk export of the audit trail?

## Not verified

No standard text read. No scaling, install or pack run was done by me. The Ratwani, CSI and NHS items are snippets.

## Sources

- https://quality.arc42.org/articles/iso-25010-update-2023 - 2023 changes summary
- https://webstore.ansi.org/preview-pages/ISO/preview_ISO+IEC+25010-2023.pdf - ANSI preview (listed, not read)
- https://ama-assn.org/study-finds-need-performance-standards-ehr-usability-safety - multi-site EHR performance variation
- https://www.csicompanies.com/insights/the-hidden-cost-of-custom-builds-an-epic-ehr-optimization-guide-for-health-system-it-leaders - customisation and upgrades (consultancy)
- https://standards.nhs.uk/published-standards/clinical-risk-management-its-application-in-the-deployment-and-use-of-health-it-systems - DCB0160 page
