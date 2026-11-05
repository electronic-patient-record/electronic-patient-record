-- Migration (EC-10): a patient version may be a drill or test record. `patient_versions.drill` is false for
-- every existing row and every new row unless the registration says otherwise, and each version of a patient
-- carries the flag of the one before. `patient_current` gains the column so search and the duplicate check
-- can keep drill and real records apart. The table stays append-only; no row is touched.

ALTER TABLE patient_versions ADD COLUMN drill BOOLEAN NOT NULL DEFAULT false;

CREATE OR REPLACE VIEW patient_current AS
             SELECT v.patient_id, v.version, v.recorded_at, v.author, v.reason, v.body, v.drill
             FROM patient_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM patient_versions n WHERE n.patient_id = v.patient_id AND n.version > v.version
             );
