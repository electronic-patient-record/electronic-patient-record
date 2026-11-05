-- Migration: the append-only `adjustment_versions` table and its `adjustment_current` view (task PA-6).
-- A reasonable adjustment or communication need is a series of versions; none is edited or removed, and
-- triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. Ending one is a new version, so what a person needed
-- stays in the history.

CREATE TABLE adjustment_versions (
                adjustment_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('active', 'ended')),
                body JSONB NOT NULL,
                PRIMARY KEY (adjustment_id, version)
            );

CREATE INDEX adjustment_versions_patient_idx
             ON adjustment_versions (patient_id, recorded_at);

CREATE VIEW adjustment_current AS
             SELECT v.adjustment_id, v.version, v.patient_id, v.recorded_at, v.author, v.status, v.body
             FROM adjustment_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM adjustment_versions n
                 WHERE n.adjustment_id = v.adjustment_id AND n.version > v.version
             );

CREATE FUNCTION adjustment_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'adjustment_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER adjustment_versions_no_change
             BEFORE UPDATE OR DELETE ON adjustment_versions
             FOR EACH ROW EXECUTE FUNCTION adjustment_versions_immutable();

CREATE TRIGGER adjustment_versions_no_truncate
             BEFORE TRUNCATE ON adjustment_versions
             FOR EACH STATEMENT EXECUTE FUNCTION adjustment_versions_immutable();
