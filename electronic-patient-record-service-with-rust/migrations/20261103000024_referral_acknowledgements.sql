-- Migration: the referrer's worklist (task PC-2/PC-3 groundwork, hazard H-044). A referral that moved into a
-- state the referrer should look at (declined, redirected, no-show, deferred, information requested, an interim
-- note, an outcome) stays on the referrer's worklist until the referrer acknowledges *that version*. An
-- acknowledgement is a row in `referral_acknowledgements`, never an edit: a later version of the same referral
-- needs a new one. The table is append-only: triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.

CREATE TABLE referral_acknowledgements (
                referral_id UUID NOT NULL,
                version INTEGER NOT NULL,
                acknowledged_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                acknowledged_by TEXT NOT NULL,
                PRIMARY KEY (referral_id, version),
                FOREIGN KEY (referral_id, version) REFERENCES referral_versions (referral_id, version)
            );

CREATE FUNCTION referral_acknowledgements_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'referral_acknowledgements is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER referral_acknowledgements_no_change
             BEFORE UPDATE OR DELETE ON referral_acknowledgements
             FOR EACH ROW EXECUTE FUNCTION referral_acknowledgements_immutable();

CREATE TRIGGER referral_acknowledgements_no_truncate
             BEFORE TRUNCATE ON referral_acknowledgements
             FOR EACH STATEMENT EXECUTE FUNCTION referral_acknowledgements_immutable();
