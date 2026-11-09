-- Migration: watch lists (WL-6, hazard H-055). A clinician's entry for a patient, with a required reason and an
-- optional last day, as a series of versions: version 1 is the entry as added (`active`), version 2 its ending by its
-- owner with a reason (`ended`). An entry whose last day has passed is read as lapsed; nothing is written for that.
-- Append-only: triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`, so an ended entry stays in the history. `owner` is
-- the clinician whose list it is; `patient_id` and `body` (the reason, the last day, the reason for ending) are
-- patient data.

CREATE TABLE watch_entry_versions (
                entry_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                owner TEXT NOT NULL,
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('active', 'ended')),
                body JSONB NOT NULL,
                PRIMARY KEY (entry_id, version),
                CHECK ((version = 1) = (status = 'active'))
            );

CREATE INDEX watch_entry_versions_owner_idx ON watch_entry_versions (owner, recorded_at);
CREATE INDEX watch_entry_versions_patient_idx ON watch_entry_versions (patient_id, recorded_at);

CREATE VIEW watch_entry_current AS
             SELECT v.entry_id, v.version, v.owner, v.patient_id, v.recorded_at, v.author, v.status, v.body
             FROM watch_entry_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM watch_entry_versions n
                 WHERE n.entry_id = v.entry_id AND n.version > v.version
             );

CREATE FUNCTION watch_entry_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'watch_entry_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER watch_entry_versions_no_change BEFORE UPDATE OR DELETE ON watch_entry_versions
             FOR EACH ROW EXECUTE FUNCTION watch_entry_versions_immutable();

CREATE TRIGGER watch_entry_versions_no_truncate BEFORE TRUNCATE ON watch_entry_versions
             FOR EACH STATEMENT EXECUTE FUNCTION watch_entry_versions_immutable();
