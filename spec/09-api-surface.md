## 9. API Surface

| Path | Method | Status |
|---|---|---|
| `/_health`, `/_ping`, `/_readiness` | GET | loco built-ins; public |
| `/api/patients/compare` | POST | built; body `{a, b}` of `Patient`; returns `MatchResult` |
| `/api/patients`, `/api/patients/{id}` | CRUD | planned (ID-3) |
| `/api/audit` | GET | planned (AU-2) |
| `/fhir/R4/metadata` | GET | planned (IO-1) |
| `/fhir/R4/{Type}` and `/{id}`, `_history`, `$everything`, `$export`, `Patient/$match`, `$summary` | | planned (RC-, IO-) |
| `/.well-known/smart-configuration` | GET | planned (IO-2) |

All routes except the three probes require a valid bearer token (built). A census test will keep the mounted FHIR routes
and the CapabilityStatement equal (IO-1).

### Error bodies

Every refusal and failure is built by `api_error::ApiError`, never by hand (`tests/error_census.rs`):

- On `/api`: JSON `{"error": message}`, with extra fields where a client acts on them (`current_version`,
  `replaced_by`, `break_glass_offer`, `candidates`, `candidates_cut`).
- A refused record is 422 `{"errors": [reason, …]}`.
- On `/fhir/`: an `OperationOutcome` with one issue, served as `application/fhir+json`. The issue codes are `login`,
  `forbidden`, `not-found`, `conflict`, `invalid` and `exception`.
- The authentication (401) and route-gate (403) middleware answer the same way, by path, and never as plain text.
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
