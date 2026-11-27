-- Migration: records of asking a person now, and of leaving against advice (RF-10, with EC-6; spec/patient-refusal-of-consent).
-- Append-only: a record is never changed, and a new conversation is a new record. The only later version marks one
-- entered in error, with who and why. Triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. `leaving_against_advice` is copied
-- out of `body` so a list of those who left can be read without the words.

CREATE TABLE asking_now_versions (
                record_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
                author TEXT NOT NULL,
                reason TEXT,
                state TEXT NOT NULL CHECK (state IN ('recorded', 'entered-in-error')),
                leaving_against_advice BOOLEAN NOT NULL,
                body JSONB NOT NULL,
                PRIMARY KEY (record_id, version),
                CONSTRAINT asking_now_versions_reason CHECK ((reason IS NOT NULL) = (state = 'entered-in-error'))
            );

CREATE INDEX asking_now_versions_patient ON asking_now_versions (patient_id);

CREATE VIEW asking_now_current AS
             SELECT DISTINCT ON (record_id) record_id, version, patient_id, recorded_at, author, reason, state,
                    leaving_against_advice, body
             FROM asking_now_versions ORDER BY record_id, version DESC;

CREATE FUNCTION asking_now_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'asking_now_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER asking_now_versions_no_change BEFORE UPDATE OR DELETE ON asking_now_versions
             FOR EACH ROW EXECUTE FUNCTION asking_now_versions_immutable();

CREATE TRIGGER asking_now_versions_no_truncate BEFORE TRUNCATE ON asking_now_versions
             FOR EACH STATEMENT EXECUTE FUNCTION asking_now_versions_immutable();
