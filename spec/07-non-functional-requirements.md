## 7. Non-Functional Requirements

Targets, not measurements. Nothing here has been measured yet. Where the field stands on each is in
[`benchmark.md`](benchmark.md) (2026-10-30): NFR-2 has first numbers but not for a chart or a typeahead ([BENCHMARKS.md](../BENCHMARKS.md), OS-5), NFR-3's stateless tier is true of the UI only when `EPR_SESSION_REDIS_URL` is set
(OS-8, built, not run behind two replicas), NFR-6's request limit is built per process only (OS-2), NFR-8's OTLP is not built and waits on OQ-OS4, though a labelled request-duration histogram is served at `/metrics` (OS-6).

- **NFR-1 Safety.** Hazard log before shipping (DCB0129 shape); no automatic patient link on missing evidence.
- **NFR-2 Performance.** Chart open p95 ≤ 1 s; patient search/typeahead p95 ≤ 200 ms; order sign p95 ≤ 2 s; FHIR
  read p50 ≤ 10 ms.
- **NFR-3 Availability.** 99.9%; stateless app tier; PostgreSQL replication; RPO ≤ 5 min, RTO ≤ 4 h; restoration within 72 h
  (proposed HIPAA rule). Downtime snapshot ([plan](../plan.md) §6.5).
- **NFR-4 Accessibility.** WCAG 2.2 AA; axe on every page; keyboard-only operation of critical tasks.
- **NFR-5 i18n.** Ten locales, `<language>-<region>` names ([locales](locales.md)); times UTC with the user's zone at the edge.
- **NFR-6 Security.** Login enforced by default; TLS 1.2+ (prefer 1.3); AES-256 at rest; MFA; per-tenant keys;
  no secrets in code or images; non-root containers; cargo-deny, SBOM, ZAP.
  **Request limits (OS-2, first slice).** Every authenticated request counts against its token subject in a
  fixed one-minute window held in the process: `EPR_RATE_LIMIT_PER_MINUTE` (default 1200, about 20 a second) for
  everything, and `EPR_RATE_LIMIT_COMPARE_PER_MINUTE` (default 120) for `POST /api/patients/compare`, the call
  that scores a candidate against the register. A value outside 1 to 1,000,000 stops the service starting. Past a
  limit the answer is 429 with `Retry-After` (seconds to the end of the window). The first refusal of a subject in
  a window is an audit row (`Execute`, outcome 4, `denied: rate-limit`, no body); the others are only counted in
  `epr_rate_limited_total`, so a flood cannot become a flood of audit writes. A limit is a refusal a clinician could
  meet, so the defaults sit far above clinical use (a test replays a clinician at five requests a second for ten
  minutes and the compare route at one call every two seconds, and neither trips) and the hazard is H-039.
  **Not built:** a limit keyed by client address (the service sits behind the UI's server, so the address is
  one caller for everybody); a sign-in lockout (sign-in is the identity provider's, not this service's); a store
  shared between replicas (each replica counts alone, so N replicas allow N times the limit); per-route limits other
  than compare; a way to lift a limit for one subject without a restart.
- **NFR-7 Auditability.** Append-only audit, tamper-evident; reads included. **When a write's audit row cannot be written, the write is refused** ([audit](audit/index.md)).
- **NFR-8 Observability.** `tracing` JSON logs, Prometheus metrics, OTLP.
- **NFR-9 Determinism.** Matching is pure and total; every score has a breakdown.
- **NFR-10 Usability.** SUS ≥ 75 in summative tests; click and time budgets per critical task; alert override rate tracked.
- **NFR-12 Support only.** No score, rank or priority computed here may reduce what a patient is offered, hide a patient from a list or delay their care; a score asks a person to look. A tripwire test fails when a migration names a column like `score`, `rank`, `deprioritise`, `suppress`, `withhold` or `overbook` without a written reason. It reads names, not behaviour, so it is not proof (PA-3).
- **NFR-11 Supply chain.** Pinned toolchain; Dependabot; licences allow-listed; SBOM on release.
