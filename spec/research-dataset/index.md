# Research dataset: de-identified data pooled across deploying organisations

Public-source research read 2026-11-14 plus this repository's own files. A proposal and a reading, not legal advice: nothing here is built, and no data protection officer, Caldicott guardian, institutional review board, research ethics committee, lawyer or patient group has reviewed it. **NOT FOR CLINICAL USE.**

Method: web searches and page reads on 2026-11-14. Most US regulatory text was read from a mirror of the regulation [LII514], because the federal pages refused automated reads; the HHS de-identification guidance itself was seen only through secondary summaries (see Not verified). Each claim below carries a source key; "snippet" means only a search snippet or a secondary summary was seen, and **[uncertain]** marks a claim I could not confirm. Repository files read: [privacy-preserving analysis](../privacy-analytics.md), [protected information](../access-control/protected-information.md), [access control](../access-control/index.md), [audit](../audit/index.md), the [DPIA](../../governance/dpia.md), [Epic](../epic/index.md), [data exchange](../data-exchange/index.md), [open questions](../16-open-questions.md) (OQ-HE1, OQ-HE2), [`plan.md`](../../plan.md) §27, and [`tasks.md`](../../tasks.md) entries XP-8, XP-9, IO-4, IO-7 and SE-2, plus `consent.rs` and `policy.rs` in the service. Examples from the US, the UK and the EU are examples, never "the rule"; law belongs in a country pack, not in the core.

## What it is, and what this project is not

The idea, in the style of Epic Cosmos or TriNetX: organisations that deploy this EPR may choose to contribute their patients' data, de-identified, to a dataset pooled across organisations, so that researchers can ask questions no single organisation can answer (a rare condition, a drug's harm, a comparison across regions).

- **Epic Cosmos** says it holds 310 million patients from 2,336 hospitals and 51.9 thousand clinics (the page gives no date) [COSMOS]. Its governance page says "patient identifiers are removed before any data is moved from a healthcare organization", that "line-level data never leaves the secure Cosmos portal", that data "cannot be sold" or used "for advertising or market comparisons", that every user attests to their use and discloses funding, that all actions are audited, and that a council of 15 elected members oversees its "Rules of the Road" [COSMOSGOV]. Participation is opt-in for Epic customers (snippet, a university page) [DREXEL].
- **An older description of Cosmos differs.** A 2021 paper by its own users says Cosmos "contains a limited dataset, as defined by" HIPAA, keeping dates of birth, dates of service, five-digit ZIP codes and a SHA-256 hash of the cross-organisation patient identifier, and reports cohorts under 11 as "10 or fewer"; it lists as limits a single vendor's population, imperfect de-duplication, all-or-none contribution, and that "contributors cannot control which queries run on their data" [TARABICHI]. Today's page says "fully deidentified" [COSMOSGOV]. Whether the model changed, or the two describe different layers, I could not establish **[uncertain]**.
- **TriNetX** describes a network of more than 13,000 clinical sites (its statistics say 14,200+) in 20+ countries and 309 million patients, with data that "stays within those sites" [TRINETX]. A paper by its staff says queries run on de-identified data and "only aggregate counts are provided"; a commissioned summary says it protects data by a minimum count threshold on aggregate results and by obfuscation when individual-level records are shared, applied to the total, the site counts and the subtotal (snippets) [TOPALOGLU] [MALIN].
- **This project is not a vendor.** It is open-source software. It cannot run a central pool, hold a contract with each deploying organisation, or sign a data use agreement. So the design below has two halves: what the **software** must do inside each deployment (extract, de-identify, honour opt-outs, audit), and what a separate **pool operator** (a university, a trusted research environment, a consortium of deploying organisations) must do. The project can write the first half and describe the second; it must not pretend to be the second.

## Who participates, and on what legal basis

### Participants

1. **A deploying organisation** (the controller, or in the US the covered entity) decides to join, and may leave. Joining is never a default of installing the software.
2. **The pool operator** receives what each organisation sends, under a contract with each.
3. **A linkage unit**, separate from the pool operator, holds the key that turns identifiers into linkage tokens (below).
4. **Researchers** on approved projects, who see aggregates or work inside a trusted research environment.
5. **Patients**, whose data it is, who may opt out and who should be represented on the access committee.

### United States

