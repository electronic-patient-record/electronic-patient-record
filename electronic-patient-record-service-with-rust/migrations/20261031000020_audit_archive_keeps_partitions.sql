-- Migration: the archive step makes the partitions ahead again before it returns (OS-13).
-- 
-- Archiving the partition that holds this month leaves no partition for today's rows, and an insert with
-- no partition fails, so for up to six hours (until the maintenance loop) an access could not be audited
-- and a verification could not write its own row. The move function keeps its body under a new name
-- and `audit_archive_before` now calls it and then `audit_ensure_partitions(24)` in the same transaction.

ALTER FUNCTION audit_archive_before(timestamptz) RENAME TO audit_archive_move;

CREATE FUNCTION audit_archive_before(cutoff timestamptz) RETURNS text[] AS $$
             DECLARE
               moved text[];
             BEGIN
               moved := audit_archive_move(cutoff);
               -- The partition for this month may just have moved: make the months ahead again now.
               PERFORM audit_ensure_partitions(24);
               RETURN moved;
             END;
             $$ LANGUAGE plpgsql;
