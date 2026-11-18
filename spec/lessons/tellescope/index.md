# Tellescope: lessons for this project

Public-source research read 2026-11-17: the features page, the homepage, the integrations page and the public API
documentation. Vendor material is marketing; nothing here was verified by using the product, and no private
documentation was read. The trust centre returned no readable content. Direction only: nothing in this project was built
from it. Plan: [`plan.md`](../../../plan.md) §37. Tasks TL-1 to TL-10.

## What it is

- A patient-engagement and workflow platform for US digital-health companies, telehealth providers and clinics: "One
  platform for everything your patients and team need" ([tellescope.com](https://www.tellescope.com/)). It describes
  itself as patient relationship management (a CRM), not a record: it syncs with an EHR rather than being one; the API
  names Canvas and Healthie ([API documentation](https://docs.tellescope.com/api-documentation)).
- Features on its features page, as the page names them: patient engagement ("automated patient outreach"), EHR
  integration, patient intake ("secure and easy forms"), patient relationship management ("the entire patient journey in
  one hub"), a healthcare API, appointment scheduling, a communication platform "across all digital channels", workflow
  automation, and a white-label telemedicine platform; and, as "HIPAA compliant tools", email, video, task management,
  two-way texting, self-scheduling, forms ("intake information and consent before appointments"), file sharing, a CRM and
  telehealth ([features](https://www.tellescope.com/features)).
- Claims HIPAA compliance and an AICPA SOC 2 report on the homepage. No HITRUST claim was found. No pricing page; the FAQ
  describes an implementation fee plus per-user or per-patient fees, and implementation in "4-6 weeks"
  ([tellescope.com](https://www.tellescope.com/)).
- Integrations shown: Apple and Outlook calendars, OpenLoop (a provider network), ScriptSure (e-prescribing), two AI
  partners, with EHR, data, communication and payment filters whose entries did not load
  ([integrations](https://www.tellescope.com/integrations)); the API also names Stripe, Chargebee, Twilio, Iterable and
  GA4 ([API documentation](https://docs.tellescope.com/api-documentation)).

## How it is built (from the API documentation)

- A REST API (JSON) with a TypeScript SDK, a production host and a **staging host** for testing against the same API,
  secret API keys, **scoped** keys (one scope seen: `form-ingestion`), and end-user tokens for the portal.
- Records carry `id`, `createdAt`, `updatedAt` and an organisation id. Lists page by `lastId` and `limit` (100), and
  **`fromUpdated`** returns what changed since a time: an incremental sync for an integrator.
- **Webhooks** subscribe to create, update and delete per model, with a URL, headers and a secret; an API-key update can
  pass `dontSendWebhook: true` to send none.
- **Journeys** are the automation: **automation steps**, each `events` → `conditions` → `action`, chained by step id.
  Events include a form response, an outcome and a field change. Conditions include form and field values, journey
  state, day of month, office hours, time zone and an A/B split; **`cancelConditions`** stop a pending step. Delays have a
  unit and **`skipIfDelayPassed`**. A pending action is an **automated action** with `processAfter`, a status, and
  `cancelReason` and `cancelledBy` when stopped.
- Actions send email, SMS and calls from templates; complete or submit forms; create **tickets** (tasks) with an
  **assignment strategy**, priority, reminders and a due date offset that can **skip days of the week**; set fields and
  tags; call webhooks; charge through Stripe or Chargebee; start an AI summary or an AI voice and chat agent; route a
  prescription; and add notes (`isInternalNote`, a coded `noteCoding` of code, display and system).
- Access is by **care team** (`careTeamTags`, `forAssigned`, `limitedToCareTeam`), **access tags**, roles, admin-only
  endpoints, and **`limitedByState`** (a clinician's licensed state). AI documents are versioned, and saving over a stale
  `expectedVersion` answers 409.
- Not found in what was read: FHIR or HL7 interfaces, an audit or access log, a consent model beyond forms, a
  clinical-safety statement.

## Assessment

Tellescope is strong where this project is thin: what happens next, and who does it. Its automation model (an event,
conditions, a delay, an action, and conditions that cancel a pending action) is the most developed thing in it, and its
operations details are careful: working-day due dates, office hours, an assignment strategy, a recorded cancel reason,
a staging host, scoped keys, an incremental sync.

It is weak, or silent, where this project is strongest: the record itself. No audit of who read what was found; no FHIR;
nothing on consent beyond a form; nothing on safety. Several of its features would be hazards in a record: an automation
that messages a patient or changes a field with no person in the loop, an A/B split on patients, marketing analytics on a
care journey, and a flag that lets a caller change data without telling subscribers.

So the lessons are about workflow around the record, kept inside this project's rules: append-only, audited, consent
applied, and no automation that acts on a patient or the record without a person.

## Compared with this project

| Area | Tellescope | This project today | Lesson |
|---|---|---|---|
| Automation | Journeys: event, conditions, delay, action; cancel conditions; office hours | Referral timeouts (PC-3), suspicion pathways as data, an outbox; no general rules | Task rules that only create tasks (TL-3), with cancel and skip-if-passed (TL-4) |
| Tasks | Tickets with assignment strategy, priority, reminders, working-day due dates | Worklists for referrals and overdue reviews; no team inbox (CA-6) | Working-day due dates on the site calendar, assignment by team (TL-3) |
| One view of a patient | A timeline of messages, forms, notes and profile | The chart opens eight separate reads | A read-only, paged timeline over the append-only versions (TL-2) |
| Integrator access | Scoped API keys; staging host | Human tokens; the HL7 feed's own agent | Scoped, hashed integration tokens (TL-1); a documented sandbox (TL-9) |
| Incremental sync | `fromUpdated` plus `lastId` | Cursor pages; an outbox; FHIR `_lastUpdated` partly | A changes feed per append-only table (TL-5) |
| Webhooks | Per model, with a secret; a caller can suppress one | Outbox relay; subscriptions planned (IO-6) | Signed deliveries; never a caller-side "send nothing" for clinical writes (TL-6) |
| Booking | Booking pages with one-time tokens and restrictions by type | Scheduling not built (CA-4); one-time link tokens exist (`link_token`) | One-time booking links on the existing token rules (TL-7) |
| AI | AI summaries from chosen sources; an AI voice agent | A boundary for AI drafts planned (DO-6) | Every sentence of a summary linked to its source, a draft until a person files it (TL-8) |
| Jurisdiction | Access limited by the clinician's licensed state | Roles, care relationships, consent, purpose; no jurisdiction | An open question for US deployments (TL-10) |
| Audit | Not found | Every access audited, hash-chained | Keep |
| Standards | No FHIR or HL7 found | FHIR R4 facade, HL7 v2 feed | Keep |

## Ideas, ranked

| # | Idea | Why | Effort | Clinical | Task |
|---|---|---|---|---|---|
| 1 | Task rules as versioned data: an event and conditions create a task for a team, due after working days on the site's calendar, with reminders; they create tasks only, never message a patient or change the record; dry run and audit | The one thing an EPR's users lack most is "what is mine to do next", and rules that only make tasks keep a person in the loop | M | yes | TL-3 |
| 2 | Cancel conditions and skip-if-passed on every scheduled reminder, with the reason and who cancelled recorded | After downtime a backlog of stale reminders buries the real ones; a cancelled reminder says why | S | yes | TL-4 |
| 3 | Scoped integration tokens: hashed, named, scoped, expiring and revocable (axa 0.16's `api_tokens`), the token's name as the audit agent | Systems should not borrow a person's sign-in, and a token's reach should be one job | S/M | no | TL-1 |
| 4 | A patient timeline: one paged, read-only list over the append-only versions, each item with kind, author, time and a link, consent and role views applied per item, audited as one read | One place to see what happened, without eight reads and without a new store | M | yes | TL-2 |
| 5 | A changes feed per append-only table (`since` a cursor) for integrators | Lets a downstream system keep up without reading everything; versions are already the history | M | no | TL-5 |
| 6 | Signed webhook deliveries (IO-6), and no caller may suppress the event for a clinical write | A silent change is an audit gap | S | no | TL-6 |
| 7 | One-time, expiring booking links on the existing link-token rules, restricted by appointment type (CA-4) | Self-booking without an account; the token rules are built and tested | M | no | TL-7 |
| 8 | AI summary boundary (DO-6): every sentence linked to the record items it came from, a labelled draft, never filed without a person | Tellescope chooses a summary's sources; this project should also show them | M | yes | TL-8 |
| 9 | A documented sandbox: a synthetic dataset and a script to load it, for integrators and training | Tellescope gives integrators a staging host; ours have to build their own | S | no | TL-9 |
| 10 | Whether access must follow a clinician's licensed jurisdiction (US telehealth) | A real rule in some deployments; this project has no jurisdiction concept | decision | yes | TL-10 |

## Considered and not adopted

- **A/B splits in clinical automation.** Experimenting on patients is research and needs ethics approval and consent; it
  does not belong in a workflow rule.
- **Automations that message patients or change the record unattended**, including AI voice agents that book or answer
  for the service. Each is a hazard needing its own safety case; TL-3 rules create tasks only.
- **Marketing integrations on a care journey** (Iterable, GA4 event tracking). Sending care events to analytics vendors
  is a disclosure this project's privacy rules do not allow.
- **A "send no webhook" switch on writes.** Recorded as a lesson to avoid (TL-6).
- **Payments, subscriptions, e-commerce orders and shipping.** Outside a record's scope.
- **Built-in SMS, voice and video.** Better integrated through a dedicated service than built here; identity checks
  and consent to contact are the hard part, not the transport.

## Open questions

- **OQ-TL1:** who approves a task rule (TL-3) before it runs, and does a rule change need the Clinical Safety Officer?
- **OQ-TL4:** who manages integration tokens (TL-1)? Assumed 2026-11-18: the clinical administrator, any of whom may
  revoke any token. There is no system-administrator role.
- **OQ-TL3:** is a team a role? The routes assume so (2026-11-18): only a caller who holds the role lists or closes the
  team's tasks. A ward, a clinic or a named rota would need a team model the service does not have.
- **OQ-TL2:** must access follow a clinician's licensed jurisdiction in any target deployment (TL-10)? **Answered 2026-11-17 by the maintainer: yes.** Building it is TL-15.

## Not verified

- Everything above is from Tellescope's own public pages and API documentation, read once on 2026-11-17; the API
  document was read in three parts of a 279,000-character page, and some schemas were cut short.
- The trust centre returned no readable content, so the SOC 2 and HIPAA claims are the homepage's only.
- No customer report, review, incident or independent evaluation was looked for.

## Sources

- [Tellescope features](https://www.tellescope.com/features)
- [Tellescope homepage](https://www.tellescope.com/)
- [Tellescope integrations](https://www.tellescope.com/integrations)
- [Tellescope API documentation](https://docs.tellescope.com/api-documentation)
- [Tellescope trust centre](https://trust.tellescope.com/) (no readable content)
