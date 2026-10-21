# Audit

One FHIR AuditEvent (IHE BALP shape) per access, reads included: agent, patient, entity, action C/R/U/D/E, outcome, purpose of use, network address, break-glass flag. Append-only and hash-chained with a keyed MAC. Patients can see who accessed their record. Tasks AU-1, AU-2, SE-6. **Built today:** the append-only table, the hash-chained writer, a chain verifier, and `GET /api/audit/verify` (auditor only; 409 when the chain is broken; each check is itself audited). Every patient and consent route writes a row, and so does each refusal by the route gate or the policy engine. **Not built:** detection of removed newest rows (needs an external anchor), audit of 401/400/422 responses, one transaction for a write and its audit row, a search API over the log (AU-2), BALP/FHIR `AuditEvent` output, the patient-visible log.

Planned; nothing here is built yet unless stated. Source: [`../../plan.md`](../../plan.md).
