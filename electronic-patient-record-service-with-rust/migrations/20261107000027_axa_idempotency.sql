-- Migration: `axa_idempotency` replaces `idempotency_keys` (task OF-1). `Idempotency-Key` handling moves to
-- `axa-idempotency`, which keeps its keys in this table: these are its `SETUP_STATEMENTS`, word for word
-- (`idempotency::tests` checks that they still are). `body` holds an answer as it was sent, so it can hold
-- patient data: rows expire after a day and the service sweeps them hourly. This is **not** clinical data (the
-- record is written elsewhere, append-only), which is why this table, unlike the clinical ones, may delete.
--
-- The old table is dropped, with any key in it. A create that was sent before the upgrade and is retried after it
-- runs as if it had no key; at most a day of keys is lost, and each was a copy of an answer, never the record.

CREATE TABLE IF NOT EXISTS axa_idempotency (
    caller TEXT NOT NULL,
    key TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    claim UUID NOT NULL,
    state TEXT NOT NULL DEFAULT 'running' CHECK (state IN ('running', 'done', 'not_kept')),
    status INTEGER,
    headers JSONB,
    body BYTEA,
    lease_until TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (caller, key)
);

CREATE INDEX IF NOT EXISTS axa_idempotency_expiry ON axa_idempotency (expires_at);

DROP TABLE idempotency_keys;
