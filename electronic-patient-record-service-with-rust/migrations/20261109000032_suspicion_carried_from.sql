-- Migration: a suspicion carried to the referral that replaces a redirected one (RS-2, hazard H-054). A redirect
-- ends a referral and a new referral replaces it; the suspicion goes with the patient, not the referral, so the
-- replacement's version 1 is a copy of the old referral's current suspicion, the same point of suspicion included,
-- with `carried_from` naming the referral it came from. Only a version 1 can be carried (the CHECK). Adding a
-- nullable column changes no row, and the append-only triggers still refuse `UPDATE` and `DELETE`.

ALTER TABLE referral_suspicion_versions ADD COLUMN carried_from UUID;

ALTER TABLE referral_suspicion_versions
    ADD CONSTRAINT referral_suspicion_carried_first CHECK (carried_from IS NULL OR version = 1);

CREATE OR REPLACE VIEW referral_suspicion_current AS
             SELECT v.referral_id, v.version, v.patient_id, v.recorded_at, v.author, v.reason, v.pathway_id,
                    v.pathway_version, v.body, v.carried_from
             FROM referral_suspicion_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM referral_suspicion_versions n
                 WHERE n.referral_id = v.referral_id AND n.version > v.version
             );
