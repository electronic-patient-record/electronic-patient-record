-- Keycloak (compose profile `keycloak`) keeps its state in its own database on this PostgreSQL.
-- Runs once, on first initialization of a fresh data directory, like 00-extensions.sql. An existing
-- data volume never re-runs it: create the database by hand (operations/keycloak/README.md).
SELECT 'CREATE DATABASE keycloak'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'keycloak')\gexec
