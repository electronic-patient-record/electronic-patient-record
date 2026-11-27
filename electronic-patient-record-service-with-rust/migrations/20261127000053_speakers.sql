-- Migration: who speaks for a person who cannot, for a decision (RF-9, spec/patient-refusal-of-consent): an attorney, a
-- guardian or deputy, a court, each parent apart, a relative with no power, or the record that nobody does. Append-only
-- versions: a view that changes, or paper seen later, is a new version; entered in error is a state that ends changes.
-- Triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. Nothing decides who wins.

CREATE TABLE speaker_versions (
                speaker_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
                author TEXT NOT NULL,
                reason TEXT,
                state TEXT NOT NULL CHECK (state IN ('stands', 'entered-in-error')),
                role TEXT NOT NULL CHECK (role IN ('attorney', 'guardian', 'court', 'parent', 'relative', 'nobody')),
                body JSONB NOT NULL,
                PRIMARY KEY (speaker_id, version),
                CONSTRAINT speaker_versions_reason CHECK (state = 'stands' OR reason IS NOT NULL)
            );

CREATE INDEX speaker_versions_patient ON speaker_versions (patient_id);

CREATE VIEW speaker_current AS
             SELECT DISTINCT ON (speaker_id) speaker_id, version, patient_id, recorded_at, author, reason, state, role, body
             FROM speaker_versions ORDER BY speaker_id, version DESC;

CREATE FUNCTION speaker_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'speaker_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER speaker_versions_no_change BEFORE UPDATE OR DELETE ON speaker_versions
             FOR EACH ROW EXECUTE FUNCTION speaker_versions_immutable();

CREATE TRIGGER speaker_versions_no_truncate BEFORE TRUNCATE ON speaker_versions
             FOR EACH STATEMENT EXECUTE FUNCTION speaker_versions_immutable();
