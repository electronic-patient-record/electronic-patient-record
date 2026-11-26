-- Migration: judgements of whether a person can make one decision, now (RF-8, spec/patient-refusal-of-consent).
-- Append-only: a judgement is never changed, and a new view is a new judgement. The only later version marks one entered
-- in error, with who and why. Triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. `decides` is copied out of `body` for the
-- stored-word check; nothing in the service decides on it. No column ranks a person by a number, on purpose.

CREATE TABLE decision_ability_versions (
                judgement_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
                author TEXT NOT NULL,
                reason TEXT,
                state TEXT NOT NULL CHECK (state IN ('recorded', 'entered-in-error')),
                decides TEXT NOT NULL CHECK (decides IN ('can', 'cannot', 'not-known')),
                body JSONB NOT NULL,
                PRIMARY KEY (judgement_id, version),
                CONSTRAINT decision_ability_versions_reason CHECK ((reason IS NOT NULL) = (state = 'entered-in-error'))
            );

CREATE INDEX decision_ability_versions_patient ON decision_ability_versions (patient_id);

CREATE VIEW decision_ability_current AS
             SELECT DISTINCT ON (judgement_id) judgement_id, version, patient_id, recorded_at, author, reason, state,
                    decides, body
             FROM decision_ability_versions ORDER BY judgement_id, version DESC;

CREATE FUNCTION decision_ability_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'decision_ability_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER decision_ability_versions_no_change BEFORE UPDATE OR DELETE ON decision_ability_versions
             FOR EACH ROW EXECUTE FUNCTION decision_ability_versions_immutable();

CREATE TRIGGER decision_ability_versions_no_truncate BEFORE TRUNCATE ON decision_ability_versions
             FOR EACH STATEMENT EXECUTE FUNCTION decision_ability_versions_immutable();
