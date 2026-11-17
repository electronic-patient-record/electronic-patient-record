-- Migration: tasks from rules (TL-3, TL-4). A task is a series of versions: version 1 as a rule made it (`open`),
-- version 2 when a person closes it (`done`) or an event or a person cancels it (`cancelled`), with why. The rule and
-- its version are named, so a task always says what made it. Append-only: triggers refuse `UPDATE`, `DELETE` and
-- `TRUNCATE`. `patient_id` and `body` (the task's words, due day, reminders, the record it came from, why it closed)
-- are patient data.
--
-- Each reminder is decided once (`task_reminder_decisions`, one row per task and day: sent, or skipped with the
-- reason), and each run of the reminder sender is recorded (`task_reminder_runs`), so a reminder whose day fell while
-- nothing ran is known to be late.

CREATE TABLE task_versions (
                task_id UUID NOT NULL,
                version INTEGER NOT NULL CHECK (version >= 1),
                patient_id UUID NOT NULL,
                rule_id TEXT NOT NULL,
                rule_version INTEGER NOT NULL CHECK (rule_version >= 1),
                team TEXT NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                author TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('open', 'done', 'cancelled')),
                due_on DATE NOT NULL,
                body JSONB NOT NULL,
                PRIMARY KEY (task_id, version),
                CHECK ((version = 1) = (status = 'open'))
            );

CREATE INDEX task_versions_team_idx ON task_versions (team, due_on);
CREATE INDEX task_versions_patient_idx ON task_versions (patient_id, recorded_at);

CREATE VIEW task_current AS
             SELECT v.task_id, v.version, v.patient_id, v.rule_id, v.rule_version, v.team, v.recorded_at, v.author,
                    v.status, v.due_on, v.body
             FROM task_versions v
             WHERE NOT EXISTS (
                 SELECT 1 FROM task_versions n
                 WHERE n.task_id = v.task_id AND n.version > v.version
             );

CREATE TABLE task_reminder_decisions (
                task_id UUID NOT NULL,
                remind_on DATE NOT NULL,
                decision TEXT NOT NULL CHECK (decision IN ('sent', 'skipped')),
                reason TEXT,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                PRIMARY KEY (task_id, remind_on),
                CHECK ((decision = 'skipped') = (reason IS NOT NULL))
            );

CREATE TABLE task_reminder_runs (
                ran_on DATE NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
            );

CREATE INDEX task_reminder_runs_idx ON task_reminder_runs (recorded_at);

CREATE FUNCTION task_tables_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION '% is append-only', TG_TABLE_NAME;
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER task_versions_no_change BEFORE UPDATE OR DELETE ON task_versions
             FOR EACH ROW EXECUTE FUNCTION task_tables_immutable();
CREATE TRIGGER task_versions_no_truncate BEFORE TRUNCATE ON task_versions
             FOR EACH STATEMENT EXECUTE FUNCTION task_tables_immutable();
CREATE TRIGGER task_reminder_decisions_no_change BEFORE UPDATE OR DELETE ON task_reminder_decisions
             FOR EACH ROW EXECUTE FUNCTION task_tables_immutable();
CREATE TRIGGER task_reminder_decisions_no_truncate BEFORE TRUNCATE ON task_reminder_decisions
             FOR EACH STATEMENT EXECUTE FUNCTION task_tables_immutable();
CREATE TRIGGER task_reminder_runs_no_change BEFORE UPDATE OR DELETE ON task_reminder_runs
             FOR EACH ROW EXECUTE FUNCTION task_tables_immutable();
CREATE TRIGGER task_reminder_runs_no_truncate BEFORE TRUNCATE ON task_reminder_runs
             FOR EACH STATEMENT EXECUTE FUNCTION task_tables_immutable();