- **De-identified data is outside HIPAA's protections.** The Privacy Rule gives two routes [LII514]:
  - **Expert determination**, §164.514(b)(1): a qualified expert finds "the risk is very small" that the information could identify a person, and documents the methods and results. HHS guidance is reported to say "there is no explicit numerical level of identification risk" (snippet) [HUNTON].
  - **Safe Harbor**, §164.514(b)(2): remove 18 kinds of identifier, including every geographic unit smaller than a state (the first three ZIP digits may stay only where that area holds more than 20,000 people, otherwise "000"), every element of a date except the year, and all ages over 89 (grouped as 90 or older), plus "any other unique identifying number, characteristic, or code"; and the entity must not have "actual knowledge" that what is left could identify someone [LII514].
  - **A re-identification code** may be kept by the discloser if it is not "derived from or related to information about the individual" and its mechanism is not disclosed, §164.514(c) [LII514]. A keyed hash of the name and birth date is derived from information about the individual; whether a keyed token can fit (c), or needs expert determination instead, is a question for counsel **[uncertain]**. PCORnet's linkage vendor states that its keyed, salted hash tokens meet expert determination (snippet) [PCORNET].
  - **Dates shifted** by a per-patient offset are not Safe Harbor dates on a plain reading of "all elements of dates (except year)"; a vendor blog says date shifting is usually treated as an expert-determination method (snippet, secondary) **[uncertain]** [HUNTON].
- **A limited data set** keeps dates, town, state and ZIP code but removes direct identifiers (names, street address, full-face photographs and others), may be used only for research, public health or health-care operations, and needs a **data use agreement** that names the permitted uses and recipients and binds the recipient not to identify or contact anyone and to report misuse, §164.514(e) [LII514]. The 2021 Cosmos paper places Cosmos here [TARABICHI].
- **Business associates.** If the software's host or the pool operator handles identifiable data on an organisation's behalf, it is a business associate. HHS says de-identifying is itself a use of protected health information, so a business associate may de-identify "only to the extent" its agreement authorises (snippet of HHS pages) [HHSBA].
- **The Common Rule.** Under 45 CFR 46.102(e) a human subject is a living individual about whom an investigator obtains identifiable private information; research on data the investigator cannot link to a person may not be human-subjects research, and OHRP's coded-information guidance sets the conditions; exemption 4 covers some secondary research (snippets of university and OHRP summaries) [OHRP]. A 2021 description says Cosmos queries, being de-identified and aggregate, "do not constitute human subjects research" (snippet) [COSMOSIRB]. An institution's review board, not the researcher, decides that.

### United Kingdom and the EU

- **Anonymous versus pseudonymised.** The ICO says identifiability "exists on a spectrum", the test is whether the risk is "sufficiently remote" (not zero), a "motivated intruder" is the person to imagine, data can be "personal data in your hands" yet "anonymous in the hands of other parties", and anonymisation decisions must be reviewed periodically [ICOEFF]. "Pseudonymised data is personal data in the hands of someone who holds the additional information"; the ICO advises against hashing without additional secret data, and notes that the same token for the same person lets people be linked across databases [ICOPSEUDO]. The guidance is under review after the Data (Use and Access) Act 2025 [ICOANON].
- **In the EU**, the EDPB adopted guidelines on pseudonymisation on 16 January 2025 (snippet; I did not see whether a final version followed) [EDPB]. The Court of Justice held on 4 September 2025, in *EDPS v SRB* (C-413/23 P), that pseudonymised data may not be personal data for a recipient with no reasonable means to re-identify, while it stays personal for the discloser who holds the key (commentary, snippets) [SRB]. So **a pooled dataset that is linkable is personal data at least for the linkage unit and the contributing organisation**, and most likely for the pool operator too; this page treats it as personal data throughout.
- **Lawful basis.** Health data is special category under UK GDPR, needing an Article 6 basis and an Article 9 condition (see [protected information](../access-control/protected-information.md)). The Data (Use and Access) Act 2025 gives "scientific research" a statutory meaning that includes commercial research and allows broad consent to areas of research; most of its data-protection provisions commenced on 5 February 2026 (secondary sources) [DUAA].
- **Common-law confidentiality in England** is separate from data protection. Using confidential patient information without consent for research needs support under section 251 of the 2006 Act that governs England's national health service, and Regulation 5 of the Control of Patient Information Regulations 2002, on advice from the Confidentiality Advisory Group of the Health Research Authority (snippet) [CAG].
- **England's national data opt-out** applies to "any disclosure of data for purposes beyond individual care", named as planning, commissioning, invoice validation, national clinical audit and research, and to deceased people's confidential information [NDOO]. It applies where section 251 support is relied on unless an exemption is in place. It does **not** apply to data anonymised in line with the ICO's code, to aggregate or count data, or where the patient gave explicit consent to that use [NDOO]. The Confidentiality Advisory Group expects a project-specific opt-out as well (snippet) [CAGNDOO].
- **Trusted research environments.** England's 2022 review of health data for research recommended a small number of shared trusted research environments as the default way researchers analyse patient records, from which only aggregate results leave after checks (snippets) [GOLDACRE]. The UK Health Data Research Alliance's 2021 principles are built around the Five Safes [HDRTRE]; SATRE, a community specification for trusted research environments (version 2.0.1, 23 July 2026), has pillars for information governance, computing, data management, supporting capabilities and federation, and covers output checking [SATRE].
- **The Five Safes**: safe projects, safe people, safe settings, safe data and safe outputs, first used at the UK Office for National Statistics in the early 2000s; they are scales to balance, not pass or fail tests (secondary sources) [FIVESAFES].
- **The European Health Data Space** regulation (EU) 2025/327 entered into force on 26 March 2025; secondary use through health data access bodies, permits and secure processing environments applies from 26 March 2029, and gives a person a right to opt out of secondary use (Article 71) (snippets of law-firm summaries) [EHDS]. A deployment in the EU in 2029 may be asked to supply a health data access body; that is a different route from a voluntary pool.

