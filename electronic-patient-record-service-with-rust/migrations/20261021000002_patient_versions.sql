-- Migration: the append-only `patient_versions` table and the `patient_current` view (plan ID-3).
-- 
-- A patient record is never updated in place: every change is a new version row with its
-- author, time and reason. Triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. There is no
-- delete path; a wrongly created record is withdrawn by a later version, not removed.

CREATE TABLE patient_versions (
                patient_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT,
                body JSONB NOT NULL,
                PRIMARY KEY (patient_id, version)
            );

CREATE VIEW patient_current AS
             SELECT DISTINCT ON (patient_id) patient_id, version, recorded_at, author, reason, body
             FROM patient_versions ORDER BY patient_id, version DESC;

CREATE FUNCTION patient_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'patient_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER patient_versions_no_change BEFORE UPDATE OR DELETE ON patient_versions
             FOR EACH ROW EXECUTE FUNCTION patient_versions_immutable();

CREATE TRIGGER patient_versions_no_truncate BEFORE TRUNCATE ON patient_versions
             FOR EACH STATEMENT EXECUTE FUNCTION patient_versions_immutable();
