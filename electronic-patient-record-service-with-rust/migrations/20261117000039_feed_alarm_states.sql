-- Migration: each change of an HL7 feed's health state (task IH-8, hazard H-023). The feed's alarm loop stores a row only
-- when the state changes, and reads the last one after a restart, so a restart does not raise an alarm again.
-- Append-only: triggers refuse UPDATE, DELETE and TRUNCATE. `reasons` are fixed words, never message content.

CREATE TABLE feed_alarm_states (
                seq BIGSERIAL PRIMARY KEY,
                agent TEXT NOT NULL,
                state TEXT NOT NULL CHECK (state IN ('unknown', 'healthy', 'warning', 'critical')),
                reasons JSONB NOT NULL DEFAULT '[]'::jsonb,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
            );

CREATE INDEX feed_alarm_states_last_idx ON feed_alarm_states (agent, recorded_at DESC, seq DESC);

CREATE TRIGGER feed_alarm_states_no_change BEFORE UPDATE OR DELETE ON feed_alarm_states
             FOR EACH ROW EXECUTE FUNCTION feed_refusals_immutable();

CREATE TRIGGER feed_alarm_states_no_truncate BEFORE TRUNCATE ON feed_alarm_states
             FOR EACH STATEMENT EXECUTE FUNCTION feed_refusals_immutable();

-- The health check reads the newest audit row a feed's agent wrote; without this it scans the trail.
CREATE INDEX IF NOT EXISTS audit_events_agent_resource_idx ON audit_events (agent, resource_ref, recorded_at DESC);