## De-identification method

Every step runs **inside the contributing organisation**, before anything leaves. Identifiable data never reaches the pool operator.

1. **Direct identifiers removed.** Names, addresses below the first part of a postcode or three ZIP digits, telephone, email, national health number and every other identifier, photographs, device serial numbers, IP addresses, free-text contact details. The list is the union of Safe Harbor's 18 [LII514] and the local law's; it is a table in the pack, checked by a test that fails when a new column is not classified (like `tests/personal_data_inventory.rs`).
2. **Dates shifted, consistently.** One random offset per patient, applied to every date of that patient so that intervals and order stay true [OHDSIPRIV]. Across organisations the offset must be the same for the same person, or a linked timeline breaks: so the offset is derived by the linkage unit from the token, never by the site from the identity. Year of birth is kept only in bands; ages over 89 become "90 or older" [LII514].
3. **Generalisation.** Geography to region; rare ethnicity, language and occupation codes to "other"; very rare diagnosis and procedure codes rolled up the terminology hierarchy.
4. **k-anonymity on quasi-identifiers.** Each combination of the remaining quasi-identifiers (age band, sex, region, year of first encounter) must match at least *k* people [SWEENEY]. Why it matters: five-digit ZIP, sex and full birth date were "likely" unique for 87% of the US population [SWEENEY], and a model estimated that 15 demographic attributes re-identify 99.98% of Americans, a figure its authors note is disputed [ROCHER]. *k* is a parameter set by the access committee, with no default here.
5. **Rare conditions.** A condition with fewer than *k* patients in a contributing organisation is either generalised or held back from that organisation's contribution; it is never sent with the patient's region. Protected categories ([protected information](../access-control/protected-information.md)) are excluded by default: gender history, HIV and sexual health, substance use (US 42 CFR Part 2), reproductive health, adoption.
6. **Free text excluded** in the first versions. Scrubbing tools report high recall on benchmarks, 99.46% and 99.92% for Philter on two corpora [PHILTER], but its own authors point out that even high recall leaves a large number of identifiers across millions of notes, and an independent comparison found the recall-first tools give up precision [HEIDER]. Free text joins only after a measured residual-identifier rate on this project's own notes, reviewed by a person.
7. **Small-cell suppression on every output.** A count below the minimum cell size is shown as suppressed, with a reason, never as zero and never as the true number. Totals and subtotals are suppressed or perturbed too, so a suppressed cell cannot be recovered by subtraction; TriNetX is reported to act on the total, the site count and the subtotal [MALIN]. Cosmos reports "10 or fewer" below 11 [TARABICHI], and other networks mask at 10 or fewer (snippet) [YUWEBER]; the threshold here is a committee decision (OQ-RD3).
8. **Re-identification risk testing before release.** For each release: measure the smallest equivalence class, the share of unique records, and run a motivated-intruder exercise [ICOEFF] against public data (local news of rare events, published registers). The report is kept with the release. In the US the same report feeds an expert determination [LII514].
9. **Linkage across organisations by keyed hashing at a third party.** A site computes tokens from normalised identifiers (several combinations, so a typo in one field does not lose the match), using a site key, and sends only tokens to the linkage unit. The linkage unit, which holds the second key and never sees identities, matches tokens and issues a **research identifier** per person that has no mathematical relation to the token. PCORnet uses the same shape: site-keyed hashes re-encrypted as transit tokens for the recipient, matched by majority of tokens (snippet) [PCORNET]. The ICO notes that unkeyed hashes of identifiers can be reversed by guessing [ICOPSEUDO]; Bloom-filter encodings, a common alternative, have published frequency and pattern-mining attacks [CHRISTEN] [VIDANAGE]. Linkage error is reported (false matches and missed matches), because a false link merges two people's histories in the research data as the matcher must never do in the record.
10. **Withdrawal and opt-out applied at extraction.** Each extract re-reads the opt-out and the organisation's participation; nothing is cached across extracts.

