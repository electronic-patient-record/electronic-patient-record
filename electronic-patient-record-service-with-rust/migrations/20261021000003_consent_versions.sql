-- Migration: the append-only `consent_versions` table and the `consent_current` view (task SE-2).
-- 
-- A consent is never edited or removed: withdrawing it, correcting it or marking it entered in
-- error is a new version with its author, time and reason. Triggers refuse `UPDATE`, `DELETE` and
-- `TRUNCATE`. `patient_id` is carried on every version so a patient's consents can be found in
-- one indexed read; it is not a foreign key because `patient_versions` has no unique patient row.

CREATE TABLE consent_versions (
                consent_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT,
                status TEXT NOT NULL CHECK (status IN ('active', 'inactive', 'entered-in-error')),
                body JSONB NOT NULL,
                PRIMARY KEY (consent_id, version)
            );

CREATE INDEX consent_versions_patient ON consent_versions (patient_id);

CREATE VIEW consent_current AS
             SELECT DISTINCT ON (consent_id) consent_id, version, patient_id, recorded_at, author,
                    reason, status, body
             FROM consent_versions ORDER BY consent_id, version DESC;

CREATE FUNCTION consent_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'consent_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER consent_versions_no_change BEFORE UPDATE OR DELETE ON consent_versions
             FOR EACH ROW EXECUTE FUNCTION consent_versions_immutable();

CREATE TRIGGER consent_versions_no_truncate BEFORE TRUNCATE ON consent_versions
             FOR EACH STATEMENT EXECUTE FUNCTION consent_versions_immutable();
