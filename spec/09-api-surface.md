## 9. API Surface

| Path | Method | Status |
|---|---|---|
| `/_health`, `/_ping`, `/_readiness` | GET | loco built-ins; public |
| `/api/patients/compare` | POST | built; body `{a, b}` of `Patient`; returns `MatchResult` |
| `/api/patients`, `/api/patients/{id}` | CRUD | planned (ID-3) |
| `/api/audit` | GET | planned (AU-2) |
| `/fhir/R4/metadata` | GET | built; a CapabilityStatement whose resource types and interactions are generated from the mounted routes (parses as FHIR R4); the notes and search parameters on each are written by hand |
| `/fhir/R5/metadata`, `/fhir/R5/Patient`, `/fhir/R5/Patient/{id}`, `/fhir/R5/AllergyIntolerance`, `/fhir/R5/Observation` | GET | built (IO-16); a read-only R5 view derived from the R4 answer, base R5 with no UK Core or US Core claim; see [interoperability](interoperability/index.md) "FHIR R5" |
| `/fhir/R4/{Type}` and `/{id}`, `_history`, `$everything`, `$export`, `Patient/$match`, `$summary` | | planned (RC-, IO-) |
| `/.well-known/smart-configuration` | GET | planned (IO-2) |

All routes except the three probes require a valid bearer token (built). Tests keep the mounted FHIR routes and the CapabilityStatement equal (IO-1): the route list is generated, and `the_capability_statement_lists_exactly_the_mounted_routes`, `…_notes_name_exactly_the_mounted_types_that_have_any` and `…_lists_the_search_parameters_the_code_reads` check the generator and the hand-written part.

### Referrals (CA-3)

`GET`/`POST /api/patients/{id}/referrals`, `GET …/referrals/{referral_id}` (every version, oldest first) and `POST …/referrals/{referral_id}/events` (`{event, expected_version, note}`). Resource `Referral`: clinician, nurse and allied health read, send and step; the patient and a proxy only read; every other role is refused by the route gate. The state table (`src/referral.rs`) decides which event follows which; a refused step is 422, a stale `expected_version` is 409, an id under another patient is a miss. The sender of a referral acts as the referrer and any other permitted subject as the recipient (OQ-PC2, assumed): the referrer can only request a cancellation, and only another subject confirms it. Each version also carries `next`: the events the state table allows from that state, with who sends each and whether it needs a note (empty in an end state), so a screen holds no copy of the table. Audit rows carry ids, versions, states and the role, never the reason, the question or a note.

### Inbound referral messages and their queue (PC-2)

`POST /api/referral-messages` (`{referral_ref?, event?, note?, message}`) is an outcome, decline or interim note from a recipient system. It moves the referral it names, or is queued; the answer is 200 either way (`{result: "moved", referral, version, state}` or `{result: "queued", queue_id, kind}`) and never repeats the message. A queued message has one of three kinds: `no-id`, `unknown-id`, `refused-by-table` (the state table, the wrong sender, a missing note or an unknown event). `GET /api/referral-queue/count`, `GET /api/referral-queue` (open entries, no text) and `GET /api/referral-queue/{id}` (with the note and message as received) and `POST /api/referral-queue/{id}/clear` (`{how: "matched" | "dismissed", referral_id?, reason}`; a hand match records the link and changes nothing in the referral; once only, 409 after). Resource `ReferralQueue`: **assumed, awaiting the maintainer (OQ-PC4)**: the integrator role sends, the clinical administrator reads and clears; no care relationship applies. Audit rows carry ids, kinds, states and the way a message was cleared, never the message, a note or a reason.

### The referrer's worklist (PC-2, assumed: OQ-PC5)

`GET /api/referrals/worklist` lists the referrals the caller sent that wait for them (`{items: [{referral_id, patient_id, version, state, recorded_at}], limit, more, site_time_zone}`), oldest first, with no recipient, reason, question or note; `GET /api/referrals/worklist/count` is `{open}`; `POST /api/referrals/{referral_id}/acknowledge` (`{version}`) says the sender has seen that version: 200 once, 409 for a stale version or a repeat, 404 for a referral the caller did not send, 422 when the current state does not wait. A referral also waits when it has gone quiet for longer than the configured period for its priority (`EPR_REFERRAL_TIMEOUTS`, no default; PC-3, assumed OQ-PC3): `why` is `no-response` (still `requested`) or `no-outcome` (`accepted`, `appointment-booked` or `interim-report`). Such an item cannot be acknowledged (422) and leaves the list when the referral moves. The list answers `untimed: {no_response: [...], no_outcome: [...]}`, the priorities with no period, so a screen can say that a referral of that priority will not appear. A referral waits as a reply (`why: reply`) when its current version is declined, redirected, no-show, deferred, information-requested, interim-report or completed and that version is not acknowledged; a later version is a new item. Resource `ReferralWorklist` (clinician, nurse, allied health): no care relationship is checked at the list, because it holds only the caller's own referrals and no text; the referral itself opens through `/api/patients/{id}/referrals`, which applies the full record-level decision. Audit rows carry ids, counts and versions.

### Lists that are cut short (PR-1)

