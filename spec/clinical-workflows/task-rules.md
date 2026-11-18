# Task rules

Status: the rules and the store built 2026-11-17 (TL-3, TL-4, TL-14); no route, screen or event hook yet, so nothing
makes a task in a running service. From [`../lessons/tellescope/index.md`](../lessons/tellescope/index.md). Hazard
H-059.

## What a rule is

A rule, held as versioned data (`electronic-patient-record-domain/src/task_rules.toml`, read and checked by
`task_rules.rs`), says: when **an event** happens to a patient's record (a result filed or signed off, an observation
out of range or entered in error, a referral stepped or closed) and **its conditions** hold (each a fact of the event,
one of some values), **a team** (a role) gets **a task**, due after **working days** on the site calendar, with
**reminders** some working days before, and **the events that cancel it** while open.

**A rule creates a task and nothing else.** It never messages a patient and never changes the record. The file has no
place for another action; an unknown key stops it loading.

**Approval.** A rule with no approver and day is a draft: the dry run shows what it would make, and nothing is made.
Every rule built in is a draft until it is decided who approves one (OQ-TL1).

**Checks when the file is read:** versions of a rule run 1, 2, 3 ... with none missing; approver and day both or
neither; a condition only on a fact its event carries, with values; due in 0 to 60 working days; at most five distinct
reminders within the due period; never cancelled by its own event; no two rules the same event, conditions and task
(TL-14).

## Working days

The event's day is the site's calendar day it fell on (`site::day_in`). Saturday, Sunday and the site's holidays are not
working days. Due in 0 working days is that day, or the next working day if it is not one. A reminder N working days
before the due day is never set before the event's day.

**The site's holidays** are `EPR_SITE_HOLIDAYS`: calendar days `YYYY-MM-DD`, separated by commas or spaces, at most 200;
unset or blank is none (weekends are still not working days). Checked once at start-up with the time zone
(`site::init`): a value that is not a calendar day, such as `25/12/2026` or `2026-02-30`, stops the service, because a
due day counted through a forgotten holiday is a day early and nobody would know why. The built-in rules are checked
then too, so a rules file that does not load stops the service rather than the first result filed.

## Events raised

| Event | Raised by | Facts | Source |
|---|---|---|---|
| `result-filed` | `results_store::create`, a result filed at version 1 that is not entered in error | `test` (the name as reported); `abnormal`, `true` for an abnormal or critical flag, `false` for a normal one, and absent with no flag (silence is not normal) | `result`, the result's id |
| `result-signed-off` | `results_store::sign_off`, a new sign-off only (signing again raises nothing) | as filed, from the version signed | `result` |
| `observation-out-of-range` | `observations_store::create`, an observation outside its reference or critical range; `update`, a version out of range when the last was not (a second save of the same abnormal value raises nothing) | `code` (LOINC); `flag` (`critical-low`, `low`, `high`, `critical-high`) | `observation`, its id |
| `observation-entered-in-error` | `observations_store::update`, the version that marks it entered in error | `code` and `flag` as they were before, so the tasks the value made can be cancelled | `observation` |
| `referral-stepped` | `referrals_store::request` (a step into `requested`) and `advance`, every step that does not end the referral | `state` (the state reached); `priority` (`routine`, `urgent`, `asap`, `stat`) | `referral`, its id |
| `referral-closed` | `referrals_store::advance`, a step into an end state (`declined`, `completed`, `cancelled`, `redirected`) | `state` | `referral` |

Raising an event (`tasks_store::raise`) first cancels the source's open tasks that the event cancels, then makes what
the approved rules call for, so a new event replaces what an older one asked and never cancels a task it has just made.
Both run on the write's own connection; the results routes pass their transaction, which also holds the audit row.

## The store

`task_versions` (append-only, migration 40): version 1 as a rule made it (`open`), version 2 `done` or `cancelled`
with the reason and who: a person, or `event:<word>`. `task_current` is the latest of each.

- **All or nothing (TL-14).** Tasks are made and cancelled inside the transaction of the write that raised the event,
  each with an audit row (`Task/<id>`, the rule and version, the team, the source's kind, never its content); a rolled
  back write leaves no task.
- **Cancellation.** An event a rule names cancels that rule's open tasks from the same source record only.
- **Closing.** A person closes a task as done or cancelled, with a reason (1 to 500 characters), at the version they
  read; a stale version is a conflict; a closed task cannot be closed again.
- **Reminders (TL-4).** Each reminder is decided once (`task_reminder_decisions`: sent, or skipped with the reason);
  each run of the sender is recorded (`task_reminder_runs`). A reminder whose day fell after the last run and before
  today was missed while nothing ran: its rule says `send-once` (sent once, now) or `skip` (recorded with why). A
  reminder puts the task on its team's list as reminded; nothing goes to a patient.
- **The sender** (`task_reminders.rs`) runs every `EPR_TASK_REMINDER_MINUTES` (1 to 1440; a bad value stops the
  start). Unset, it is off and the start logs a warning: no reminder is decided, so none shows on a list. Each run takes
  the reminders whose day has come on the site's today, decides each (sent, or skipped with the reason), and records
  the run; one already decided, by another replica or an earlier run, is left. Each decision is its own write, so a
  failure part way leaves the rest for the next run. The log line carries counts only. A team's list shows each task's
  `reminded_on`, the latest day a reminder was sent, or null.

## Routes

| Route | Who | What |
|---|---|---|
| `GET /api/tasks?team=<role>` | staff who hold the team's role | the team's open tasks, due first, up to 200, with `more` |
| `POST /api/tasks/{task_id}/close` | staff who hold the task's team role | `{expected_version, status, reason}`: done or cancelled, with a reason, at the version read |
| `POST /api/tasks/dry-run` | a clinician, the on-duty clinician, the Clinical Safety Officer | `{event, facts, day?}`: what the service's rules would make, drafts marked `approved: false`; writes nothing |

Policy resource `Tasks`, not clinical: a task is a pointer, and the record opens through the patient's routes. Every
list, close and dry run is audited with ids and counts; a refused list or close is audited too. **Assumed, awaiting the
maintainer (OQ-TL3):** a team is a role, so a task for `clinician` is for whoever holds that role, at any site and on
any care team. The service starts only if every built-in rule's team is a role word.

## Not built

An observation with no value, no range, or a range that
makes no sense has no flag and raises nothing (never read as normal or abnormal); a result's later versions (amended, corrected, entered in error) raise nothing yet, so a task made
for a result later entered in error stays open until a person closes it; results from an HL7 feed (none is built);
an alert beyond the list (nobody is paged or messaged when a reminder is sent); a site's own rules file (only the built-in
catalogue runs, and every rule in it is a draft); a screen; who approves a rule (OQ-TL1); teams finer than a role (OQ-TL3).
