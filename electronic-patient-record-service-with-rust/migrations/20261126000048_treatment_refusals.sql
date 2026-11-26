-- Migration: refusals of treatment, append-only (RF-5, spec/patient-refusal-of-consent).
-- A refusal is never edited or removed: a doubt, a withdrawal that was heard, a correction or "entered in error" is a new
-- version with its author, time and reason. Triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. `patient_id` is on every
-- version so one indexed read finds a patient's refusals; `state`, `kind` and `identity` are copied out of `body` so that a
-- merge or an unmerge can ask about each refusal without reading the words. This is not `consent_versions`, which holds a
-- refusal of disclosure (RF-2).

CREATE TABLE treatment_refusal_versions (
                refusal_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
                author TEXT NOT NULL,
                reason TEXT,
                state TEXT NOT NULL CHECK (state IN ('stands', 'doubted', 'withdrawn', 'entered-in-error')),
                kind TEXT NOT NULL CHECK (kind IN ('spoken-now', 'advance', 'order', 'representative', 'parent', 'carried', 'reported')),
                identity TEXT NOT NULL CHECK (identity IN ('certain', 'uncertain')),
                body JSONB NOT NULL,
                PRIMARY KEY (refusal_id, version)
            );

CREATE INDEX treatment_refusal_versions_patient ON treatment_refusal_versions (patient_id);

CREATE VIEW treatment_refusal_current AS
             SELECT DISTINCT ON (refusal_id) refusal_id, version, patient_id, recorded_at, author, reason, state, kind,
                    identity, body
             FROM treatment_refusal_versions ORDER BY refusal_id, version DESC;

CREATE FUNCTION treatment_refusal_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'treatment_refusal_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER treatment_refusal_versions_no_change BEFORE UPDATE OR DELETE ON treatment_refusal_versions
             FOR EACH ROW EXECUTE FUNCTION treatment_refusal_versions_immutable();

CREATE TRIGGER treatment_refusal_versions_no_truncate BEFORE TRUNCATE ON treatment_refusal_versions
             FOR EACH STATEMENT EXECUTE FUNCTION treatment_refusal_versions_immutable();
