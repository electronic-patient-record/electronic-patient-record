-- Migration: the append-only `audit_events` table (plan AU-1).
-- 
-- One row per access (read or write). Rows are hash-chained: `prev_digest` is
-- the previous row's `digest`, so a missing or altered row breaks the chain.
-- Triggers refuse `UPDATE`, `DELETE` and `TRUNCATE`.

CREATE TABLE audit_events (
                id BIGSERIAL PRIMARY KEY,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                -- FHIR AuditEvent.action: C, R, U, D, E
                action CHAR(1) NOT NULL CHECK (action IN ('C','R','U','D','E')),
                -- 0 success, 4 minor failure, 8 serious failure, 12 major failure
                outcome SMALLINT NOT NULL DEFAULT 0 CHECK (outcome IN (0,4,8,12)),
                agent TEXT NOT NULL,
                patient_ref TEXT,
                resource_ref TEXT,
                purpose_of_use TEXT,
                break_glass BOOLEAN NOT NULL DEFAULT FALSE,
                network_address TEXT,
                detail JSONB,
                prev_digest TEXT,
                digest TEXT NOT NULL
            );

CREATE INDEX audit_events_patient_idx ON audit_events (patient_ref, recorded_at);

CREATE FUNCTION audit_events_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'audit_events is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER audit_events_no_change BEFORE UPDATE OR DELETE ON audit_events
             FOR EACH ROW EXECUTE FUNCTION audit_events_immutable();

CREATE TRIGGER audit_events_no_truncate BEFORE TRUNCATE ON audit_events
             FOR EACH STATEMENT EXECUTE FUNCTION audit_events_immutable();
