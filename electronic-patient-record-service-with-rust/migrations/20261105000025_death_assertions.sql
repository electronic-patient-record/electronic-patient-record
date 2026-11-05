-- Migration: the append-only `death_assertion_versions` table and its `death_assertion_current` view
-- (task BV-1). A report that a patient has died is a series of versions; none is edited or removed,
-- and triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. A withdrawal is a new version, so a death
-- recorded in error stays in the history with its reason.

CREATE TABLE death_assertion_versions (
                assertion_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('reported', 'verified', 'certified', 'registered', 'withdrawn-in-error')),
                body JSONB NOT NULL,
                PRIMARY KEY (assertion_id, version)
            );

CREATE INDEX death_assertion_versions_patient_idx
             ON death_assertion_versions (patient_id, recorded_at);

CREATE VIEW death_assertion_current AS
             SELECT v.assertion_id, v.version, v.patient_id, v.recorded_at, v.author, v.status, v.body
             FROM death_assertion_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM death_assertion_versions n
                 WHERE n.assertion_id = v.assertion_id AND n.version > v.version
             );

CREATE FUNCTION death_assertion_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'death_assertion_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER death_assertion_versions_no_change
             BEFORE UPDATE OR DELETE ON death_assertion_versions
             FOR EACH ROW EXECUTE FUNCTION death_assertion_versions_immutable();

CREATE TRIGGER death_assertion_versions_no_truncate
             BEFORE TRUNCATE ON death_assertion_versions
             FOR EACH STATEMENT EXECUTE FUNCTION death_assertion_versions_immutable();
