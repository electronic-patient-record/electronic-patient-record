-- Migration: integration tokens (TL-1). A system signs in with a named token, not a person's sign-in. The token itself
-- is shown once, when it is issued, and never kept: `api_tokens` holds only the SHA-256 hash of its secret
-- (`axa::api_tokens`), so a copy of this table lets no one in.
--
-- What a token is never changes: its name (its audit agent, so never reused), hash, scopes, expiry and who issued it
-- (`api_tokens`, which refuses `UPDATE`, `DELETE` and `TRUNCATE`). What may change, its description and whether it is
-- revoked, is a series of versions (`api_token_versions`, append-only, version 1 at issue). Its last use
-- (`api_token_uses`, one row per token) is operational: written at most once a minute per token, so it may be updated,
-- and never deleted. No patient data. `created_by` and `author` are staff identifiers.

CREATE TABLE api_tokens (
                token_id UUID PRIMARY KEY,
                name TEXT NOT NULL UNIQUE CHECK (name ~ '^[a-z][a-z0-9-]{1,63}$'),
                hash TEXT NOT NULL CHECK (hash ~ '^[0-9a-f]{64}$'),
                scopes TEXT[] NOT NULL CHECK (cardinality(scopes) BETWEEN 1 AND 16),
                expires_at TIMESTAMPTZ NOT NULL,
                created_by TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                CHECK (expires_at > created_at)
            );

CREATE TABLE api_token_versions (
                token_id UUID NOT NULL REFERENCES api_tokens (token_id),
                version INTEGER NOT NULL CHECK (version >= 1),
                description TEXT NOT NULL CHECK (char_length(description) <= 500),
                revoked BOOLEAN NOT NULL,
                reason TEXT CHECK (reason IS NULL OR char_length(reason) BETWEEN 1 AND 500),
                author TEXT NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                PRIMARY KEY (token_id, version),
                CHECK (version > 1 OR NOT revoked),
                CHECK (revoked = (reason IS NOT NULL))
            );

CREATE VIEW api_token_current AS
             SELECT t.token_id, t.name, t.hash, t.scopes, t.expires_at, t.created_by, t.created_at,
                    v.version, v.description, v.revoked, v.reason, v.author, v.recorded_at
             FROM api_tokens t
             JOIN api_token_versions v ON v.token_id = t.token_id
             WHERE NOT EXISTS (
                 SELECT 1 FROM api_token_versions n
                 WHERE n.token_id = v.token_id AND n.version > v.version
             );

CREATE TABLE api_token_uses (
                token_id UUID PRIMARY KEY REFERENCES api_tokens (token_id),
                last_used_at TIMESTAMPTZ NOT NULL
            );

CREATE FUNCTION api_token_tables_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION '% is append-only', TG_TABLE_NAME;
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER api_tokens_no_change BEFORE UPDATE OR DELETE ON api_tokens
             FOR EACH ROW EXECUTE FUNCTION api_token_tables_immutable();
CREATE TRIGGER api_tokens_no_truncate BEFORE TRUNCATE ON api_tokens
             FOR EACH STATEMENT EXECUTE FUNCTION api_token_tables_immutable();
CREATE TRIGGER api_token_versions_no_change BEFORE UPDATE OR DELETE ON api_token_versions
             FOR EACH ROW EXECUTE FUNCTION api_token_tables_immutable();
CREATE TRIGGER api_token_versions_no_truncate BEFORE TRUNCATE ON api_token_versions
             FOR EACH STATEMENT EXECUTE FUNCTION api_token_tables_immutable();
CREATE TRIGGER api_token_uses_no_delete BEFORE DELETE ON api_token_uses
             FOR EACH ROW EXECUTE FUNCTION api_token_tables_immutable();
CREATE TRIGGER api_token_uses_no_truncate BEFORE TRUNCATE ON api_token_uses
             FOR EACH STATEMENT EXECUTE FUNCTION api_token_tables_immutable();
