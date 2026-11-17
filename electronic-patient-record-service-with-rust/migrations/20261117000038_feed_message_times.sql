-- Migration: the time of each message the HL7 feed applied, per feed and patient (task IH-5). An update older than
-- the newest one a feed has applied for a patient is not applied: it waits for a person, so a late or resent message
-- never undoes a newer one. One row per applied message; append-only, like the rest of the feed's tables: triggers
-- refuse UPDATE, DELETE and TRUNCATE. `control_id` is the fingerprint the feed keeps (IH-3), never MSH-10 itself.

CREATE TABLE feed_message_times (
                agent TEXT NOT NULL,
                patient_id UUID NOT NULL,
                message_time TIMESTAMPTZ NOT NULL,
                control_id TEXT NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
            );

CREATE INDEX feed_message_times_newest_idx ON feed_message_times (agent, patient_id, message_time DESC);

CREATE TRIGGER feed_message_times_no_change BEFORE UPDATE OR DELETE ON feed_message_times
             FOR EACH ROW EXECUTE FUNCTION feed_refusals_immutable();

CREATE TRIGGER feed_message_times_no_truncate BEFORE TRUNCATE ON feed_message_times
             FOR EACH STATEMENT EXECUTE FUNCTION feed_refusals_immutable();
