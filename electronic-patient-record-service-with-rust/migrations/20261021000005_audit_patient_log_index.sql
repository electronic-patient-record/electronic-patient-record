-- Migration: an index so the patient-visible access log (`audit::patient_log`) reads one patient's
-- rows without scanning the audit table. Audit rows record `Patient/<id>` in the case the caller
-- used, so the index is on the lower-cased value.

CREATE INDEX IF NOT EXISTS audit_events_patient_log_idx
                 ON audit_events (lower(patient_ref), id DESC);
