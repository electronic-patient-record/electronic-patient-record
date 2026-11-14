# Analytics: operational reports and analyst exploration, kept apart from the live record

Public-source research read 2026-11-14 plus this repository's own files. A proposal: nothing here is built, and no clinician, data protection officer, Clinical Safety Officer or analyst has reviewed it.

The question: how does a site get (1) operational reports (bed occupancy, waits, referral turnaround, overdue work, audit and access reports) and (2) ad hoc exploration by analysts, without analysts querying the live clinical store, without losing consent or sensitivity labels on the way, and without anything flowing back into the record? The short answer: a separate reporting database fed by PostgreSQL logical replication for operational reports, and a separate, pseudonymised, columnar exploration copy built from FHIR Bulk Data export and SQL-on-FHIR views, with one registry of metric definitions, a minimum cell size, a stated freshness on every number, and no write path back.

## Method

Web reading of primary sources where they could be fetched (the PostgreSQL 18 manual, the HL7 SQL-on-FHIR and Bulk Data guides, the OHDSI OMOP CDM pages, DuckDB, OpenSAFELY); search snippets for the rest, marked "(snippet)" in Sources. Claims I could not confirm are marked [uncertain]. Repository files read: [privacy-preserving analysis](../privacy-analytics.md), [productivity measurement](../productivity-measurement/index.md), [persistence](../10-persistence.md), [schema map](../schema-map.md), [domain model 5.2.1](../05-domain-model.md), [virtual ward](../virtual-ward/index.md) (for page shape), [implementation status](../14-implementation-status.md), [audit](../audit/index.md), [access control](../access-control/index.md), [protected information](../access-control/protected-information.md), [NFR-12](../07-non-functional-requirements.md), and the entries IO-4, IO-7, OR-7, XP-9, XP-14 and PM-1 to PM-5 in [`tasks.md`](../../tasks.md). Nothing was installed or run.

## What the store gives an analytics design, and what it does not

- **Append-only versions.** Each clinical record is a run of versions in a `*_versions` table, and a `*_current` view picks the latest ([persistence](../10-persistence.md)). Nothing is updated or deleted. So any past state can be rebuilt exactly: "what did the referral list look like at 09:00 on Monday" is a query, not a backup restore. This is the single most useful property for reporting, because a report can be reproduced.
- **Insert-only change stream.** Because the version tables, `audit_events` and the outbox record only ever receive `INSERT`, a logical replication publication can publish inserts only. PostgreSQL then lets a row filter use any column, where filters on published updates and deletes are limited to replica-identity columns [PGROW].
- **Hash-chained audit.** Each `audit_events` row carries `prev_digest` and `digest` ([audit](../audit/index.md)). A copy can check that each row's `prev_digest` equals the previous row's `digest`, which detects a gap in what was copied without holding the MAC key (my reasoning from the column design; not tested).
- **`recorded_at` is not event time.** Most version tables carry only `recorded_at`. `observation_versions` also carries `effective_at`. A wait or turnaround measured from `recorded_at` measures when the record was written, which may be later than when the event happened. Each metric must say which time it uses.
- **What is not there.** There is no encounter, admission, bed, ward, or task table ([schema map](../schema-map.md)); the HL7 ADT feed is inbound and does not yet store a stay. No clinical record yet carries a security label (SE-4, [protected information](../access-control/protected-information.md)). No `tenant_id` column exists yet ([persistence](../10-persistence.md), planned). So bed occupancy cannot be computed today, and any report that needs it returns null with the reason "no bed or stay data is recorded", never zero.

## The two kinds of use, and the line between them and direct care

| Use | Who | Rows or aggregates | Where it runs | Purpose of use |
|---|---|---|---|---|
| Worklist (overdue acknowledgement, unsigned result, unanswered referral) | Clinicians and administrators caring for the patient | Named patients | **The live service**, under the care-relationship check | Treatment |
| Audit investigation (who opened this record) | Auditor | Named staff and patients | **The live service** (`GET /api/audit`, already built) | Operations or system administration |
| Operational report (turnaround, backlog, occupancy, access volume) | Managers, operational analysts | Aggregates, minimum cell size | **Reporting database** | Healthcare operations, quality improvement |
| Ad hoc exploration | Approved analysts on a named project | Pseudonymised rows inside the environment; aggregates out | **Exploration environment** | Quality improvement, research, as approved |

