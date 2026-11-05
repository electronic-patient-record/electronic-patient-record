-- Migration (CQ-9): the consent, allergy, relationship and link `*_current` views as an anti-join
-- ("no later version of this record exists") instead of `DISTINCT ON` the record's id. A page asks
-- for one patient's records; under `DISTINCT ON (consent_id)` Postgres cannot push a `patient_id`
-- filter below the `DISTINCT ON`, so it built the current version of every record first. As an
-- anti-join the filter reaches the versions table and its patient index, and the "no later
-- version" probe uses the primary key. The columns, and the rows, are the same
-- (`tests/current_views.rs` compares them with the old definition).

CREATE OR REPLACE VIEW consent_current AS
                 SELECT v.consent_id, v.version, v.patient_id, v.recorded_at, v.author, v.reason, v.status, v.body FROM consent_versions v
                 WHERE NOT EXISTS (
                     SELECT 1 FROM consent_versions n WHERE n.consent_id = v.consent_id AND n.version > v.version
                 );

CREATE OR REPLACE VIEW allergy_current AS
                 SELECT v.allergy_id, v.version, v.patient_id, v.recorded_at, v.author, v.reason, v.status, v.body FROM allergy_versions v
                 WHERE NOT EXISTS (
                     SELECT 1 FROM allergy_versions n WHERE n.allergy_id = v.allergy_id AND n.version > v.version
                 );

CREATE OR REPLACE VIEW relationship_current AS
                 SELECT v.relationship_id, v.version, v.patient_id, v.subject, v.kind, v.status, v.recorded_at, v.author, v.reason, v.body FROM relationship_versions v
                 WHERE NOT EXISTS (
                     SELECT 1 FROM relationship_versions n WHERE n.relationship_id = v.relationship_id AND n.version > v.version
                 );

CREATE OR REPLACE VIEW patient_link_current AS
                 SELECT v.link_id, v.version, v.recorded_at, v.author, v.reason, v.source_patient_id, v.target_patient_id, v.status, v.match_score, v.match_decision FROM patient_link_versions v
                 WHERE NOT EXISTS (
                     SELECT 1 FROM patient_link_versions n WHERE n.link_id = v.link_id AND n.version > v.version
                 );
