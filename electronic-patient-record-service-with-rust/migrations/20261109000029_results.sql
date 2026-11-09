-- Migration: results and their sign-off (tasks OR-4 first slice and WL-1).
-- `result_versions` is append-only: a result is a series of versions (preliminary, final, amended, corrected, entered in
-- error), none edited or removed. `result_signoffs` is append-only too: a sign-off names the result **and the version**
-- it was made on, so a result changed after sign-off reads as changed, not as signed. Triggers refuse `UPDATE`,
-- `DELETE` and `TRUNCATE` on both.

CREATE TABLE result_versions (
                result_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('preliminary', 'final', 'amended', 'corrected', 'entered-in-error')),
                reason TEXT,
                body JSONB NOT NULL,
                PRIMARY KEY (result_id, version)
            );

CREATE INDEX result_versions_patient_idx
             ON result_versions (patient_id, recorded_at);

CREATE VIEW result_current AS
             SELECT v.result_id, v.version, v.patient_id, v.recorded_at, v.author, v.status, v.reason, v.body
             FROM result_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM result_versions n
                 WHERE n.result_id = v.result_id AND n.version > v.version
             );

CREATE TABLE result_signoffs (
                signoff_id UUID NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
                result_id UUID NOT NULL,
                result_version INTEGER NOT NULL CHECK (result_version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                signed_by TEXT NOT NULL,
                UNIQUE (result_id, result_version),
                FOREIGN KEY (result_id, result_version) REFERENCES result_versions (result_id, version)
            );

CREATE FUNCTION results_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION '% is append-only', TG_TABLE_NAME;
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER result_versions_no_change
             BEFORE UPDATE OR DELETE ON result_versions
             FOR EACH ROW EXECUTE FUNCTION results_immutable();

CREATE TRIGGER result_versions_no_truncate
             BEFORE TRUNCATE ON result_versions
             FOR EACH STATEMENT EXECUTE FUNCTION results_immutable();

CREATE TRIGGER result_signoffs_no_change
             BEFORE UPDATE OR DELETE ON result_signoffs
             FOR EACH ROW EXECUTE FUNCTION results_immutable();

CREATE TRIGGER result_signoffs_no_truncate
             BEFORE TRUNCATE ON result_signoffs
             FOR EACH STATEMENT EXECUTE FUNCTION results_immutable();