A report that names patients is a worklist and belongs in the clinical application, with its access checks and audit. Analytics never produces a list of named patients for action. This keeps the clinical access model in one place and keeps NFR-12 ("support only") enforceable.

Purpose-of-use codes come from the HL7 PurposeOfUse value set: `HOPERAT` (healthcare operations), `HQUALIMP` (health quality improvement) and `HRESCH` (healthcare research) are the ones that apply here (snippet) [POU]. The audit table already has a `purpose_of_use` column ([schema map](../schema-map.md)).

## Options for getting data out of the live store

| Option | What it is | For | Against | Verdict |
|---|---|---|---|---|
| **Hot standby (physical read replica)** | A byte-for-byte copy that accepts read-only queries [PGHS] | Simple; exact; no schema work | It is the whole database, so it cannot drop sensitive tables or rows; it cannot hold marts ("not even temporary tables may be written") [PGHS]; long analyst queries either get cancelled by replay or delay replay, and `hot_standby_feedback` stops dead-row cleanup on the primary, so bloat [PGHS] | **No** for analysts. Keep standbys for availability only |
| **Logical replication into a separate PostgreSQL 18** | A publication on the primary streams chosen tables; a subscriber applies them [PGROW] [PGCOL] | Chosen tables only; insert-only publication with row filters on any column [PGROW]; the subscriber is writable, so marts and indexes live there; same SQL and same `*_current` view logic | A slot retains WAL until consumed and can fill the primary's disk; slots "know nothing about the state of their consumer(s)" [PGLD]; column lists are explicitly **not a security control** [PGCOL] | **Recommended** for operational reports |
| **CDC through Debezium and a message broker** | Logical decoding with `pgoutput` into Kafka topics (snippet) [DBZ] | Many consumers; replay | A broker to run and secure; the same slot risk; a second copy of every change in the broker | **Not now.** Reconsider if several downstream systems need the stream |
| **The service's own outbox** | `axa_outbox_*` already records events in the same transaction as the change ([persistence](../10-persistence.md)) | Application-level events, already audited | It carries notification events, not the full record; using it as a feed couples analytics to the event design | Use for event counts only, not as the copy |
| **FHIR Bulk Data `$export` (NDJSON)** | Asynchronous system, `Patient` or `Group` export; NDJSON required; `_since` for increments; a `deleted` list in increments [BULK2] | Standard; portable between sites; the export is the natural point to apply consent and labels (IO-4) | Batch, not live; needs FHIR resources for each record kind (today only Patient, AllergyIntolerance and AuditEvent are served); `_typeFilter` and `_elements` are experimental [BULK2] | **Recommended** as the feed for exploration and for sharing outside the site |
| **SQL-on-FHIR v2 ViewDefinitions** | A portable definition of a flat table over one FHIR resource type, with FHIRPath columns and `forEach` unnesting; runners write database tables, Parquet or CSV [SOF2] [SOF3] | Tech-stack independent [SOF3]; shareable between sites; turns NDJSON into analyst tables | No joins, sorting or aggregation by design [SOF2]; version churn (2.0.0 released; 3.0.0 is a ballot with new `$sql-run` and `$sql-export` operations) [SOF2] [SOF3] | **Recommended** with the Bulk export (IO-7), pinned to 2.0.0 |
| **OMOP CDM copy** | A person-centred research model (current release v5.5) with standardised vocabularies; tables such as PERSON, VISIT_OCCURRENCE, CONDITION_OCCURRENCE, MEASUREMENT [OMOP] [OMOP54] | Joins research networks and their tools; an HL7 FHIR-to-OMOP guide exists (informative, targets v5.4) (snippet) [F2OMOP] | Vocabulary mapping is a large, clinical task; no encounter or problem list here yet to map; not shaped for operational reports | **Later**, when a research partner asks |
| **Modelled marts (dbt-style)** | SQL models in version control, with tests and a freshness check per source (snippet) [DBTF] | Reviewed, tested, reproducible transformations | Another tool and language for contributors | **Recommended** as the shape; the tool is an open question |
| **Columnar engine: DuckDB** | Embedded analytical engine; reads Parquet; its Postgres extension can attach a database `READ_ONLY` [DUCKPG] | One file, no server; good for an exploration environment over Parquet | Without `READ_ONLY` the Postgres extension can write, including `DELETE` and arbitrary statements [DUCKPG] | **Recommended** for exploration, never attached to the primary |
| **Columnar engine: ClickHouse** | A column store server; `MaterializedPostgreSQL` replicates from Postgres (snippet) [CHMPG] | Very large volumes | The replication engine is marked experimental and not supported in ClickHouse Cloud (snippet) [CHMPG]; a server to run | **Not needed** at the expected volume [uncertain: volume not measured] |

