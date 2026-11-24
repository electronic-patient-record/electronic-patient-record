-- Migration: the problem list (EH-1) and family history (EH-3). Both are append-only versions with a `*_current` view
-- (an anti-join, as CQ-9 found); triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. `recorded_at` is the clock when the
-- row is written, not when its transaction began, so a later version never carries an earlier time and a read as at a
-- time (EH-2) cannot show the versions out of order.

CREATE TABLE problem_versions (
                problem_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
                author TEXT NOT NULL,
                reason TEXT,
                status TEXT NOT NULL CHECK (status IN ('active', 'remission', 'inactive', 'resolved', 'entered-in-error')),
                body JSONB NOT NULL,
                PRIMARY KEY (problem_id, version)
            );

CREATE INDEX problem_versions_patient_idx ON problem_versions (patient_id, recorded_at);

CREATE VIEW problem_current AS
             SELECT v.problem_id, v.version, v.patient_id, v.recorded_at, v.author, v.reason, v.status, v.body
             FROM problem_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM problem_versions n WHERE n.problem_id = v.problem_id AND n.version > v.version
             );

CREATE TABLE family_history_versions (
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

CREATE INDEX family_history_versions_patient_idx ON family_history_versions (patient_id, recorded_at);

CREATE VIEW family_history_current AS
             SELECT v.entry_id, v.version, v.patient_id, v.recorded_at, v.author, v.reason, v.status, v.body
             FROM family_history_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM family_history_versions n WHERE n.entry_id = v.entry_id AND n.version > v.version
             );

CREATE FUNCTION problem_family_history_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'problem and family history tables are append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER problem_versions_no_change BEFORE UPDATE OR DELETE ON problem_versions
             FOR EACH ROW EXECUTE FUNCTION problem_family_history_immutable();

CREATE TRIGGER problem_versions_no_truncate BEFORE TRUNCATE ON problem_versions
             FOR EACH STATEMENT EXECUTE FUNCTION problem_family_history_immutable();

CREATE TRIGGER family_history_versions_no_change BEFORE UPDATE OR DELETE ON family_history_versions
             FOR EACH ROW EXECUTE FUNCTION problem_family_history_immutable();

CREATE TRIGGER family_history_versions_no_truncate BEFORE TRUNCATE ON family_history_versions
             FOR EACH STATEMENT EXECUTE FUNCTION problem_family_history_immutable();
