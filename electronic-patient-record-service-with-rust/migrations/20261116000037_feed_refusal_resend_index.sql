-- Migration: find a refused message that is already waiting (task IH-1). The feed looks up an open entry by agent and
-- control id before it keeps a message again, so a sender's resend adds no row. An index only: no table changes.
CREATE INDEX feed_refusals_resend_idx ON feed_refusals (agent, control_id);
