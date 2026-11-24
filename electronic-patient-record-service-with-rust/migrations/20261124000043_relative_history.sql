-- Migration: `relative_history_versions` replaces the table migration 42 made for EH-3, whose name and whose record were
-- both changed within the hour: the relative's genetic, gestational and shared-environment links are asked apart, and the
-- word the SG-2 guard refuses is gone. An entry of the old shape cannot be read as the new one, and no clinical row is
-- ever removed, so this refuses to run while there is one. Nothing was released with the old table.

LOCK TABLE family_history_versions IN ACCESS EXCLUSIVE MODE;

DO $$
             BEGIN
               IF EXISTS (SELECT 1 FROM family_history_versions) THEN
                 RAISE EXCEPTION 'family_history_versions holds entries: they cannot be read as relative history, and are not removed';
               END IF;
             END;
             $$;

DROP VIEW family_history_current;

DROP TABLE family_history_versions;

ALTER FUNCTION problem_family_history_immutable() RENAME TO problem_relative_history_immutable;

CREATE TABLE relative_history_versions (
                entry_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
                author TEXT NOT NULL,
                reason TEXT,
                status TEXT NOT NULL CHECK (status IN ('recorded', 'entered-in-error')),
                body JSONB NOT NULL,
                PRIMARY KEY (entry_id, version)
            );

CREATE INDEX relative_history_versions_patient_idx ON relative_history_versions (patient_id, recorded_at);

CREATE VIEW relative_history_current AS
             SELECT v.entry_id, v.version, v.patient_id, v.recorded_at, v.author, v.reason, v.status, v.body
             FROM relative_history_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM relative_history_versions n WHERE n.entry_id = v.entry_id AND n.version > v.version
             );

CREATE TRIGGER relative_history_versions_no_change BEFORE UPDATE OR DELETE ON relative_history_versions
             FOR EACH ROW EXECUTE FUNCTION problem_relative_history_immutable();

CREATE TRIGGER relative_history_versions_no_truncate BEFORE TRUNCATE ON relative_history_versions
             FOR EACH STATEMENT EXECUTE FUNCTION problem_relative_history_immutable();
