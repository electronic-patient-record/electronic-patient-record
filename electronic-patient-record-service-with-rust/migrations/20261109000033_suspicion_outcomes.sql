-- Migration: how a suspicion ends (RS-3, hazard H-054). A suspicion ends only with a recorded outcome: confirmed
-- (with the diagnosis), ruled out (who, and on what) or withdrawn (why); never by its referral closing, being
-- cancelled or redirected, or a missed test or appointment. One outcome per referral's suspicion, naming the
-- version it ended. Append-only: triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. `patient_id` and `body` (the
-- diagnosis, who ruled it out and on what, or why it was withdrawn) are patient data.

CREATE TABLE referral_suspicion_outcomes (
                referral_id UUID PRIMARY KEY,
                suspicion_version INTEGER NOT NULL,
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                kind TEXT NOT NULL CHECK (kind IN ('confirmed', 'ruled-out', 'withdrawn')),
                body JSONB NOT NULL,
                FOREIGN KEY (referral_id, suspicion_version)
                    REFERENCES referral_suspicion_versions (referral_id, version)
            );

CREATE INDEX referral_suspicion_outcomes_patient_idx ON referral_suspicion_outcomes (patient_id, recorded_at);

CREATE FUNCTION referral_suspicion_outcomes_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'referral_suspicion_outcomes is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER referral_suspicion_outcomes_no_change BEFORE UPDATE OR DELETE ON referral_suspicion_outcomes
             FOR EACH ROW EXECUTE FUNCTION referral_suspicion_outcomes_immutable();

CREATE TRIGGER referral_suspicion_outcomes_no_truncate BEFORE TRUNCATE ON referral_suspicion_outcomes
             FOR EACH STATEMENT EXECUTE FUNCTION referral_suspicion_outcomes_immutable();
