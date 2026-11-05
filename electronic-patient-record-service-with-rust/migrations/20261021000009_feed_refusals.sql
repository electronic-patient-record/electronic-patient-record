-- Migration: the work queue for HL7 messages the feed refused (task OR-3). `feed_refusals` holds
-- each refused message as received; `feed_refusal_resolutions` holds at most one decision per
-- refusal (a primary key on the refusal), so a message is never silently closed twice. Both tables
-- are append-only: triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`. "Open" is the absence of a
-- resolution, so closing a refusal changes no row.

CREATE TABLE feed_refusals (
                refusal_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                agent TEXT NOT NULL,
                peer TEXT,
                event TEXT NOT NULL,
                control_id TEXT NOT NULL,
                kind TEXT NOT NULL CHECK (kind IN ('unsupported', 'conflict')),
                reasons JSONB NOT NULL DEFAULT '[]'::jsonb,
                message TEXT NOT NULL
            );

CREATE TABLE feed_refusal_resolutions (
                refusal_id UUID PRIMARY KEY REFERENCES feed_refusals (refusal_id),
                resolved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                resolved_by TEXT NOT NULL,
                outcome TEXT NOT NULL CHECK (outcome IN ('handled', 'dismissed')),
                reason TEXT NOT NULL CHECK (length(btrim(reason)) > 0)
            );

CREATE INDEX feed_refusals_received_idx ON feed_refusals (received_at, refusal_id);

CREATE FUNCTION feed_refusals_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION '% is append-only', TG_TABLE_NAME;
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER feed_refusals_no_change BEFORE UPDATE OR DELETE ON feed_refusals
             FOR EACH ROW EXECUTE FUNCTION feed_refusals_immutable();

CREATE TRIGGER feed_refusals_no_truncate BEFORE TRUNCATE ON feed_refusals
             FOR EACH STATEMENT EXECUTE FUNCTION feed_refusals_immutable();

CREATE TRIGGER feed_refusal_resolutions_no_change BEFORE UPDATE OR DELETE ON feed_refusal_resolutions
             FOR EACH ROW EXECUTE FUNCTION feed_refusals_immutable();

CREATE TRIGGER feed_refusal_resolutions_no_truncate BEFORE TRUNCATE ON feed_refusal_resolutions
             FOR EACH STATEMENT EXECUTE FUNCTION feed_refusals_immutable();
