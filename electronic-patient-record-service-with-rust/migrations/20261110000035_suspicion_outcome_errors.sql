-- Migration: an outcome recorded in error (RS-7, hazard H-054). A suspicion's outcome (confirmed, ruled out,
-- withdrawn) may be marked entered in error by a clinician with a reason; the suspicion is then open again, may
-- be amended, and may be given a new outcome. Nothing is removed: the outcome row stays, the error is its own
-- append-only row, and the history shows both.
--
-- So a referral's suspicion can hold more than one outcome over time, each numbered (`seq`, from 1). Adding the
-- column with a constant default and re-keying the table on (referral_id, seq) are schema changes, not row
-- changes: no row is updated or deleted, the existing rows read as seq 1, and the append-only triggers still
-- refuse `UPDATE`, `DELETE` and `TRUNCATE`. The current outcome is the one with no error row
-- (`referral_suspicion_outcome_current`); a new outcome needs every earlier one marked in error (the store
-- checks, and a concurrent second writer meets the primary key). `patient_id` and `reason` are patient data.

ALTER TABLE referral_suspicion_outcomes ADD COLUMN seq INTEGER NOT NULL DEFAULT 1 CHECK (seq >= 1);

ALTER TABLE referral_suspicion_outcomes DROP CONSTRAINT referral_suspicion_outcomes_pkey;

ALTER TABLE referral_suspicion_outcomes ADD PRIMARY KEY (referral_id, seq);

CREATE TABLE referral_suspicion_outcome_errors (
                referral_id UUID NOT NULL,
                seq INTEGER NOT NULL,
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                reason TEXT NOT NULL CHECK (length(btrim(reason)) > 0),
                PRIMARY KEY (referral_id, seq),
                FOREIGN KEY (referral_id, seq) REFERENCES referral_suspicion_outcomes (referral_id, seq)
            );

CREATE INDEX referral_suspicion_outcome_errors_patient_idx
    ON referral_suspicion_outcome_errors (patient_id, recorded_at);

CREATE VIEW referral_suspicion_outcome_current AS
             SELECT o.referral_id, o.seq, o.suspicion_version, o.patient_id, o.recorded_at, o.author, o.kind, o.body
             FROM referral_suspicion_outcomes o
             WHERE NOT EXISTS (
                 SELECT 1 FROM referral_suspicion_outcome_errors e
                 WHERE e.referral_id = o.referral_id AND e.seq = o.seq
             );

CREATE FUNCTION referral_suspicion_outcome_errors_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'referral_suspicion_outcome_errors is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER referral_suspicion_outcome_errors_no_change BEFORE UPDATE OR DELETE ON referral_suspicion_outcome_errors
             FOR EACH ROW EXECUTE FUNCTION referral_suspicion_outcome_errors_immutable();

CREATE TRIGGER referral_suspicion_outcome_errors_no_truncate BEFORE TRUNCATE ON referral_suspicion_outcome_errors
             FOR EACH STATEMENT EXECUTE FUNCTION referral_suspicion_outcome_errors_immutable();
