//! FHIR R4 `AuditEvent` for an audit row (task AU-2), shaped after IHE BALP.
//!
//! One row becomes one `AuditEvent`: `type` is the REST operation; `action`, `recorded` and
//! `outcome` are the row's; the agent is the token `sub` as an identifier (not a name); the patient,
//! when the row named one, is an `entity` reference; the purpose of use is a `purposeOfEvent` in the
//! local system `urn:epr:purpose-of-use` (not the v3 ActReason codes: the mapping is not made);
//! break-the-glass is an extension. `detail` is not carried: it is free structure that belongs to
//! this service, and the JSON endpoint is where an auditor reads it.

use crate::audit::AuditRow;
use fhir::r4::resources::Resource;
use fhir::r4::validate::Validate;
use serde_json::json;

/// The system for purpose-of-use codes.
pub const PURPOSE_SYSTEM: &str = "urn:epr:purpose-of-use";
/// The extension that flags a break-the-glass access.
pub const BREAK_GLASS_EXTENSION: &str = "urn:epr:break-glass";

/// The row as a FHIR `AuditEvent`, validated by the typed R4 model.
///
/// # Errors
///
/// A message when the typed model rejects the result (a bug here, or a value it will not take).
pub fn to_fhir(row: &AuditRow) -> Result<Resource, String> {
    let mut agent = json!({
        "who": { "identifier": { "value": row.agent } },
        "requestor": true,
    });
    if let Some(address) = &row.network_address {
        agent["network"] = json!({ "address": address, "type": "2" });
    }
    let mut out = json!({
        "resourceType": "AuditEvent",
        "id": row.id.to_string(),
        "type": {
            "system": "http://terminology.hl7.org/CodeSystem/audit-event-type",
            "code": "rest",
            "display": "Restful Operation",
        },
        "action": row.action,
        "recorded": row.at,
        "outcome": row.outcome.to_string(),
        "agent": [agent],
        "source": { "observer": { "display": "electronic-patient-record" } },
    });
    let map = out.as_object_mut().ok_or("not an object")?;
    if let Some(what) = &row.resource {
        map.insert(
            "subtype".into(),
            json!([{ "system": "urn:epr:audited-resource", "code": what }]),
        );
    }
    if let Some(patient) = &row.patient {
        map.insert(
            "entity".into(),
            json!([{
                "what": { "reference": patient },
                "type": {
                    "system": "http://terminology.hl7.org/CodeSystem/audit-entity-type",
                    "code": "1",
                    "display": "Person",
                },
            }]),
        );
    }
    if let Some(purpose) = &row.purpose {
        map.insert(
            "purposeOfEvent".into(),
            json!([{ "coding": [{ "system": PURPOSE_SYSTEM, "code": purpose }] }]),
        );
    }
    if row.break_glass {
        map.insert(
            "extension".into(),
            json!([{ "url": BREAK_GLASS_EXTENSION, "valueBoolean": true }]),
        );
    }
    let resource: Resource = serde_json::from_value(out).map_err(|e| e.to_string())?;
    let issues = resource.validate();
    if issues.is_empty() {
        Ok(resource)
    } else {
        Err(format!("the FHIR model rejects the result: {issues:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> AuditRow {
        AuditRow {
            id: 42,
            at: "2026-10-21T09:00:12.123456Z".into(),
            action: "R".into(),
            outcome: 0,
            agent: "607b683d-fd04-4a73-ba47-f4d05df3d046".into(),
            patient: Some("Patient/528e5594-a62f-4883-bb06-620815c5d6a3".into()),
            resource: Some("Patient".into()),
            purpose: Some("treatment".into()),
            break_glass: false,
            network_address: None,
            detail: Some(json!({ "secret": "never carried" })),
        }
    }

    fn json_of(r: &AuditRow) -> serde_json::Value {
        serde_json::to_value(to_fhir(r).unwrap()).unwrap()
    }

    #[test]
    fn a_row_becomes_a_valid_audit_event_without_its_detail() {
        let j = json_of(&row());
        assert_eq!(j["resourceType"], "AuditEvent");
        assert_eq!(j["id"], "42");
        assert_eq!(j["action"], "R");
        assert_eq!(j["outcome"], "0");
        assert_eq!(j["recorded"], "2026-10-21T09:00:12.123456Z");
        assert_eq!(
            j["agent"][0]["who"]["identifier"]["value"],
            "607b683d-fd04-4a73-ba47-f4d05df3d046"
        );
        assert_eq!(
            j["entity"][0]["what"]["reference"],
            "Patient/528e5594-a62f-4883-bb06-620815c5d6a3"
        );
        assert_eq!(j["purposeOfEvent"][0]["coding"][0]["code"], "treatment");
        assert!(!j.to_string().contains("never carried"));
        assert!(j.get("extension").is_none());
    }

    #[test]
    fn a_sparse_row_omits_what_it_lacks_and_break_glass_is_flagged() {
        let mut r = row();
        r.patient = None;
        r.purpose = None;
        r.resource = None;
        r.break_glass = true;
        r.action = "E".into();
        r.outcome = 4;
        r.network_address = Some("10.0.0.7".into());
        let j = json_of(&r);
        assert!(j.get("entity").is_none() && j.get("purposeOfEvent").is_none());
        assert_eq!(j["extension"][0]["url"], BREAK_GLASS_EXTENSION);
        assert_eq!(j["outcome"], "4");
        assert_eq!(j["agent"][0]["network"]["address"], "10.0.0.7");
    }

    #[test]
    fn every_outcome_and_action_the_table_allows_is_accepted_by_the_model() {
        for action in ["C", "R", "U", "D", "E"] {
            for outcome in [0, 4, 8, 12] {
                let mut r = row();
                r.action = action.into();
                r.outcome = outcome;
                assert!(to_fhir(&r).is_ok(), "{action} {outcome}");
            }
        }
    }
}
