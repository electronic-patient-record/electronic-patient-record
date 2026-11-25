-- Migration (AG-12): which patients the duplicate check looks at. `family_keys` gives the family names of a body folded
-- (case, accents, spaces, hyphens, apostrophes), and each part of a double one of four letters or more; `given_keys` the
-- first given name of each. Both sides of a comparison are folded here, so there is one fold. **It only chooses whom the
-- matcher compares**: it decides nothing, changes nothing that was recorded and is never shown.
--
-- The keys are **stored columns the database makes from the body**. As index expressions they were computed for each row
-- again and again while ranking (25 ms became 54 ms for a name of 1,000 people in 50,000); stored, it is 6 ms. Adding them
-- rewrites `patient_versions` once, and holds it while it does (about 3 seconds for 50,000 rows): no recorded value
-- changes, and the triggers that refuse a change are as they were.
--
-- `pg_c_utf8` and not the database's collation, so that the fold is the same on every server and may be indexed
-- (PostgreSQL 17 or later, a UTF-8 database). The bodies are SQL-standard, as in migration 45.

CREATE FUNCTION name_key(name text) RETURNS text
             LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE
             RETURN replace(replace(replace(replace(translate(
                 normalize(lower(name COLLATE pg_c_utf8), NFKD),
                 E'\u00F8\u0142\u0111\u00F0\u0131\u0300\u0301\u0302\u0303\u0304\u0305\u0306\u0307\u0308\u0309\u030A\u030B\u030C\u030D\u030E\u030F\u0310\u0311\u0312\u0313\u0314\u0315\u0316\u0317\u0318\u0319\u031A\u031B\u031C\u031D\u031E\u031F\u0320\u0321\u0322\u0323\u0324\u0325\u0326\u0327\u0328\u0329\u032A\u032B\u032C\u032D\u032E\u032F\u0330\u0331\u0332\u0333\u0334\u0335\u0336\u0337\u0338\u0339\u033A\u033B\u033C\u033D\u033E\u033F\u0340\u0341\u0342\u0343\u0344\u0345\u0346\u0347\u0348\u0349\u034A\u034B\u034C\u034D\u034E\u034F\u0350\u0351\u0352\u0353\u0354\u0355\u0356\u0357\u0358\u0359\u035A\u035B\u035C\u035D\u035E\u035F\u0360\u0361\u0362\u0363\u0364\u0365\u0366\u0367\u0368\u0369\u036A\u036B\u036C\u036D\u036E\u036F\u0009\u000A\u000B\u000C\u000D\u0020\u0085\u00A0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200A\u2028\u2029\u202F\u205F\u3000\u0027\u2018\u2019\u02BC\u0060\u00B4\u002E\u2010\u2011\u2012\u2013\u2014-', 'olddi'),
                 E'\u00DF', 'ss'), E'\u00E6', 'ae'), E'\u0153', 'oe'), E'\u00FE', 'th');

CREATE FUNCTION family_keys(body jsonb) RETURNS text[]
             LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE
             RETURN (SELECT coalesce(array_agg(DISTINCT k), ARRAY[]::text[])
             FROM jsonb_array_elements(
                 CASE WHEN jsonb_typeof(body->'names') = 'array' THEN body->'names' ELSE '[]'::jsonb END
             ) n
             CROSS JOIN LATERAL (
                 SELECT name_key(n->>'family') AS k
                 UNION ALL
                 SELECT name_key(part) FROM regexp_split_to_table(n->>'family', '[[:space:]-]+') AS part
                 WHERE char_length(name_key(part)) >= 4
             ) keys
             WHERE k <> '');

CREATE FUNCTION given_keys(body jsonb) RETURNS text[]
             LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE
             RETURN (SELECT coalesce(array_agg(DISTINCT k), ARRAY[]::text[])
             FROM jsonb_array_elements(
                 CASE WHEN jsonb_typeof(body->'names') = 'array' THEN body->'names' ELSE '[]'::jsonb END
             ) n
             CROSS JOIN LATERAL (SELECT name_key(n->'given'->>0) AS k) keys
             WHERE k <> '');

ALTER TABLE patient_versions
             ADD COLUMN id_keys text[] GENERATED ALWAYS AS (identifier_keys(body)) STORED,
             ADD COLUMN family_name_keys text[] GENERATED ALWAYS AS (family_keys(body)) STORED,
             ADD COLUMN given_name_keys text[] GENERATED ALWAYS AS (given_keys(body)) STORED;

DROP INDEX patient_versions_identifier_keys;

CREATE INDEX patient_versions_id_keys ON patient_versions USING gin (id_keys);

CREATE INDEX patient_versions_family_name_keys ON patient_versions USING gin (family_name_keys);

CREATE OR REPLACE VIEW patient_current AS
             SELECT v.patient_id, v.version, v.recorded_at, v.author, v.reason, v.body, v.drill, v.id_keys,
                    v.family_name_keys, v.given_name_keys
             FROM patient_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM patient_versions n WHERE n.patient_id = v.patient_id AND n.version > v.version
             );
