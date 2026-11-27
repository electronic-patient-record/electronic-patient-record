-- Migration: a refusal of disclosure that the organisation declined (DR-10, spec/patient-refusal-of-disclosure-of-records):
-- who declined it, on what ground, when and how the patient was told, and how to complain. Possible only where the
-- tenant's pack allows it (never under `strength = binds`, the default), which the service checks before it writes.
-- Append-only: the only later version marks one entered in error, with who and why. Triggers refuse `UPDATE`, `DELETE` and
-- `TRUNCATE`. It decides nothing yet: the refusal still applies (DR-10 records and tells; what declining changes is open).

CREATE TABLE declined_refusal_versions (
                decline_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                consent_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
                author TEXT NOT NULL,
                reason TEXT,
                state TEXT NOT NULL CHECK (state IN ('recorded', 'entered-in-error')),
                ground TEXT NOT NULL CHECK (ground IN ('law', 'risk-to-another', 'cannot-be-done-safely')),
                pack_version TEXT NOT NULL CHECK (length(pack_version) > 0),
                body JSONB NOT NULL,
                PRIMARY KEY (decline_id, version),
                CONSTRAINT declined_refusal_versions_reason CHECK ((reason IS NOT NULL) = (state = 'entered-in-error'))
            );

CREATE INDEX declined_refusal_versions_patient ON declined_refusal_versions (patient_id);

CREATE VIEW declined_refusal_current AS
             SELECT DISTINCT ON (decline_id) decline_id, version, patient_id, consent_id, recorded_at, author, reason, state,
                    ground, pack_version, body
             FROM declined_refusal_versions ORDER BY decline_id, version DESC;

CREATE FUNCTION declined_refusal_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'declined_refusal_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER declined_refusal_versions_no_change BEFORE UPDATE OR DELETE ON declined_refusal_versions
             FOR EACH ROW EXECUTE FUNCTION declined_refusal_versions_immutable();

CREATE TRIGGER declined_refusal_versions_no_truncate BEFORE TRUNCATE ON declined_refusal_versions
             FOR EACH STATEMENT EXECUTE FUNCTION declined_refusal_versions_immutable();
