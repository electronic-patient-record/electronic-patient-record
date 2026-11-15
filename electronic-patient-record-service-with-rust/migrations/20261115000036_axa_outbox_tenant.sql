-- Migration: the outbox moves to `axa-transactional-outbox` 0.4.0 (axa 0.14), which writes a tenant.
--
-- The statements below are the ones of `axa::outbox::SETUP_STATEMENTS` that migration 20261108100028 does not
-- hold, in axa's order and words; `outbox::tests` fails if the two migrations together drift from axa. Each is safe
-- to repeat. They add a nullable `tenant` to `axa_outbox_events` and `axa_outbox_owed` (rows already there have
-- none), a second form of `axa_outbox_fingerprint` that covers a tenant (with none it is the first form byte for
-- byte, so no stored fingerprint changes), and the trigger that copies an event's tenant to what is owed.
-- `Outbox::connect_existing` refuses tables without them. Nothing is removed and no row is changed.
--
-- This service names no tenant: tenancy (SE-5) is not built, and every event is written with none.

ALTER TABLE axa_outbox_events ADD COLUMN IF NOT EXISTS tenant TEXT
    CONSTRAINT axa_outbox_events_tenant_id
    CHECK (tenant ~ '^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$');

ALTER TABLE axa_outbox_owed ADD COLUMN IF NOT EXISTS tenant TEXT;

CREATE OR REPLACE FUNCTION axa_outbox_fingerprint(
    event_id UUID, topic TEXT, kind TEXT, key TEXT, tenant TEXT, payload JSONB, sealed BYTEA)
RETURNS TEXT LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT encode(sha256(
        uuid_send(event_id)
        || convert_to(topic, 'UTF8') || '\x00'::bytea
        || convert_to(kind, 'UTF8') || '\x00'::bytea
        || convert_to(key, 'UTF8') || '\x00'::bytea
        || CASE WHEN tenant IS NULL THEN ''::bytea
                ELSE '\x74'::bytea || convert_to(tenant, 'UTF8') || '\x00'::bytea END
        || CASE WHEN sealed IS NOT NULL THEN '\x73'::bytea || sealed
                ELSE '\x6a'::bytea || convert_to(payload::text, 'UTF8') END
    ), 'hex')
$$;

CREATE OR REPLACE FUNCTION axa_outbox_owe() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO axa_outbox_owed (event_id, id, topic, key, tenant)
    VALUES (NEW.event_id, NEW.id, NEW.topic, NEW.key, NEW.tenant);
    RETURN NULL;
END
$$;
