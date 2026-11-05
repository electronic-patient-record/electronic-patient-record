-- Migration: the transactional outbox (task OS-3). `outbox_events` holds an event that must leave
-- the service, written in the same transaction as the change it reports; `outbox_attempts` holds what
-- happened to it (`delivered`, `refused` by policy or consent, or `failed`). Neither is edited or
-- removed, and triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`, so what was disclosed, and what was
-- refused, stays on record. `payload` and `patient_id` are patient data; `reason` names no content.

CREATE TABLE outbox_events (
                id UUID PRIMARY KEY,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                kind TEXT NOT NULL CHECK (kind <> ''),
                recipient TEXT NOT NULL CHECK (recipient <> ''),
                recipient_roles TEXT[] NOT NULL,
                purpose TEXT NOT NULL,
                resource TEXT NOT NULL,
                patient_id UUID,
                payload JSONB NOT NULL
            );

CREATE TABLE outbox_attempts (
                id BIGSERIAL PRIMARY KEY,
                event_id UUID NOT NULL REFERENCES outbox_events (id),
                attempted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                outcome TEXT NOT NULL CHECK (outcome IN ('delivered', 'refused', 'failed')),
                reason TEXT
            );

CREATE INDEX outbox_attempts_event_idx ON outbox_attempts (event_id, outcome);

CREATE INDEX outbox_events_created_idx ON outbox_events (created_at);

CREATE FUNCTION outbox_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'outbox tables are append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER outbox_events_no_change BEFORE UPDATE OR DELETE ON outbox_events
             FOR EACH ROW EXECUTE FUNCTION outbox_immutable();

CREATE TRIGGER outbox_events_no_truncate BEFORE TRUNCATE ON outbox_events
             FOR EACH STATEMENT EXECUTE FUNCTION outbox_immutable();

CREATE TRIGGER outbox_attempts_no_change BEFORE UPDATE OR DELETE ON outbox_attempts
             FOR EACH ROW EXECUTE FUNCTION outbox_immutable();

CREATE TRIGGER outbox_attempts_no_truncate BEFORE TRUNCATE ON outbox_attempts
             FOR EACH STATEMENT EXECUTE FUNCTION outbox_immutable();
