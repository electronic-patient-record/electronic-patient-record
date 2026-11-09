-- Migration: completed assessments (WL-3). `assessment_versions` is append-only: each row keeps the form id
-- and form version it was made under (the forms are data in `src/assessment_forms.toml`), so a later form
-- version never rewrites what was asked or answered. Triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.

CREATE TABLE assessment_versions (
                assessment_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                form_id TEXT NOT NULL,
                form_version INTEGER NOT NULL CHECK (form_version >= 1),
                body JSONB NOT NULL,
                PRIMARY KEY (assessment_id, version)
            );

CREATE INDEX assessment_versions_patient_idx
             ON assessment_versions (patient_id, recorded_at);

CREATE VIEW assessment_current AS
             SELECT v.assessment_id, v.version, v.patient_id, v.recorded_at, v.author, v.form_id, v.form_version, v.body
             FROM assessment_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM assessment_versions n
                 WHERE n.assessment_id = v.assessment_id AND n.version > v.version
             );

CREATE FUNCTION assessment_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'assessment_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER assessment_versions_no_change
             BEFORE UPDATE OR DELETE ON assessment_versions
             FOR EACH ROW EXECUTE FUNCTION assessment_versions_immutable();

CREATE TRIGGER assessment_versions_no_truncate
             BEFORE TRUNCATE ON assessment_versions
             FOR EACH STATEMENT EXECUTE FUNCTION assessment_versions_immutable();
