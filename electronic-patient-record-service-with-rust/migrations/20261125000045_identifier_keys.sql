-- Migration (AG-12): identifiers are looked up by what they mean. `identifier_keys` gives one text per identifier of a
-- patient's body, `system|value`, with the spaces and hyphens taken out of a **national number** (the two systems the
-- matcher reads that way) and every other value as it was typed. Registration, search and the HL7 feed compare keys, so
-- `943 476 5919` finds `9434765919`. The record keeps what was typed. `tests/identifier_keys.rs` holds this function to
-- the Rust one that makes the key asked for (`patients_store::identifier_key`). The bodies are SQL-standard (`RETURN`), so
-- each names what it calls when it is made: an index is built, and a dump restored, with no search path to find it by.

CREATE FUNCTION identifier_key(system text, value text) RETURNS text
             LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE
             RETURN system || '|' || CASE
                 WHEN system IN ('https://fhir.nhs.uk/Id/nhs-number', 'http://ns.electronichealth.net.au/id/hi/ihi/1.0')
                 THEN translate(value, E'\u0009\u000A\u000B\u000C\u000D\u0020\u0085\u00A0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200A\u2028\u2029\u202F\u205F\u3000-', '')
                 ELSE value
             END;

CREATE FUNCTION identifier_keys(body jsonb) RETURNS text[]
             LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE
             RETURN (SELECT coalesce(array_agg(identifier_key(i->>'system', i->>'value')), ARRAY[]::text[])
             FROM jsonb_array_elements(
                 CASE WHEN jsonb_typeof(body->'identifiers') = 'array' THEN body->'identifiers' ELSE '[]'::jsonb END
             ) i
             WHERE i->>'system' IS NOT NULL AND i->>'value' IS NOT NULL);

CREATE INDEX patient_versions_identifier_keys ON patient_versions USING gin (identifier_keys(body));

-- Nothing looks identifiers up as JSON any more.
DROP INDEX patient_versions_identifiers;
