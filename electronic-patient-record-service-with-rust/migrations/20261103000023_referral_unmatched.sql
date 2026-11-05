-- Migration: the unmatched-outcome queue (task PC-2, hazard H-044). An inbound referral message (an
-- outcome, a decline, an interim note) that names no referral, names one that does not exist, or asks
-- for a step the state table refuses is kept as received in `referral_unmatched`, never dropped and never
-- matched by patient and date. A person closes an entry with a row in `referral_unmatched_clearances`
-- (matched by hand to a named referral, or dismissed with a reason). Both tables are append-only: triggers
-- refuse `UPDATE`, `DELETE` and `TRUNCATE`. `message` and `note` are patient data.

CREATE TABLE referral_unmatched (
                unmatched_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                agent TEXT NOT NULL,
                kind TEXT NOT NULL CHECK (kind IN ('no-id', 'unknown-id', 'refused-by-table')),
                referral_ref TEXT,
                event TEXT,
                reasons JSONB NOT NULL,
                note TEXT,
                message TEXT NOT NULL
            );

CREATE TABLE referral_unmatched_clearances (
                unmatched_id UUID PRIMARY KEY REFERENCES referral_unmatched (unmatched_id),
                cleared_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                cleared_by TEXT NOT NULL,
                how TEXT NOT NULL CHECK (how IN ('matched', 'dismissed')),
                referral_id UUID,
                reason TEXT NOT NULL,
                CHECK ((how = 'matched') = (referral_id IS NOT NULL))
            );

CREATE FUNCTION referral_unmatched_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'referral queue tables are append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER referral_unmatched_no_change BEFORE UPDATE OR DELETE ON referral_unmatched
             FOR EACH ROW EXECUTE FUNCTION referral_unmatched_immutable();

CREATE TRIGGER referral_unmatched_no_truncate BEFORE TRUNCATE ON referral_unmatched
             FOR EACH STATEMENT EXECUTE FUNCTION referral_unmatched_immutable();

CREATE TRIGGER referral_unmatched_clearances_no_change
             BEFORE UPDATE OR DELETE ON referral_unmatched_clearances
             FOR EACH ROW EXECUTE FUNCTION referral_unmatched_immutable();

CREATE TRIGGER referral_unmatched_clearances_no_truncate
             BEFORE TRUNCATE ON referral_unmatched_clearances
             FOR EACH STATEMENT EXECUTE FUNCTION referral_unmatched_immutable();
