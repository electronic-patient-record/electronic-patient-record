-- Migration: the append-only `patient_photos` table and `patient_photo_current` view (patient
-- banner photo). Same pattern as `patient_versions`: a new photo, or the withdrawal of one, is a
-- new version, never an edit; triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.

CREATE TABLE patient_photos (
                patient_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT,
                -- NULL bytes and content type: this version withdraws the photo
                content_type TEXT CHECK (content_type IN ('image/png', 'image/jpeg')),
                bytes BYTEA CHECK (bytes IS NULL OR octet_length(bytes) BETWEEN 1 AND 524288),
                sha256 TEXT,
                PRIMARY KEY (patient_id, version),
                CHECK ((bytes IS NULL) = (content_type IS NULL) AND (bytes IS NULL) = (sha256 IS NULL))
            );

CREATE VIEW patient_photo_current AS
             SELECT DISTINCT ON (patient_id) patient_id, version, recorded_at, author, reason,
                    content_type, bytes, sha256
             FROM patient_photos ORDER BY patient_id, version DESC;

CREATE FUNCTION patient_photos_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'patient_photos is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER patient_photos_no_change BEFORE UPDATE OR DELETE ON patient_photos
             FOR EACH ROW EXECUTE FUNCTION patient_photos_immutable();

CREATE TRIGGER patient_photos_no_truncate BEFORE TRUNCATE ON patient_photos
             FOR EACH STATEMENT EXECUTE FUNCTION patient_photos_immutable();