Pathling is an open-source toolkit that implements SQL-on-FHIR views and Bulk Data and writes NDJSON, CSV or Parquet; it calls itself experimental (snippet) [PATH]. It is a candidate runner, not evaluated.

### Recommendation

1. **Reporting database (operational reports).** A separate PostgreSQL 18 instance, owned by the deployer, subscribed to an insert-only publication of named tables. It holds the same sensitivity as the primary and sits inside the same trust boundary, because column lists do not protect columns [PGCOL]. Analysts do not query its raw tables; they query marts and the semantic layer built in it.
2. **Exploration environment (ad hoc).** Built from Bulk Data `$export` NDJSON, flattened by pinned SQL-on-FHIR 2.0.0 ViewDefinitions to Parquet, pseudonymised, with consent and labels applied before the files are written, and queried with DuckDB from notebooks or a SQL workbench. Results leave only through output checking.
3. **No analytics on the primary or on a standby**, ever, except the database owner's own diagnostics.
4. **OMOP later**, as a mapping from the same ViewDefinitions output, when a research partner needs it.

## The replication slot is a clinical availability risk

A logical slot keeps WAL on the primary until the subscriber confirms it. If the reporting database stops, WAL accumulates, and in extreme cases the primary shuts down [PGLD]. That would take the clinical record offline to protect a report. PostgreSQL 18 offers two limits: `max_slot_wal_keep_size` (default `-1`, unlimited) caps retained WAL, after which the subscriber can no longer continue; and `idle_replication_slot_timeout` (default `0`, off) invalidates a slot left inactive [PGREPL]. The design sets both, so the reporting copy breaks before the clinical store does, and the break is an alert and a resynchronisation, not an outage. Slot lag is monitored from `pg_stat_replication.replay_lag` on the primary and `pg_stat_subscription.latest_end_time` on the subscriber [PGSTAT].

## Metric registry and semantic layer

One registry of metric definitions, as files in version control, read by both the reports and the exploration environment. Each entry has:

| Field | Meaning |
|---|---|
| `id`, `version` | A stable name and an integer version; a change of meaning is a new version, never an edit in place |
| `owner` | A named role (for example, the referral service lead), who approves each version |
| `definition` | Plain-English sentence a manager can check |
| `numerator`, `denominator`, `time_basis` | Which records, and whether time is `recorded_at` or an event time |
| `source` | Tables or ViewDefinitions, with their versions |
| `absent` | What happens when the source is missing or empty: the result is null with a stated reason, never zero |
| `min_cell`, `rounding` | Disclosure rules for this metric (see below) |
| `max_age` | How old the data may be before the number is shown as stale |
| `balancing` | The paired safety measure, required for any speed or volume metric ([productivity](../productivity-measurement/index.md), PM-5) |
| `review_status` | `draft`, `reviewed`, `retired`; who reviewed and when |

