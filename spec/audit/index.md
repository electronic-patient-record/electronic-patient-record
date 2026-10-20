# Audit

One FHIR AuditEvent (IHE BALP shape) per access, reads included: agent, patient, entity, action C/R/U/D/E, outcome, purpose of use, network address, break-glass flag. Append-only and hash-chained with a keyed MAC. Patients can see who accessed their record. Tasks AU-1, AU-2, SE-6. **Built today:** the append-only table, the hash-chained writer and a chain verifier in the service, and the compare endpoint writes one row per call. **Not built:** a verify endpoint, audit of denied requests, any access to stored patient data (none exists yet), the patient-visible log.

Planned; nothing here is built yet unless stated. Source: [`../../plan.md`](../../plan.md).
