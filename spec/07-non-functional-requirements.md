## 7. Non-Functional Requirements

Targets, not measurements. Nothing here has been measured yet.

- **NFR-1 Safety.** Hazard log before shipping (DCB0129 shape); no automatic patient link on missing evidence.
- **NFR-2 Performance.** Chart open p95 ≤ 1 s; patient search/typeahead p95 ≤ 200 ms; order sign p95 ≤ 2 s; FHIR
  read p50 ≤ 10 ms.
- **NFR-3 Availability.** 99.9%; stateless app tier; PostgreSQL replication; RPO ≤ 5 min, RTO ≤ 4 h; restoration within 72 h
  (proposed HIPAA rule). Downtime snapshot ([plan](../plan.md) §6.5).
- **NFR-4 Accessibility.** WCAG 2.2 AA; axe on every page; keyboard-only operation of critical tasks.
- **NFR-5 i18n.** Ten locales, `<language>-<region>` names ([locales](locales.md)); times UTC with the user's zone at the edge.
- **NFR-6 Security.** Login enforced by default; TLS 1.2+ (prefer 1.3); AES-256 at rest; MFA; per-tenant keys;
  no secrets in code or images; non-root containers; cargo-deny, SBOM, ZAP.
- **NFR-7 Auditability.** Append-only audit, tamper-evident; reads included.
- **NFR-8 Observability.** `tracing` JSON logs, Prometheus metrics, OTLP.
- **NFR-9 Determinism.** Matching is pure and total; every score has a breakdown.
- **NFR-10 Usability.** SUS ≥ 75 in summative tests; click and time budgets per critical task; alert override rate tracked.
- **NFR-12 Support only.** No score, rank or priority computed here may reduce what a patient is offered, hide a patient from a list or delay their care; a score asks a person to look. A tripwire test fails when a migration names a column like `score`, `rank`, `deprioritise`, `suppress`, `withhold` or `overbook` without a written reason. It reads names, not behaviour, so it is not proof (PA-3).
- **NFR-11 Supply chain.** Pinned toolchain; Dependabot; licences allow-listed; SBOM on release.