## The dataset model

Two candidates; the recommendation is OMOP, reached through FHIR.

| | OMOP Common Data Model | FHIR Bulk export to SQL-on-FHIR views |
|---|---|---|
| What | A person-centred relational model with standard vocabularies, managed by OHDSI; current version 5.5 [OMOP] | NDJSON from `$export` (IO-4), flattened by ViewDefinitions into tables (IO-7); the v2 specification's continuous build is 2.1.0-pre, a draft [SOF] |
| For | A large body of shared analysis tools and studies; OHDSI's stated aim is to "keep patient-level data local and share only aggregate results" [OHDSIPRIV] | Uses what this project already plans to build; no second model to maintain |
| Against | Mapping local codes to standard concepts is hard and lossy; the model "can retain" identifiers, so it is not de-identified by itself [OHDSIPRIV] | Few research tools read it; no de-identification in the specification [SOF] |

**Proposal:** `$export` (IO-4) feeds SQL-on-FHIR views (IO-7) that produce OMOP tables, inside the organisation. The de-identification steps above run on those tables. The same views serve the organisation's own quality work, so the pipeline is tested by local use before any data leaves.

**Shape of operation, federated first.** Two tiers:

- **Tier A, aggregate queries, federated.** The query travels to each site's de-identified OMOP store; each returns suppressed counts; the pool combines them and suppresses again. No row leaves a site. This is the TriNetX and OHDSI shape [TOPALOGLU] [OHDSIPRIV], and close to OpenSAFELY's "code to the data", where researchers write code against dummy data and outputs are checked by a person before release (snippets) [OPENSAFELY].
- **Tier B, row-level, in a trusted research environment.** Rows from linked sites, de-identified and tokenised, are held by the pool operator inside a trusted research environment that meets SATRE [SATRE]; approved researchers work there; only checked outputs leave. This is the Cosmos "line-level data never leaves" shape [COSMOSGOV] and the shape England's review recommends [GOLDACRE].

Tier A needs no central copy of rows and should come first.

## Governance

- **A data access committee** with members from contributing organisations, researchers, a privacy lead and at least two patient or public members, with published terms of reference, minutes and a register of approved projects. Cosmos's elected council is a precedent for organisation members [COSMOSGOV]; patient members are this page's addition.
- **Approved projects only** (safe projects): purpose, protocol, ethics or review-board decision where needed, funding disclosed, lay summary published, an end date. No project for advertising, for marketing, for comparing named organisations against their will, for pricing, or for re-identification. Cosmos forbids sale, advertising and market comparison [COSMOSGOV]; this page proposes the same and adds insurance underwriting and law enforcement.
- **Safe people**: named, trained, under a signed agreement; in the US a data use agreement where a limited data set is used [LII514].
- **Output checking** (safe outputs): principles-based checking by a trained checker for Tier B, as the statistical disclosure control handbooks describe [SDCHB] [SACRO]; rules-based automatic checks (minimum cell, dominance, differencing) for Tier A, with a person reviewing anything unusual.
- **Audit.** Every extract, query, release and output decision is an append-only audit event at the site and at the pool, as the service already audits every read ([audit](../audit/index.md)). The patient-facing access log ([DPIA](../../governance/dpia.md) §3) shows "your organisation's de-identified data was included in research extract N on date D", without the research identifier.
- **Withdrawal by an organisation, going forward.** An organisation that leaves stops contributing at once. Rows already in Tier B stay for projects already approved, then are deleted at those projects' end dates; aggregates already published cannot be recalled. This must be in the contract and stated plainly to patients.
- **Opt-out by a patient, going forward.** An opt-out recorded before an extract excludes the patient from it and every later one. Rows already sent cannot be found without re-identifying them: so a patient's opt-out removes their research identifier from the next refresh of Tier B (the linkage unit holds the map from token to research identifier and can drop it), and the pool deletes the rows with that identifier. Published aggregates stay. The page must not promise more than this.
- **The record stays append-only.** Nothing in the clinical record is changed or deleted for research; withdrawal and opt-out are new records, and research exclusion is computed from them.
- **Transparency.** A public register of approved projects, results and the current minimum cell size and *k*; an annual report of opt-out counts, re-identification tests and incidents.

