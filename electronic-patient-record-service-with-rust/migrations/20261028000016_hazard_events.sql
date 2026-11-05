-- Migration: the append-only `hazard_versions` table and its `hazard_current` view (task AS-17). A
-- hazard is a series of versions (raised, a control added, retired, reopened); none is edited or
-- removed, and triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`, so retiring a hazard cannot erase
-- its history. `event` is what happened in this version and `body` the whole hazard after it. The
-- table holds no patient data.

CREATE TABLE hazard_versions (
                hazard_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                state TEXT NOT NULL CHECK (state IN ('open', 'retired')),
                event JSONB NOT NULL,
                body JSONB NOT NULL,
                PRIMARY KEY (hazard_id, version)
            );

CREATE INDEX hazard_versions_recorded_idx ON hazard_versions (recorded_at);

CREATE VIEW hazard_current AS
             SELECT v.hazard_id, v.version, v.recorded_at, v.author, v.state, v.event, v.body
             FROM hazard_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM hazard_versions n
                 WHERE n.hazard_id = v.hazard_id AND n.version > v.version
             );

CREATE FUNCTION hazard_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'hazard tables are append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER hazard_versions_no_change BEFORE UPDATE OR DELETE ON hazard_versions
             FOR EACH ROW EXECUTE FUNCTION hazard_versions_immutable();

CREATE TRIGGER hazard_versions_no_truncate BEFORE TRUNCATE ON hazard_versions
             FOR EACH STATEMENT EXECUTE FUNCTION hazard_versions_immutable();
