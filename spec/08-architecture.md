## 8. Architecture

```
UI (SvelteKit + BFF)  ──►  Service (loco.rs / Axum)  ──►  Matcher (pure, crates.io dependency)
                                │   /api/…  native REST
                                │   /fhir/R4/…  FHIR facade (built: src/controllers/fhir.rs); /fhir/R5/… read-only, derived (fhir_r5.rs)
                                │   policy engine (built: src/policy.rs, authz.rs) → audit writer (built; every-access coverage not verified)
                                ├──► PostgreSQL 18 (resource_version, search index tables, audit, outbox)
                                ├──► tantivy (patient typeahead)
                                └──► TerminologyService (Snowstorm / Ontoserver; embedded fixtures in tests)
```

Dependency direction is strictly downward. Events are to leave through a transactional outbox (subscriptions, webhooks,
CDS Hooks, HL7 v2 adapter). Multi-tenancy is to be by `tenant_id` + PostgreSQL row-level security.
**Not built as of 2026-10-30:** the outbox (OS-3), tantivy, the terminology service adapters beyond fixtures, `tenant_id`
and row-level security; none is in `Cargo.toml` or a migration. The diagram shows the target; the
[benchmark](benchmark.md) shows what comparable systems run today. Authentication: OIDC upstream,
PASETO v4.public verified offline; SMART scopes map onto the same policy engine (OQ-3). Detail and rationale:
[`../plan.md`](../plan.md) §6. Built (checked against `src/` on 2026-10-30): the UI, the service with the auth guard, the policy engine, consent, break-glass, the audit chain, the FHIR R4 facade and read-only R5 view, the ADT feed, the matcher.