## What researchers get

| | Tier A, aggregate | Tier B, row-level in a trusted research environment |
|---|---|---|
| Who | Any approved researcher at a contributing organisation | Approved researchers on an approved project, named |
| What | Counts, rates, simple survival curves, suppressed | De-identified, tokenised rows for the project's cohort only |
| Where | A query page | Inside the environment; no download |
| Leaves | The suppressed result | Outputs that pass checking |
| Risk | Differencing and repeated queries; mitigated by suppression, rounding and query logging | A breach of the environment; a researcher who knows a patient; mitigated by the Five Safes |
| Lawful footing (England) | Aggregate and anonymised: outside the national data opt-out [NDOO], but the access committee should honour it anyway (Rule 7) | Pseudonymised personal data: needs a lawful basis, confidentiality footing (consent or section 251), and the opt-out [NDOO] [CAG] |

## Consent and patient voice

- **The default is opt-out**, not consent, because consent-only datasets are known to skew (a reason given for opt-out models in public debate; no source read here measured the skew for this design) **[uncertain]**. A patient's explicit consent to a specific study overrides their opt-out for that study only [NDOO].
- **Every opt-out that applies is honoured, even where the law does not require it**: England's national data opt-out, a project-specific opt-out, a local opt-out, and the EU's from 2029 [EHDS]. A patient should not have to know which applies.
- **Public trust has failed before.** England's care.data plan was cancelled in July 2016 after loss of public trust, and in May 2021, when a plan to collect general practice data was announced with a short window, 107,429 people registered a national opt-out in one month against 72,225 in the ten months before (snippets; sources partly advocacy) [CAREDATA]. The lesson for this design is notice before collection, a long window, plain words, and the right to see what was used.
- **Patient voice** in the design and in the access committee, through a patient and public panel paid for their time; plain-language notices in every locale the deployment uses (machine translation is not reviewed translation).

## The honest case for and against

