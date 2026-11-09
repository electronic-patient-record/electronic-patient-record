-- Migration: a suspicion on a referral (RS-1, `spec/referral-suspicion/index.md`, hazard H-053). One suspicion per
-- referral, as a series of versions: version 1 is the suspicion as raised, each later version an amendment with a
-- reason (the CHECK holds both ways), because a clock may start from the point of suspicion. Every row names the
-- pathway and version it was made under (`src/suspicion_pathways.toml`). Append-only: triggers refuse `UPDATE`,
-- `DELETE` and `TRUNCATE`. `patient_id`, `reason` and `body` (the condition, the grounds, what the patient was told,
-- any factors and agreement) are patient data.

CREATE TABLE referral_suspicion_versions (
                referral_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT,
                pathway_id TEXT NOT NULL,
                pathway_version INTEGER NOT NULL CHECK (pathway_version >= 1),
                body JSONB NOT NULL,
                PRIMARY KEY (referral_id, version),
                CHECK ((version = 1) = (reason IS NULL))
            );

CREATE INDEX referral_suspicion_versions_patient_idx ON referral_suspicion_versions (patient_id, recorded_at);

CREATE VIEW referral_suspicion_current AS
             SELECT v.referral_id, v.version, v.patient_id, v.recorded_at, v.author, v.reason, v.pathway_id,
                    v.pathway_version, v.body
             FROM referral_suspicion_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM referral_suspicion_versions n
                 WHERE n.referral_id = v.referral_id AND n.version > v.version
             );

CREATE FUNCTION referral_suspicion_versions_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'referral_suspicion_versions is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER referral_suspicion_versions_no_change BEFORE UPDATE OR DELETE ON referral_suspicion_versions
             FOR EACH ROW EXECUTE FUNCTION referral_suspicion_versions_immutable();

CREATE TRIGGER referral_suspicion_versions_no_truncate BEFORE TRUNCATE ON referral_suspicion_versions
             FOR EACH STATEMENT EXECUTE FUNCTION referral_suspicion_versions_immutable();
