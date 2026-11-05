# Schema map

Every table and view the service's migrations create. **Generated** by [`scripts/generate-schema-map.py`](../scripts/generate-schema-map.py) from the `CREATE TABLE` and `CREATE VIEW` statements in `electronic-patient-record-service-with-rust/migration/src/`; the docs job fails when this page is out of date (task OF-6). Do not edit it by hand.

Not listed: tables made by a statement built at run time (the monthly `audit_events` partitions), and what a column means (the migrations carry few comments, so only the name and SQL type are shown). The clinical tables are append-only: a change is a new row, and triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.

24 tables, 13 views.

## Tables

| Table | Created in | Columns |
|---|---|---|
| `allergy_assertion_versions` | `m20261021_000007_allergies` | `patient_id` UUID, `version` INTEGER, `recorded_at` TIMESTAMPTZ, `author` TEXT, `reason` TEXT, `kind` TEXT |
| `allergy_versions` | `m20261021_000007_allergies` | `allergy_id` UUID, `version` INTEGER, `patient_id` UUID, `recorded_at` TIMESTAMPTZ, `author` TEXT, `reason` TEXT, `status` TEXT, `body` JSONB |
| `amendment_versions` | `m20261027_000015_amendments` | `amendment_id` UUID, `version` INTEGER, `patient_id` UUID, `recorded_at` TIMESTAMPTZ, `author` TEXT, `state` TEXT, `target_kind` TEXT, `target_id` UUID, `event` JSONB, `body` JSONB |
| `audit_archive_anchors` | `m20261031_000018_audit_partitions` | `partition` TEXT, `first_id` BIGINT, `last_id` BIGINT, `row_count` BIGINT, `first_prev_digest` TEXT, `last_digest` TEXT, `archived_at` TIMESTAMPTZ |
| `audit_events` | `m20261020_000001_audit_events`, `m20261031_000018_audit_partitions` | `id` BIGSERIAL, `recorded_at` TIMESTAMPTZ, `action` CHAR(1), `outcome` SMALLINT, `agent` TEXT, `patient_ref` TEXT, `resource_ref` TEXT, `purpose_of_use` TEXT, `break_glass` BOOLEAN, `network_address` TEXT, `detail` JSONB, `prev_digest` TEXT, `digest` TEXT |
| `break_glass_grants` | `m20261021_000010_break_glass` | `grant_id` UUID, `patient_id` UUID, `subject` TEXT, `reason` TEXT, `granted_at` TIMESTAMPTZ, `expires_at` TIMESTAMPTZ |
| `break_glass_reviews` | `m20261021_000010_break_glass` | `grant_id` UUID, `reviewed_at` TIMESTAMPTZ, `reviewed_by` TEXT, `outcome` TEXT, `note` TEXT |
| `consent_versions` | `m20261021_000003_consent_versions` | `consent_id` UUID, `version` INTEGER, `patient_id` UUID, `recorded_at` TIMESTAMPTZ, `author` TEXT, `reason` TEXT, `status` TEXT, `body` JSONB |
| `death_assertion_versions` | `m20261105_000025_death_assertions` | `assertion_id` UUID, `version` INTEGER, `patient_id` UUID, `recorded_at` TIMESTAMPTZ, `author` TEXT, `status` TEXT, `body` JSONB |
| `feed_refusal_resolutions` | `m20261021_000009_feed_refusals` | `refusal_id` UUID, `resolved_at` TIMESTAMPTZ, `resolved_by` TEXT, `outcome` TEXT, `reason` TEXT |
| `feed_refusals` | `m20261021_000009_feed_refusals` | `refusal_id` UUID, `received_at` TIMESTAMPTZ, `agent` TEXT, `peer` TEXT, `event` TEXT, `control_id` TEXT, `kind` TEXT, `reasons` JSONB, `message` TEXT |
| `hazard_versions` | `m20261028_000016_hazard_events` | `hazard_id` UUID, `version` INTEGER, `recorded_at` TIMESTAMPTZ, `author` TEXT, `state` TEXT, `event` JSONB, `body` JSONB |
| `idempotency_keys` | `m20261102_000021_idempotency_keys` | `subject` TEXT, `key` TEXT, `method` TEXT, `path` TEXT, `body_digest` TEXT, `state` TEXT, `status` SMALLINT, `content_type` TEXT, `response` BYTEA, `created_at` TIMESTAMPTZ |
| `observation_versions` | `m20261026_000014_observations` | `observation_id` UUID, `version` INTEGER, `patient_id` UUID, `recorded_at` TIMESTAMPTZ, `author` TEXT, `reason` TEXT, `status` TEXT, `effective_at` TIMESTAMPTZ, `site_zone` TEXT, `body` JSONB |
| `outbox_attempts` | `m20261030_000017_outbox` | `id` BIGSERIAL, `event_id` UUID, `attempted_at` TIMESTAMPTZ, `outcome` TEXT, `reason` TEXT |
| `outbox_events` | `m20261030_000017_outbox` | `id` UUID, `created_at` TIMESTAMPTZ, `kind` TEXT, `recipient` TEXT, `recipient_roles` TEXT[], `purpose` TEXT, `resource` TEXT, `patient_id` UUID, `payload` JSONB |
| `patient_link_versions` | `m20261021_000008_patient_links` | `link_id` UUID, `version` INTEGER, `recorded_at` TIMESTAMPTZ, `author` TEXT, `reason` TEXT, `source_patient_id` UUID, `target_patient_id` UUID, `status` TEXT, `match_score` DOUBLE, `match_decision` TEXT |
| `patient_photos` | `m20261021_000006_patient_photos` | `patient_id` UUID, `version` INTEGER, `recorded_at` TIMESTAMPTZ, `author` TEXT, `reason` TEXT, `content_type` TEXT, `bytes` BYTEA, `sha256` TEXT |
| `patient_versions` | `m20261021_000002_patient_versions` | `patient_id` UUID, `version` INTEGER, `recorded_at` TIMESTAMPTZ, `author` TEXT, `reason` TEXT, `body` JSONB |
| `referral_acknowledgements` | `m20261103_000024_referral_acknowledgements` | `referral_id` UUID, `version` INTEGER, `acknowledged_at` TIMESTAMPTZ, `acknowledged_by` TEXT |
| `referral_unmatched` | `m20261103_000023_referral_unmatched` | `unmatched_id` UUID, `received_at` TIMESTAMPTZ, `agent` TEXT, `kind` TEXT, `referral_ref` TEXT, `event` TEXT, `reasons` JSONB, `note` TEXT, `message` TEXT |
| `referral_unmatched_clearances` | `m20261103_000023_referral_unmatched` | `unmatched_id` UUID, `cleared_at` TIMESTAMPTZ, `cleared_by` TEXT, `how` TEXT, `referral_id` UUID, `reason` TEXT |
| `referral_versions` | `m20261102_000022_referrals` | `referral_id` UUID, `version` INTEGER, `patient_id` UUID, `recorded_at` TIMESTAMPTZ, `author` TEXT, `state` TEXT, `event` JSONB, `body` JSONB |
| `relationship_versions` | `m20261021_000004_relationship_versions` | `relationship_id` UUID, `version` INTEGER, `patient_id` UUID, `subject` TEXT, `kind` TEXT, `status` TEXT, `recorded_at` TIMESTAMPTZ, `author` TEXT, `reason` TEXT, `body` JSONB |

## Views

| View | Created in |
|---|---|
| `allergy_assertion_current` | `m20261021_000007_allergies` |
| `allergy_current` | `m20261021_000007_allergies` |
| `amendment_current` | `m20261027_000015_amendments` |
| `audit_events_all` | `m20261031_000019_audit_archive_view` |
| `consent_current` | `m20261021_000003_consent_versions` |
| `death_assertion_current` | `m20261105_000025_death_assertions` |
| `hazard_current` | `m20261028_000016_hazard_events` |
| `observation_current` | `m20261026_000014_observations` |
| `patient_current` | `m20261021_000002_patient_versions`, `m20261021_000012_patient_search_indexes`, `m20261024_000013_patient_drill` |
| `patient_link_current` | `m20261021_000008_patient_links` |
| `patient_photo_current` | `m20261021_000006_patient_photos` |
| `referral_current` | `m20261102_000022_referrals` |
| `relationship_current` | `m20261021_000004_relationship_versions` |