Tools: dbt's MetricFlow defines semantic models and metrics in YAML (snippet) [MF]; dbt Labs says it relicensed MetricFlow to Apache 2.0 in October 2025, from version 0.209.0 (snippet; secondary sources only) [MFLIC] [uncertain]. The registry should be tool-neutral so it can be read by a small project-owned runner first and mapped to MetricFlow later if wanted.

Candidate first metrics, with their source today:

| Metric | Source in this repo | Built today? |
|---|---|---|
| Referral acknowledgement time (median, 90th centile) and open unacknowledged count by age band | `referral_versions`, `referral_acknowledgements` | Data exists |
| Referral turnaround (sent to accepted, declined or redirected) | `referral_versions` states ([closed-loop referral](../interoperability/closed-loop-referral.md)) | Data exists |
| Unmatched referral outcomes awaiting clearance, and time to clear | `referral_unmatched`, `referral_unmatched_clearances` | Data exists |
| Result sign-off delay, and unsigned results by age band | `result_versions`, `result_signoffs` | Data exists |
| Feed refusals awaiting resolution | `feed_refusals`, `feed_refusal_resolutions` ([data quality](../data-quality/index.md)) | Data exists |
| Break-the-glass grants and their review time | `break_glass_grants`, `break_glass_reviews` | Data exists |
| Record access volume by role and organisation, refusals, out-of-hours share | `audit_events` (`detail.actor`) | Data exists from 2026-10-21 |
| Alert override rate per rule (OR-7) | None | Not built: null, "no alert data" |
| Bed occupancy | None | Not built: null, "no bed or stay data" |
| Referral-to-treatment wait | None (no treatment start) | Not built |
| Time in each pathway step (XP-14) | Referrals only; pathways not built | Partly |

Definitions differ between countries and must be pinned, not assumed. Two examples: England's bed return counts an overnight bed as occupied if a patient is in it at midnight and counts only staffed, available beds (snippet) [KH03]; England's referral-to-treatment rules start a clock at referral to a consultant-led service and stop it at first definitive treatment, and a missed appointment does not stop it (snippet) [RTT]. Both are examples for a country pack, never the core's definition.

## Small numbers and disclosure control

- **Default minimum cell size of 10.** Ten is a common threshold, though five and twenty are also used and the right number is "an unsolved problem" (snippet) [RITCHIE]. One US federal policy forbids showing counts of 1 to 10 and says a zero is not a violation (snippet) [CMS]. England's hospital statistics show 1 to 7 as an asterisk below national level and round other counts to the nearest 5 (snippet) [HESDC]. Proposal: counts from 1 to 9 are withheld by default, each site may raise the threshold but not lower it, and a site's country pack may add rounding.
- **Secondary disclosure.** A withheld cell can be recovered from a total and its other cells. Totals are computed after withholding, or the next-smallest cell is also withheld [CMS] [HESDC].
- **Percentages** from rounded numbers can be far from the truth on small denominators [HESDC]; a rate is withheld when its denominator is under the threshold.
- **A withheld cell is shown as withheld, never as zero or blank.** Zero is shown only when it is a true count of zero. Missing data is a third state: null with a reason.
- **Staff measures at team level only**, with the same threshold ([productivity](../productivity-measurement/index.md), PM-2).
- **Naming in code.** NFR-12's tripwire fails on a migration column named like `suppress` or `withhold` without a written reason ([NFR-12](../07-non-functional-requirements.md)). If disclosure fields ever live in the service's migrations, they carry that written reason; the reporting database's schema is not the service's and is outside that tripwire.
- Output-checking guides with rules of thumb by output class exist (the SACRO guide, snippet) [SACRO]; differential privacy is an option for published outputs ([privacy-preserving analysis](../privacy-analytics.md)).

## Access control and audit for analysts

