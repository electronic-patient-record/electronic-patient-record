## 10. Persistence

PostgreSQL 18. **Built**: `audit_events` (append-only; triggers refuse UPDATE/DELETE/TRUNCATE; action in C/R/U/D/E;
outcome in 0/4/8/12; `prev_digest`/`digest` reserved for the hash chain).

**Planned**: `resource_version` (type, id, version_id, tenant_id, patient_id, recorded_at, author_ref, provenance_id,
status, `data` JSONB, security labels) and `resource_current`; typed search index tables (token, string, date,
reference, quantity, URI); `outbox`; per-tenant data-encryption keys; row-level security. No SQL path deletes a
version. Retention follows the UK Records Management Code of Practice 2023 and US state ranges ([12](12-compliance.md));
legal hold and defensible destruction are deployer workflows.

Migrations run with `db migrate`; production sets `auto_migrate: false` and runs the `migrate` compose service.
