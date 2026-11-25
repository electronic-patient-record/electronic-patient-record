-- Migration: compatibility typing (DM-1), what laboratories reported about a person's blood groups, HLA types, antibodies
-- and cytomegalovirus status. Append-only versions with a `*_current` view, as `problem_versions`; triggers refuse
-- `UPDATE`, `DELETE` and `TRUNCATE`. Nothing here names a donor or a recipient, or says that two people match.

CREATE TABLE typing_versions (
                typing_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
                author TEXT NOT NULL,
                reason TEXT,
                status TEXT NOT NULL CHECK (status IN ('reported', 'entered-in-error')),
                body JSONB NOT NULL,
                PRIMARY KEY (typing_id, version)
            );

CREATE INDEX typing_versions_patient_idx ON typing_versions (patient_id, recorded_at);

CREATE VIEW typing_current AS
             SELECT v.typing_id, v.version, v.patient_id, v.recorded_at, v.author, v.reason, v.status, v.body
             FROM typing_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM typing_versions n WHERE n.typing_id = v.typing_id AND n.version > v.version
             );

CREATE FUNCTION typing_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'typing_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER typing_versions_no_change BEFORE UPDATE OR DELETE ON typing_versions
             FOR EACH ROW EXECUTE FUNCTION typing_versions_immutable();

CREATE TRIGGER typing_versions_no_truncate BEFORE TRUNCATE ON typing_versions
             FOR EACH STATEMENT EXECUTE FUNCTION typing_versions_immutable();