- **Five Safes.** Safe projects, people, settings, data and outputs is the frame used by secure data services (snippet) [FIVE]; this page maps to it: project approval names the purpose; roles and training gate people; the two environments are the settings; pseudonymisation and label handling make the data safe; output checking guards outputs.
- **Roles.** `report-viewer` (marts and reports only), `analyst-operational` (reporting database marts, aggregated), `analyst-project` (exploration environment for one approved project), `output-checker` (a different person from the analyst), `metric-owner`. None of these is a clinical role, and none exists in the live service's role table.
- **Purpose of use on every session.** A session starts with a project or report id and a purpose code (`HOPERAT`, `HQUALIMP`, `HRESCH`) [POU]; access without one is refused.
- **Row-level limits.** Marts carry the organisation or site; PostgreSQL row security restricts each analyst to the sites the project names. Policies are default deny once enabled, the table owner bypasses them unless `FORCE ROW LEVEL SECURITY` is set, and superusers and `BYPASSRLS` roles always bypass [PGRLS]. So no analyst role is an owner, superuser or `BYPASSRLS`, and every mart has `FORCE ROW LEVEL SECURITY`.
- **Consent carried into the copy.** A patient's objection to secondary use, where the law gives one, is applied when the copy is made, not left to the analyst. The current `consent_versions` covers identity only ([access control](../access-control/index.md)); a secondary-use provision is a new consent kind and an open question.
- **Labels carried into the copy.** Every exported row keeps its confidentiality and sensitivity labels as columns. Rows labelled restricted or very restricted, or with a sensitive category (`HIV`, `STD`, `SUD`, `ETH`, `PSY` and the others in [protected information](../access-control/protected-information.md)), are excluded from the exploration environment unless the project approval names that category; in the reporting database they count in aggregates only through marts that have been reviewed for that category.
- **Merged and linked patients.** Counts use `patient_link_current` so one person counted under two ids is counted once; a link created later changes past counts, so a report states the link snapshot it used.
- **Pseudonyms.** In the exploration copy, patient ids are replaced by a keyed hash with a key per project, held by the data owner, so two projects cannot join their copies (follows [privacy-preserving analysis](../privacy-analytics.md) rule 2). Dates of birth become year or month; free text is not exported by default.
- **Audit.** Every session, query and output request in either environment is logged with user, project, purpose and time. The live audit trail keeps search criteria names only, because a query names people ([audit](../audit/index.md)); the analytics query log keeps full query text, because without it an output cannot be checked, so the query log is itself restricted to the output checker and auditor (open question OQ-AN4). The copy job itself writes one row to the live `audit_events` per run, as an export (IO-5 treats `$everything` the same way).

## Freshness on every report

