# Tellescope API documentation: lessons for this project

The public API reference ([docs.tellescope.com/api-documentation](https://docs.tellescope.com/api-documentation)) read
in full on 2026-11-17: 279,252 characters, in three parts. Vendor documentation; nothing was verified against the
running API. Direction only. The product overview is [`../tellescope/index.md`](../tellescope/index.md); this page is
about the API's conventions. Plan: [`plan.md`](../../../plan.md) §38. Tasks TL-11 to TL-14, and changes to TL-1, TL-3 and
TL-4.

## What the page holds

**It is shorter than its subject.** The published page documents models alphabetically from `AiConversations` to
`AutomationSteps` and ends mid-example, inside a task reminder of the `Create AutomationSteps` body. Endusers, forms,
tickets, calendar events, care plans and the webhook section are referred to but not on the page. What follows is what
it does hold:

- **Getting started.** A REST API with a TypeScript SDK; production `api.tellescope.com/v1` and a staging host,
  `staging-api.tellescope.com/v1`, for testing against the same API. A secret API key in an `apiKey` parameter or an
  `Authorization: API_KEY` header; end users (patients) send a bearer `authToken`; some endpoints are public. Users are
  the team; endusers are patients.
- **Common fields and lists.** Every record has `id`, `createdAt`, `updatedAt` and an organisation id. Lists page by
  `lastId` and `limit` (100 by default), sort `oldFirst` or `newFirst`, take `filter` and `search` objects, and
  `fromUpdated` (changed since a time) and `from`/`to` (created between). List-valued fields hold at most 1,000 items.
- **Updates.** Partial: an array field **appends** by default, an object field **merges** key by key, and
  `replaceObjectFields: true` overwrites; `dontSendWebhook: true` suppresses the webhook.
- **AI.** `AiConversations` record each message's status, token and cache-token counts, latency and credit charge.
  `AiDocuments` (a saved AI app or agent) are private to the creator and admins unless shared, read-only when shared;
  only an admin may set `runAsUserId`; an agent has `approverUserIds`, `maxCredits`, `autopilotEnabled` and
  `scheduleCron`. Saving a version appends an **immutable** `AiDocumentVersion` and answers **409** if `expectedVersion`
  is stale; versions are readable by exactly those who can read the document.
- **Analytics.** Saved frames visible by role or user, and an admin-only **custom aggregation** that runs a MongoDB
  aggregation on a named model.
- **API keys.** A key is shown once: "The returned key is not stored in Tellescope and cannot be retrieved later."
  Scopes (`form-ingestion` seen) restrict it to some endpoints, and setting one needs an admin. Admins list keys
  (metadata only, never the secret) and may delete any key in the organisation; only the description can be edited; a
  key outside the organisation is a 404.
- **Booking pages.** A title that must be unique; a public endpoint to validate a booking token; a one-time token with
  `expiresAt`; restrictions per appointment type by care team, state, hours and portal tags.
- **Automated actions.** The queue of pending automation work: `status`, `processAfter`, `cancelConditions`,
  `cancelReason`, `cancelledBy`, `externalId`, and an admin-only worker endpoint that processes one.
- **Automation steps.** `events`, `conditions` and `action`, with per-action `continueOnError` and
  `isBrandedWebhook`. Two rules: an event automation may not duplicate another's event, action and conditions; a
  list-item change may only touch the sub-fields its list declares. Actions can hold a webhook's `secret` and `headers`,
  and prescription data (`drugs`, `diagnoses`), in the step's own configuration.
- **Deletes.** Most models have `DELETE`, including automated actions and allergy codes.

**Not on the page:** error formats (only 404 and 409 appear), rate limits, idempotency, webhook payloads, signing and
retries, sessions and MFA, audit or access logs, PHI handling and encryption, data export.

## Assessment

The API is consistent and practical: one list shape everywhere, a staging host, keys shown once, admin-only scopes,
versioned AI configuration with a 409 on a stale save, and recorded cost and latency for every AI call. Those are good
practice and this project should match or keep them.

Its defaults are a record's risks. An update that **appends to arrays and merges objects unless told otherwise** turns
a correction into an accumulation. Hard **deletes** on most models, an admin **aggregation over any model**, a
**"send no webhook"** flag, **`continueOnError`** in automations, and **secrets kept inside automation configuration**
each trade safety or audit for convenience. None is documented as audited. The page is also incomplete, so a reader
cannot see the rules for the patient record itself.

## Compared with this project

| Convention | Tellescope API | This project | Lesson |
|---|---|---|---|
| Keys | Shown once; scopes need an admin; metadata only after; any admin may revoke | No integration keys yet | TL-1 should match this exactly |
| Secrets in configuration | A webhook's secret and headers live in the automation step | Secrets only from the environment or files | Any stored secret is write-only and never read back (TL-11) |
| Versioning | AI documents: immutable versions, 409 on stale `expectedVersion` | Every clinical record: append-only versions, 409 on a stale version | Keep; the same rule for task rules (TL-3) |
| AI governance | Approvers, credit cap, autopilot flag, cron; tokens, latency and cost per message | A boundary for AI drafts planned (DO-6) | Approvers, off by default, a cap, and a cost record per call (TL-12) |
| Update semantics | Arrays append, objects merge, unless `replaceObjectFields` | Whole new versions; no PATCH | Keep whole versions; if PATCH comes, absent and `null` differ and nothing appends implicitly (TL-13) |
| List limits | 1,000 items per list field | Text limits per field; no general list limit | A stated maximum for every list in the OpenAPI document (TL-13) |
| Deletes | Hard deletes on most models | No delete path for clinical data, by trigger | Keep |
| Ad hoc queries | Admin aggregation over any model | Reads only through routes that apply policy and audit | Keep: not adopted |
| Event suppression | `dontSendWebhook` | Not possible | Keep (TL-6) |
| Partial failure | `continueOnError` per action | A write and its audit row are one transaction | A task rule's actions all happen or none (TL-14) |
| Duplicates | An identical automation is refused | Not applicable yet | Refuse a duplicate task rule when rules load (TL-14) |
| Lists and sync | `lastId`, `limit`, `sort`, `fromUpdated` | Cursor pages; FHIR `_lastUpdated` partly | The changes feed (TL-5) |
| Environments | A staging host | None documented | The sandbox (TL-9) |

## Ideas, ranked

| # | Idea | Effort | Clinical | Task |
|---|---|---|---|---|
| 1 | Integration keys as Tellescope does them: shown once, never stored in clear, scopes set by an administrator, metadata-only lists, revocable by any administrator, every use and change audited | S/M | no | TL-1 (extended) |
| 2 | A task rule's actions all happen or none, in one transaction; an identical rule is refused when rules load | S | yes | TL-14 |
| 3 | Write-only secrets: any secret the service stores is shown once and never returned; a census finds no secret-named field in any answer | S | no | TL-11 |
| 4 | AI governance for DO-6: named approvers, off by default, a spending cap, and model, version, tokens, latency and cost recorded per call, never the content in a log | S/M | yes | TL-12 |
| 5 | Explicit update semantics if PATCH ever comes (absent differs from `null`, nothing appends implicitly), and a stated maximum for every list in the OpenAPI document | S | yes | TL-13 |
| 6 | A cancelled reminder records who or what cancelled it (`cancelledBy`) beside why | S | yes | TL-4 (extended) |

## Considered and not adopted

- **Implicit append and merge on update.** A clinical correction must replace, and say so.
- **Hard deletes.** The record is append-only by trigger; nothing here changes that.
- **An aggregation over any model**, even for administrators: it bypasses consent, role views and audit. Analytics go
  through views that apply them.
- **`continueOnError`** for anything that writes to the record.
- **Secrets kept readable inside configuration.**

## Not verified

- Read from the published page only, once; the page itself is truncated, so models after `AutomationSteps` (endusers,
  forms, tickets, webhooks among them) were not documented there to read.
- Nothing was tried against the API or its staging host.

## Sources

- [Tellescope API documentation](https://docs.tellescope.com/api-documentation), parts at characters 0, 100,000 and
  200,000 of 279,252