**For**
- Questions no single organisation can answer: rare conditions, rare harms, differences between groups and regions, equity of care.
- Contributing organisations learn about their own patients from the same pipeline.
- Organisations can do it with open software and published rules rather than handing data to a vendor whose rules are not public (Cosmos's rules "are not publicly published", snippet [DREXEL]).

**Against**
- **Re-identification is never zero** [ICOEFF] and rich longitudinal records are close to unique [ROCHER]. A pool across many organisations is a richer target than any one.
- **This project cannot run it.** Without a funded, accountable pool operator and linkage unit, the software half is useless and could tempt a deployer to send data somewhere without the other half.
- **Cost and staff.** Output checking is manual [privacy-preserving analysis](../privacy-analytics.md); an access committee meets; a trusted research environment is a service with staff.
- **Bias.** Users of one piece of software are not a population; the Cosmos paper names single-vendor selection bias [TARABICHI], and opt-outs remove people unevenly [CAREDATA].
- **Commercial pressure.** "Scientific research" now includes commercial research in UK law [DUAA]; patients may not expect that.
- **Distraction.** At milestone M0 there is no clinical record of substance (no problem list, no notes, no medicines; see [implementation status](../14-implementation-status.md)). A research dataset of allergies and vital signs is of little value.

## Rules

1. Installing the software never enrols an organisation; joining is an explicit, recorded act by the organisation, and leaving is one act.
2. No identifiable data leaves a contributing organisation for the pool. A test feeds an extract containing every classified identifier column and fails if any value of one appears in the output.
3. The extract job reads under purpose of use `research`, never under a care relationship, and runs as a dedicated service identity; no human role gains `research` from this work.
4. Every extract, query and release writes an append-only audit event in the same transaction as the decision to release it.
5. A patient with an active opt-out (local, national or project-specific) at the moment of extraction is absent from that extract and all later ones; a test records an opt-out between two extracts and finds the patient in the first and not the second.
6. England's national data opt-out covers planning as well as research [NDOO]; the extract job honours the opt-out for every secondary purpose, not only the `research` code. A test checks the opt-out refuses an extract requested under `operations` for planning.
7. Tier A results honour every opt-out even though aggregate data is outside England's national data opt-out [NDOO].
8. A count below the minimum cell size is returned as null with the reason `suppressed`, never as 0 and never as the true value; a test sends a cohort of 1 and of 0 and finds both suppressed with the same reason.
9. A total or subtotal from which a suppressed cell could be recovered by subtraction is itself suppressed or perturbed; a test with one suppressed cell and an exact total fails if the cell can be computed.
10. Date shifting uses one offset per patient across all their events, derived by the linkage unit; a test finds every interval between a patient's events unchanged.
11. No free text, no image, no document attachment is in any extract until a measured residual-identifier rate on this project's notes has been accepted by the access committee.
12. Protected categories ([protected information](../access-control/protected-information.md)) are excluded by default; a test with each label finds the record absent.
13. Linkage tokens use a secret key; a token computed without the key does not match any stored token.
14. Missing data is shown as missing with a reason in every output: an absent value never counts as "no" or as zero.
15. Withdrawal of an organisation stops its next extract; it never edits or deletes the clinical record.

## Hazards to add when built

Described only; none is in the hazard log, and none is accepted or closed. They go to the hazard log (AS-1) with the first code, for the Clinical Safety Officer and the data protection officer.

- **RD-H1 Re-identification of a patient** from a pooled extract or an output, by linking to public or local knowledge (a rare condition in a small region; a famous patient; a staff member's own record). Harm: disclosure of protected information, loss of trust, people avoiding care.
- **RD-H2 A withdrawn or opted-out patient's data still used**: an extract cached from before the opt-out; an opt-out recorded at another organisation not seen by this one; a row in Tier B not dropped at refresh; the opt-out honoured for `research` but not for planning.
- **RD-H3 A false link across organisations** merges two people's histories in research data, so a study's finding is wrong; a missed link counts one person twice.
- **RD-H4 A suppressed cell recovered** by differencing two queries or a total.
- **RD-H5 A research finding fed back into care** (a risk score built on the pool) carries the pool's bias into a clinical decision. This is the one route by which the dataset becomes clinical: any such use goes through clinical safety as a new feature.
- **RD-H6 Free text with an identifier** released because scrubbing missed it.

## First slice

On synthetic data only, inside one deployment, nothing leaving it:

1. An extract job that reads the seeded demo under purpose `research` as a service identity, honours the local opt-out and an organisation participation flag, and writes an audit event (Rules 2 to 5).
2. Output: the OMOP `person` and `observation_period` tables plus allergies and measurements, with identifiers removed and dates shifted.
3. A Tier A count query on that store with suppression and null-with-reason (Rules 8, 9, 14).
4. A test per rule; a measured smallest equivalence class on the synthetic data, reported, not claimed as a real risk figure.

No linkage, no pool, no second organisation, no free text in the first slice.

## What this repo has and lacks

| Need | Has | Lacks |
|---|---|---|
| Purpose of use on each request | `Purpose::Research` exists and is permitted to no role (`policy.rs`, test `purpose_must_fit_the_role`) | A service identity for extraction; a planning purpose distinct from `operations` |
| Patient opt-out | A local record of kind `NationalDataOptOut` that denies `research` and nothing else (`consent.rs`, test `the_opt_out_refuses_research_and_nothing_else`) | A check against England's national service; refusal for planning (Rule 6, a gap against [NDOO]); a project-specific opt-out |
| Consent | SE-2, partly landed: identity, allergies | FHIR `Consent`; "bulk export honours consent" (no export exists) |
| Audit | Every read and write, append-only, hash-chained | Events for extract, query, release and output decision |
| Protected labels | Masking by label | Labels on clinical records (SE-4); exclusion from extracts |
| Bulk export | Planned (IO-4) | Built |
| SQL-on-FHIR views | Planned (IO-7) | Built; OMOP mapping |
| Secondary-use design | [Privacy-preserving analysis](../privacy-analytics.md) design rules; XP-9 | Minimum cell size, *k*, key custody for linkage, the DPO's answer (OQ-HE1) |
| Clinical content worth researching | Allergies, vital signs | Problems, medicines, notes, results, encounters |
| Personal data inventory | Every column classified, with a test | A "research release" class per column |

## Not verified

- The HHS de-identification guidance itself (on free text, date shifting, and whether an expert determination may expire) was not read: the HHS site refused automated reads, and a mirrored copy returned 404. Its content here is from a law-firm summary and a vendor blog [HUNTON]. The regulation text was read on a mirror [LII514], not on the eCFR.
- The OHRP coded-information conditions and the text of 45 CFR 46 were seen only in summaries [OHRP].
- The ICO pages were read; the 2025 status note says the guidance is under review after the Data (Use and Access) Act [ICOANON]. The Act's research provisions were seen through secondary summaries [DUAA].
- The EDPB guidelines, the *SRB* judgment and the EHDS regulation were seen through commentary, not the official texts [EDPB] [SRB] [EHDS].
- The national data opt-out guidance pages read are version 4 chapters last edited in 2020 [NDOO]; whether a later version changed them was not checked.
- The Five Safes paper, the UK Health Data Research Alliance principles, the SDC handbooks, the 2022 review and the care.data history were seen as summaries [FIVESAFES] [HDRTRE] [SDCHB] [SACRO] [GOLDACRE] [CAREDATA].
- Cosmos and TriNetX figures are their own claims; Cosmos's de-identification method and its "Rules of the Road" are not public; the difference between the 2021 limited-data-set description and today's "fully deidentified" was not resolved.
- The bias of consent-only datasets was not sourced.
- No re-identification test, linkage test, scrubber or OMOP mapping was run. No figure here comes from this project's data.

## Sources

- [LII514] 45 CFR 164.514, Cornell Legal Information Institute: https://www.law.cornell.edu/cfr/text/45/164.514
- [HUNTON] Hunton, HHS guidance on de-identification (secondary): https://www.hunton.com/privacy-and-cybersecurity-law-blog/hhs-publishes-guidance-on-how-to-de-identify-protected-health-information ; the HHS page itself (not read): https://www.hhs.gov/hipaa/for-professionals/special-topics/de-identification/index.html
- [HHSBA] HHS FAQ, may a health information organization de-identify information (snippet): https://www.hhs.gov/hipaa/for-professionals/faq/544/may-a-health-information-organization-de-identify-information/index.html
- [OHRP] OHRP, coded private information or biospecimens used in research (snippet): https://hhs.gov/ohrp/coded-private-information-or-biospecimens-used-research.html
- [COSMOS] Epic Cosmos: https://cosmos.epic.com/
- [COSMOSGOV] Epic Cosmos governance: https://cosmos.epic.com/governance/
- [TARABICHI] Tarabichi et al., The Cosmos Collaborative, ACI Open 2021: https://pmc.ncbi.nlm.nih.gov/articles/PMC8775787
- [COSMOSIRB] Cosmos and human-subjects research, 2021 (snippet): https://www.thieme-connect.com/products/ejournals/pdf/10.1055/s-0041-1731004.pdf
- [DREXEL] Drexel IT, TriNetX and Cosmos (snippet): https://drexel.edu/it/services/catalog/trinetx-cosmos/~/media/4AC67D80A4E6414CBECB6054487111A8.ashx
- [TRINETX] TriNetX: https://trinetx.com/
- [TOPALOGLU] Topaloglu and Palchuk, JCO Clinical Cancer Informatics 2018 (snippet): https://pmc.ncbi.nlm.nih.gov/articles/pmid/30652541
- [MALIN] Malin, TriNetX empirical summary 2020 (snippet, vendor-commissioned): https://trinetx.com/wp-content/uploads/2021/12/TriNetX-Empirical-Summary-by-Brad-Malin-2020.pdf
- [YUWEBER] Yu and Weber, JMIR 2020 (snippet): https://pmc.ncbi.nlm.nih.gov/articles/PMC7671849
- [ICOANON] ICO, anonymisation guidance contents: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/data-sharing/anonymisation/
- [ICOEFF] ICO, how do we ensure anonymisation is effective: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/data-sharing/anonymisation/how-do-we-ensure-anonymisation-is-effective/
- [ICOPSEUDO] ICO, pseudonymisation: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/data-sharing/anonymisation/pseudonymisation/
- [EDPB] EDPB Guidelines 01/2025 on pseudonymisation (snippet): https://www.edpb.europa.eu/our-work-tools/documents/public-consultations/2025/guidelines-012025-pseudonymisation_en
- [SRB] Commentary on *EDPS v SRB*, C-413/23 P (snippet): https://www.cliffordchance.com/insights/resources/blogs/talking-tech/en/articles/2025/09/pseudonymized-data-after-edps-v-srb.html
- [DUAA] UK government, Data (Use and Access) Act 2025 changes (snippet): https://www.gov.uk/guidance/data-use-and-access-act-2025-data-protection-and-privacy-changes ; commencement (secondary): https://privacymatters.dlapiper.com/2026/02/uk-commencement-of-the-data-protection-provisions-in-the-data-use-and-access-act/
- [CAG] Health Research Authority, guidance for Confidentiality Advisory Group applicants (snippet): https://www.hra.nhs.uk/about-us/committees-and-services/confidentiality-advisory-group/guidance-cag-applicants/
- [CAGNDOO] Health Research Authority, national data opt-out information for research organisations (snippet): https://www.hra.nhs.uk/about-us/committees-and-services/confidentiality-advisory-group/national-data-opt-out-supplementary-information-for-research-organisations/
- [NDOO] National data opt-out operational policy guidance, what are national data opt-outs: https://digital.nhs.uk/services/national-data-opt-out/operational-policy-guidance-document/what-are-national-data-opt-outs
- [GOLDACRE] Better, broader, safer: using health data for research and analysis, 2022 (snippet): https://www.gov.uk/government/publications/better-broader-safer-using-health-data-for-research-and-analysis/better-broader-safer-using-health-data-for-research-and-analysis
- [HDRTRE] UK Health Data Research Alliance, Building Trusted Research Environments, 2021 (snippet): https://zenodo.org/records/5766513
- [SATRE] SATRE specification: https://satre-specification.readthedocs.io/en/stable/
- [FIVESAFES] Five Safes, overview (secondary): https://en.wikipedia.org/wiki/Five_safes ; Ritchie (UWE repository): https://uwe-repository.worktribe.com/OutputFile/880718
- [SDCHB] Handbook on Statistical Disclosure Control for Outputs, 2019 (snippet): https://ukdataservice.ac.uk/thf_datareport_aw_web
- [SACRO] The SACRO guide to statistical output checking, 2023 (snippet): https://zenodo.org/records/10282526/files/The%20SACRO%20guide%20to%20statistical%20output%20checking.pdf
- [EHDS] European Commission, European Health Data Space regulation (snippet): https://health.ec.europa.eu/ehealth-digital-health-and-care/european-health-data-space-regulation-ehds_en ; summary of secondary use: https://www.mccannfitzgerald.com/knowledge/pharma-and-life-sciences/ehds-regulation-secondary-use-of-electronic-health-data
- [OMOP] OHDSI, OMOP Common Data Model: https://ohdsi.github.io/CommonDataModel/
- [OHDSIPRIV] OHDSI, patient privacy and OMOP: https://ohdsi.github.io/CommonDataModel/cdmPrivacy.html
- [SOF] SQL on FHIR v2, continuous build: https://build.fhir.org/ig/FHIR/sql-on-fhir-v2/
- [OPENSAFELY] Bennett Institute, how OpenSAFELY works (snippet): https://www.bennett.ox.ac.uk/blog/2025/01/how-opensafely-works/ ; federation: https://docs.opensafely.org/federation
- [SWEENEY] Sweeney, k-anonymity: a model for protecting privacy, 2002: https://www.cs.ubc.ca/~laks/cpsc504/kanonymity.pdf
- [ROCHER] Rocher, Hendrickx and de Montjoye, Nature Communications 2019: https://pmc.ncbi.nlm.nih.gov/articles/PMC6650473
- [PHILTER] Norgeot et al., Philter, npj Digital Medicine 2020: https://pmc.ncbi.nlm.nih.gov/articles/PMC7156708
- [HEIDER] Heider and Meystre, multisystem de-identification evaluation, JMIR 2024 (snippet): https://www.jmir.org/2024/1/e55676
- [PCORNET] PPRL framework within PCORnet, BMC Research Notes 2022 (snippet): https://www.ncbi.nlm.nih.gov/pmc/articles/PMC9620597/
- [CHRISTEN] Christen et al., pattern-mining attack on Bloom-filter PPRL, PAKDD 2018 (snippet): https://users.cecs.anu.edu.au/~christen/publications/christen2018pakdd.pdf
- [VIDANAGE] Vidanage et al., a taxonomy of attacks on PPRL, Journal of Privacy and Confidentiality 2022 (snippet): https://journalprivacyconfidentiality.org/index.php/jpc/article/view/764
- [CAREDATA] care.data and GPDPR opt-outs (snippets; one source is an advocacy group): https://estsjournal.org/index.php/ests/article/download/1197/469 ; https://medconfidential.org/care-data-2013-2016/