The feed-refusal list, the break-glass review list and the referral queue list return the oldest 50, a `limit`, and `more`: the store fetches one row beyond the limit and `more` is `true` when it arrives (audit search answers `truncated` the same way). The break-glass and feed screens say the list is cut when `more` is true; the referral queue has no screen yet. A screen talking to an older service that sends no `more` treats a full list as cut. The field is added; no field is renamed. The "showing 50 of N" wording from the proposal is not built: the screens say "this is the oldest {n}, more are waiting". A queue shown oldest first hides the newest entry once 50 are waiting, so this is a safety rule for the queues and a courtesy for the rest.

### Idempotent creates (OF-1)

A `POST` under `/api` may carry `Idempotency-Key: <1 to 128 of letters, digits, . _ : ->`. The request runs once per
(token subject, key). A retry with the same method, path and body returns the first answer, status and body, with
`Idempotent-Replayed: true`, and writes nothing; the replay is audited (resource `IdempotentReplay`) and is refused
(500) if that row cannot be written. The same key with a different method, path or body is 422; a key whose first
request is unfinished, or ended without an answer being kept, is 409. A key that is not well formed is 400. Only a
success is kept: a refusal or failure releases the key so the corrected request can use it. Keys are per subject and
expire after 24 hours (`idempotency_keys`, purged hourly); the stored answer can hold patient data, so it is the one
place a copy of a record is kept outside the append-only tables, and it is the only table here that deletes. If the
key cannot be checked the request is refused, not run unguarded. A request without the header behaves as before.
`/fhir/` creates are not covered. Hazard H-043.

### Error bodies

Every refusal and failure is built by `api_error::ApiError`, never by hand (`tests/error_census.rs`):

- On `/api`: JSON `{"error": message}`, with extra fields where a client acts on them (`current_version`,
  `replaced_by`, `break_glass_offer`, `candidates`, `candidates_cut`).
- A refused record is 422 `{"errors": [reason, …]}`. A refused patient (create or update) also carries `error_codes`, one stable code per reason in the same order (`family_name_required`, `birth_date_invalid`, `nhs_number_placeholder`, `nhs_number_check_digit`, `identifier_incomplete`, and on update `temporary_identity_reason_required`), so a client can say it in the reader's language; `errors` stays the English text.
- On `/fhir/`: an `OperationOutcome` with one issue, served as `application/fhir+json`. The issue codes are `login`,
  `forbidden`, `not-found`, `conflict`, `invalid` and `exception`.
- The authentication (401) and route-gate (403) middleware answer the same way, by path, and never as plain text.
- A criterion of the wrong type on the audit routes (`outcome=x`, `checkpoint_id=x`) is the same JSON 400 as a bad date, not the framework's plain text.
- A 500 says `internal error`. The cause is logged and never sent.

### API versioning

The REST API lives at `/api` and is versioned by a request header, `Accepts-version`, never by the path. Do **not**
add `/api/v1`, `/api/v2` or any other version segment to a route. A new version of a route changes behaviour behind the
same path, chosen by the header.

- The header carries the version the client was written for. Naming the values and the default (what a request with no
  header gets) is part of the first change that needs a second version; until then there is one version and the header
  is not read.
- A version the server does not know is refused, not silently served as another one.
- The route table, the census test and the audit `resource_ref` use the unversioned path, so one route has one policy
  entry whatever its version.
- `/fhir/R4` is not this API's version: `R4` is the FHIR release the endpoints implement, fixed by that standard's
  base-URL convention. A different FHIR release would be a separate mount, not an API version.

**Not built:** reading the header, the list of versions, and the refusal of an unknown one. This is the rule, not code.

### OpenAPI document (OS-7)

`GET /api/openapi.json` (authenticated, the same policy row as `/fhir/R4/metadata`) and the committed copy
[`openapi.json`](../electronic-patient-record-service-with-rust/openapi.json) list every served path and method with its
authentication requirement and, for protected routes, the policy resource and action (`x-epr-resource`,
`x-epr-action`). The document is **derived** from `controllers::all_routes()` and `routes::TABLE`, never written by
hand, and `tests/openapi_census.rs` fails when the committed copy is stale, a served route is missing, a public route
lacks its marker, or a path carries a version segment.

**It describes error bodies only** (`components.schemas.Error`, `ErrorList`, `OperationOutcome`, referenced by every 401 and 403; unit tests check real `ApiError` bodies against the required keys and that every reference resolves). It does not describe any request body or success response, so a client cannot be generated from it; it is an inventory of the surface, not a contract. FHIR shapes are in `/fhir/R4/metadata`. The exceptions are `GET /metrics` (200 text, 404), the patient's own routes (`GET /api/me/patients` and its `access-log`, JSON or CSV), the feed refusal routes, the audit routes (search, export, verify), the break-glass routes, the hazard routes, the merge, unmerge and links routes, the photo routes, the allergy-statement route (`PUT …/allergy-statement`), the amendment routes (list, ask, step), the consent and relationship routes (list, create, update), the observation routes (list, create, update, versions), the referral routes (list, history, send, step), the inbound referral message and referral queue routes, the allergy routes (`GET`/`POST …/allergies`, `PUT …/allergies/{allergy_id}`) and the patient identity routes, `POST /api/patients`, `PUT /api/patients/{id}`, `GET /api/patients/{id}` and `GET /api/patients`: `Patient`, `Stored`, `CreatePatient`, `UpdatePatient`, `PatientView`, `PatientSearchResults` and the two 409 bodies are described, and unit tests compare their property names with the fields the real types serialise and read. Other body schemas are open work (OS-7).

