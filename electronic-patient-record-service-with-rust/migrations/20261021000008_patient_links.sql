-- Migration: the append-only `patient_link_versions` table and `patient_link_current` view (merge
-- and unmerge, task ID-5). A merge is a link from the record merged away (`source`) to the one that
-- survives (`target`); unmerging is a new version of the same link with status `reversed`. No
-- patient or clinical row is ever rewritten, so a reversal restores exactly what was there.
-- Triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.

CREATE TABLE patient_link_versions (
                link_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                -- provenance: why, always
                reason TEXT NOT NULL CHECK (length(btrim(reason)) > 0),
                source_patient_id UUID NOT NULL,
                target_patient_id UUID NOT NULL CHECK (target_patient_id <> source_patient_id),
                status TEXT NOT NULL CHECK (status IN ('active', 'reversed')),
                match_score DOUBLE PRECISION,
                match_decision TEXT,
                PRIMARY KEY (link_id, version)
            );

CREATE INDEX patient_link_source_idx ON patient_link_versions (source_patient_id);

CREATE INDEX patient_link_target_idx ON patient_link_versions (target_patient_id);

CREATE VIEW patient_link_current AS
             SELECT DISTINCT ON (link_id) link_id, version, recorded_at, author, reason,
                    source_patient_id, target_patient_id, status, match_score, match_decision
             FROM patient_link_versions ORDER BY link_id, version DESC;

CREATE FUNCTION patient_link_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'patient_link_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER patient_link_versions_no_change BEFORE UPDATE OR DELETE ON patient_link_versions
             FOR EACH ROW EXECUTE FUNCTION patient_link_versions_immutable();

CREATE TRIGGER patient_link_versions_no_truncate BEFORE TRUNCATE ON patient_link_versions
             FOR EACH STATEMENT EXECUTE FUNCTION patient_link_versions_immutable();
