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

## Not built

The event hooks in the stores that raise each event; routes (a team's list, close, a dry run); the reminder sender's
loop; the site holiday list setting; a screen; who approves a rule (OQ-TL1).
