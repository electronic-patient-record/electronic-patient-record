## 10. Persistence

PostgreSQL 18. **Built**: `audit_events` (append-only; triggers refuse UPDATE/DELETE/TRUNCATE; action in C/R/U/D/E;
outcome in 0/4/8/12; `prev_digest`/`digest` reserved for the hash chain).

**Planned**: `resource_version` (type, id, version_id, tenant_id, patient_id, recorded_at, author_ref, provenance_id,
status, `data` JSONB, security labels) and `resource_current`; typed search index tables (token, string, date,
reference, quantity, URI); `outbox`; per-tenant data-encryption keys; row-level security. No SQL path deletes a
version. Retention follows the UK Records Management Code of Practice 2023 and US state ranges ([12](12-compliance.md));
legal hold and defensible destruction are deployer workflows.

**Current views.** Each record is a run of append-only versions, and a `*_current` view gives the latest. When a
query filters the view on a column other than the record's own key (a patient's consents, allergies, relationships,
links), the view is an anti-join: a version with no later version of the same record. The filter then reaches the
versions table and its index. `DISTINCT ON (key)` hides such a filter from the index and builds every record's current
version first, so it is used only for views keyed by what they are filtered on (`patient_photo_current`,
`allergy_assertion_current`). `patient_current` is an anti-join too, because search filters it on the record body.
`patient_versions` has a GIN index on `body->'identifiers'` (`jsonb_path_ops`, for `@>`) and an expression index on
`body->>'birth_date'`. Family-name search and the duplicate check's family arm have no index yet (RC-2). Ids are
bound as `uuid`, never compared as text (`tests/sql_census.rs`).

**Appending a version.** A new version is an insert of version N+1 in a savepoint (`store::append_version`). When
two writers pass the version check at once, the primary key refuses the second. That refusal is read from the
driver's error kind (a unique violation; the version tables have no other unique constraint), never from the
message text, and is answered as a version conflict naming the version now current.

**No delete path in `down`.** A `down` that drops a clinical table (patient, consent, relationship, allergy,
photo, link, feed, break-glass, audit) fails before it changes anything, unless `LOCO_ENV` is `development` or
`test` **and** `EPR_ALLOW_DESTRUCTIVE_DOWN=true`. An unset `LOCO_ENV` counts as production. Index and view
migrations (5, 11, 12) can still be reversed.

Migrations run with `db migrate`; production sets `auto_migrate: false` and runs the `migrate` compose service.
