## 8. Architecture

```
UI (SvelteKit + BFF)  ──►  Service (loco.rs / Axum)  ──►  Matcher (pure, crates.io dependency)
                                │   /api/…  native REST
                                │   /fhir/R4/…  FHIR facade (planned)
                                │   policy engine → audit writer on every access (planned)
                                ├──► PostgreSQL 18 (resource_version, search index tables, audit, outbox)
                                ├──► tantivy (patient typeahead)
                                └──► TerminologyService (Snowstorm / Ontoserver; embedded fixtures in tests)
```

Dependency direction is strictly downward. Events leave through a transactional outbox (subscriptions, webhooks,
CDS Hooks, HL7 v2 adapter). Multi-tenancy by `tenant_id` + PostgreSQL row-level security. Authentication: OIDC upstream,
PASETO v4.public verified offline; SMART scopes map onto the same policy engine (OQ-3). Detail and rationale:
[`../plan.md`](../plan.md) §6. Built at M0: the UI shell, the service shell with the auth guard, the matcher.
