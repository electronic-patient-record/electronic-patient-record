# REST and FHIR conventions

- Native REST lives under `/api/…` (task-shaped, paginated, JSON). FHIR R4 lives under `/fhir/R4/…` (planned, IO-1).
- Errors: 401 missing or bad token; 403 policy denied (planned, SE-1); 404 unknown or hidden; 409 conflict (a signed
  note, a likely duplicate); 422 invalid content.
- Every read and write is audited (AU-1). A 403 is audited too.
- No endpoint deletes clinical data.
- New endpoints are pinned in OpenAPI and, for FHIR, in the CapabilityStatement (a census test keeps them equal).