- Every report and every exported table shows "data as of" (the last replicated commit time or the export's `transactionTime` [BULK2]), the metric id and version, and the threshold applied.
- A number older than its metric's `max_age` is shown as stale with its age, never silently.
- When freshness cannot be established (no heartbeat, slot invalidated), the report shows no number: null, "freshness unknown".
- A heartbeat row written on the primary at a fixed interval and replicated gives a measured freshness even when no clinical change happens; `pg_stat_subscription` gives the receiver side [PGSTAT]. dbt-style freshness checks warn and then fail on stale sources (snippet) [DBTF].
- Operational reports are not live boards. A bed state needed for a decision now belongs in the clinical application, not in a replicated report.

## Ad hoc exploration tooling and its governance

- **Notebooks** through a multi-user notebook server (JupyterHub, with an authenticator against the site's identity provider, snippet) [JHUB], and **a SQL workbench** over DuckDB and Parquet [DUCKPG]. No tool connects to the primary.
- **Develop on synthetic data, run on real data.** OpenSAFELY's pattern: researchers develop against dummy data, submit code that runs in the secure zone, and results leave only after output checkers confirm they are not disclosive and match the approval; permanent logs record the code run and the outputs released [OSAFE]. This project already keeps synthetic patients for tests and training, which suits the first half.
- **An airlock.** Outputs (tables, charts, model coefficients) are requested, checked by a second person against the disclosure rules, and released or returned with a reason. No row-level file leaves.
- **Code is kept.** Every released output is tied to the commit of the code that produced it and to the metric and ViewDefinition versions.
- **No local downloads** of row data; the environment has no outbound network except package mirrors the deployer approves [uncertain: not researched in depth].

## Never writing back

- The replication role on the primary has `REPLICATION` and no write grant; the exploration environment receives files, not a connection.
- If DuckDB ever attaches a Postgres database, it is the reporting database, attached `READ_ONLY` [DUCKPG].
- No analytics output (a score, a cohort, a flag) enters the clinical record or the clinical screens through any path except a governed clinical decision support change with its own hazard assessment; NFR-12 applies to anything that does ([NFR-12](../07-non-functional-requirements.md)).
- The reporting database's tables are not a source of truth: a correction is made in the clinical record and flows down.

## Rules

1. No analyst, report or notebook connects to the primary database or to a physical standby.
2. The reporting database is fed only by a publication that publishes `INSERT` and names its tables explicitly; a test fails if the publication is `FOR ALL TABLES` or publishes `UPDATE`, `DELETE` or `TRUNCATE`.
3. The primary sets `max_slot_wal_keep_size` and `idle_replication_slot_timeout` to non-default values whenever an analytics slot exists; a deployment check fails otherwise.
4. Every metric has an id, a version, an owner, a plain-English definition, a time basis, a minimum cell size, a maximum age and, for speed or volume metrics, a balancing metric; the report runner refuses a metric missing any of these.
5. A changed definition is a new metric version; a report never shows two versions joined in one series without marking the break.
6. A count from 1 to the minimum cell size minus one is shown as withheld, a true zero as 0, and absent data as null with a reason; a test covers all three.
7. A total or rate that would let a withheld cell be recalculated is itself withheld.
8. Every report and exported table states its data-as-of time, metric version and disclosure threshold; a report with unknown freshness shows no number.
9. No analytics output names a patient or a member of staff; a list of named patients for action is a worklist in the clinical application.
10. Every analytics session carries a project or report id and a purpose-of-use code; a session without them is refused and the refusal logged.
11. No analyst role is a table owner, a superuser or `BYPASSRLS`, and every mart has row security forced on.
12. Rows with a restricted confidentiality or a sensitive-category label never reach the exploration environment unless the project approval names that category; a test seeds a labelled row and asserts it is absent.
13. A patient's recorded objection to secondary use is applied when the copy is made; a test seeds an objection and asserts the patient is absent from the copy.
14. Exploration copies use a per-project keyed pseudonym; two projects' copies of the same patient do not share an id.
15. No output leaves the exploration environment without a second person's release, and each release records the code commit and definition versions.
16. Nothing computed in either environment is written to the clinical store.
17. Each copy run writes one audit row in the live trail naming the job, purpose and row counts.

## Hazards to add when built

Described only; not added to the hazard log. For the Clinical Safety Officer to assess; none is accepted.

- **AN-H1** A stale report is read as current (for example, a backlog or occupancy figure hours old used to decide where a patient goes).
- **AN-H2** A withheld small count is read as zero, or a null "no data" is read as "no problem" (for example, no overdue results because results are not recorded).
- **AN-H3** The analytics replication slot fills the primary's disk and the clinical record goes offline.
- **AN-H4** A metric's definition changes silently and a trend is read as a real change in care.
- **AN-H5** An analytics-derived score or cohort reaches a clinical screen without clinical decision support governance, and changes what a patient is offered (NFR-12).
- **AN-H6** Consent objections or sensitivity labels are lost in the copy, and a sensitive fact reaches an analyst or a published output.
- **AN-H7** Re-identification from a small cell, from differencing across reports, or from joining two projects' copies.
- **AN-H8** Merged or linked patients are double-counted, or a later link silently changes a published figure.
- **AN-H9** Event time and record time are confused, so a wait looks shorter or longer than it was.
- **AN-H10** A bed census rule (for example, the midnight count) is applied in the wrong time zone or across a clock change.

## First slice

On synthetic data only, and as a design and test, not a deployment:

1. A second PostgreSQL 18 in the test compose file, subscribed to an insert-only publication of `referral_versions`, `referral_acknowledgements`, `result_versions` and `result_signoffs`.
2. One metric registry file with two metrics: referral acknowledgement time and unsigned results by age band, each with owner, version, minimum cell size, maximum age and a balancing metric.
3. A runner that produces both as a table with "data as of", metric version, withheld cells and null-with-reason, and a test for each of rules 2, 4, 6 and 8.
4. Slot limits set in the test configuration, and a test that stops the subscriber and shows the primary stays up.

The exploration environment, the Bulk Data export (IO-4) and the ViewDefinitions (IO-7) come after, because the service serves only three FHIR resource types today.

## What this repo has and lacks

| Need | Has | Lacks |
|---|---|---|
| A reproducible change history | Append-only version tables and `*_current` views | Event times on most tables |
| An insert-only feed | All clinical tables and the audit trail are insert-only under triggers | No publication, no subscriber, no slot limits |
| Gap detection in the copy | `prev_digest` and `digest` on audit rows | No checker on a copy |
| Operational sources | Referrals, acknowledgements, results, sign-offs, feed refusals, unmatched outcomes, break-the-glass, audit | Encounters, beds, stays, tasks, alerts, pathways |
| Purpose of use | A column on `audit_events`; the policy engine decides on purpose | Analyst purposes, project approvals |
| Labels and consent | Label codes and masking rules in the policy engine; identity consent | Labels on clinical records (SE-4); a secondary-use consent kind |
| FHIR export | Patient, AllergyIntolerance and AuditEvent served; audit export as CSV or NDJSON | `$export` (IO-4), ViewDefinitions (IO-7) |
| Disclosure rules | Named in XP-9, XP-14, PM-2 | A threshold, a runner, a test |
| Metric definitions | Measures listed in productivity measurement | A registry, owners, versions |
| Synthetic data | Synthetic patients for tests and training | Synthetic referrals and results at volume |

## Not verified

- The SQL-on-FHIR status is unclear: the 2.0.0 page says "release" and also "Draft as of 2024-10-09"; the 3.0.0 page is a ballot. Which to pin is a judgement, not a finding.
- The Bulk Data guide page at the unversioned address showed only a banner for v3.0.0; the detail above is from the v2.0.0 (STU2) page. What changed in v3.0.0 was not read.
- Whether a publication row filter may use JSONB operators (to filter on a label inside `body`) was not tested; the manual forbids user-defined functions and non-immutable built-ins [PGROW] [uncertain].
- Debezium, ClickHouse, Pathling, MetricFlow, dbt freshness, JupyterHub, the CMS policy, the disclosure-control method of England's hospital statistics, the Five Safes, the SACRO guide, the PurposeOfUse codes, the bed return and the referral-to-treatment rules were seen as search snippets only.
- The MetricFlow licence change rests on dbt Labs' blog and secondary pages; the licence file was not read.
- No volume estimate was made, so the claim that ClickHouse is not needed is unmeasured.
- The OMOP page names v5.5 as current; the HL7 FHIR-to-OMOP guide targets v5.4. The difference was not read.
- No BI or dashboard tool was compared.
- Nothing here has been checked against any country's law on secondary use, opt-outs or research approval.

## Sources

- [SOF2] SQL on FHIR 2.0.0: https://sql-on-fhir.org/ig/2.0.0/
- [SOF3] SQL on FHIR 3.0.0-ballot (HL7): https://build.fhir.org/ig/HL7/sql-on-fhir/en/index.html
- [BULK2] FHIR Bulk Data Access 2.0.0, export: https://hl7.org/fhir/uv/bulkdata/STU2/export.html
- [BULK3] FHIR Bulk Data Access, current (banner only): https://hl7.org/fhir/uv/bulkdata/export.html
- [OMOP] OHDSI OMOP Common Data Model: https://ohdsi.github.io/CommonDataModel/
- [OMOP54] OMOP CDM v5.4 tables: https://ohdsi.github.io/CommonDataModel/cdm54.html
- [F2OMOP] HL7 FHIR to OMOP IG (snippet): https://hl7.org/fhir/uv/omop/INFORMATIVE1/en/
- [PGHS] PostgreSQL 18, hot standby: https://www.postgresql.org/docs/18/hot-standby.html
- [PGROW] PostgreSQL 18, logical replication row filters: https://www.postgresql.org/docs/18/logical-replication-row-filter.html
- [PGCOL] PostgreSQL 18, logical replication column lists: https://www.postgresql.org/docs/18/logical-replication-col-lists.html
- [PGLD] PostgreSQL 18, logical decoding concepts and slots: https://www.postgresql.org/docs/18/logicaldecoding-explanation.html
- [PGREPL] PostgreSQL 18, replication settings: https://www.postgresql.org/docs/18/runtime-config-replication.html
- [PGSTAT] PostgreSQL 18, statistics views: https://www.postgresql.org/docs/18/monitoring-stats.html
- [PGRLS] PostgreSQL 18, row security policies: https://www.postgresql.org/docs/18/ddl-rowsecurity.html
- [DBZ] Debezium PostgreSQL connector (snippet): https://debezium.io/documentation/reference/2.0/connectors/postgresql.html
- [DUCKPG] DuckDB Postgres extension: https://duckdb.org/docs/2.0/core_extensions/postgres/overview.html
- [CHMPG] ClickHouse MaterializedPostgreSQL (snippet): https://clickhouse.com/docs/engines/database-engines/materialized-postgresql
- [PATH] Pathling (snippet): https://pathling.csiro.au/docs
- [MF] dbt, about MetricFlow (snippet): https://docs.getdbt.com/docs/build/about-metricflow
- [MFLIC] dbt Labs, open-sourcing MetricFlow (snippet): https://www.getdbt.com/blog/open-source-metricflow-governed-metrics and https://pypi.org/p/metricflow
- [DBTF] dbt, source freshness (snippet): https://docs.getdbt.com/reference/resource-properties/freshness
- [CMS] US Department of Health and Human Services, CMS cell suppression policy (snippet): https://www.hhs.gov/guidance/document/cms-cell-suppression-policy
- [HESDC] Disclosure control method for England's hospital episode and emergency care statistics (snippet): https://digital.nhs.uk/data-and-information/data-tools-and-services/data-services/hospital-episode-statistics/disclosure-control-methodology-for-hospital-episode-statistics-and-emergency-care-data-set
- [RITCHIE] Ritchie, "10 is the safest number that there's ever been" (snippet): https://uwe-repository.worktribe.com/OutputFile/9853171
- [SACRO] The SACRO guide to statistical output checking (snippet): https://zenodo.org/records/10054629
- [FIVE] UK Data Service, the Five Safes (snippet): https://ukdataservice.ac.uk/help/secure-lab/what-is-the-five-safes-framework/
- [OSAFE] OpenSAFELY, how it works: https://docs.opensafely.org/how-opensafely-works
- [POU] HL7 PurposeOfUse value set (snippet): https://terminology.hl7.org/7.0.0/ValueSet-v3-PurposeOfUse.html
- [KH03] England's bed availability and occupancy return, guidance (snippet): https://www.england.nhs.uk/statistics/wp-content/uploads/sites/2/2021/06/KH03-Guidance-June-2021.pdf
- [RTT] England's referral-to-treatment recording and reporting guidance v5.0 (snippet): https://www.england.nhs.uk/statistics/wp-content/uploads/sites/2/2025/02/Recording-and-reporting-RTT-waiting-times-guidance-v5.0-Feb25.pdf
- [JHUB] JupyterHub, authenticators (snippet): https://jupyterhub.readthedocs.io/en/5.2.1/reference/authenticators.html
- This repository: [privacy-preserving analysis](../privacy-analytics.md), [productivity measurement](../productivity-measurement/index.md), [persistence](../10-persistence.md), [schema map](../schema-map.md), [audit](../audit/index.md), [access control](../access-control/index.md), [protected information](../access-control/protected-information.md), [NFR](../07-non-functional-requirements.md), [hazard log](../../governance/clinical-safety/hazard-log.csv)
