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
