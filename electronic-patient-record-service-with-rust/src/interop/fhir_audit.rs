//! FHIR R4 `AuditEvent` for an audit row (task AU-2), which claims an IHE BALP profile in `meta.profile`
//! only when [`violations`] finds nothing (task PX-4). A row that cannot meet the profile is exported
//! all the same, without the claim: see `spec/audit/index.md`.
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
use serde_json::{Map, Value, json};
use std::net::IpAddr;

/// The system for purpose-of-use codes.
pub const PURPOSE_SYSTEM: &str = "urn:epr:purpose-of-use";
/// The extension that flags a break-the-glass access.
pub const BREAK_GLASS_EXTENSION: &str = "urn:epr:break-glass";

/// The profiles' common prefix, in `ihe.iti.balp` 1.1.4.
const BALP: &str = "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.";
const DICOM: &str = "http://dicom.nema.org/resources/ontology/DCM";
const PARTICIPATION: &str = "http://terminology.hl7.org/CodeSystem/v3-ParticipationType";
const ENTITY_TYPE: &str = "http://terminology.hl7.org/CodeSystem/audit-entity-type";
const OBJECT_ROLE: &str = "http://terminology.hl7.org/CodeSystem/object-role";

/// Why a row cannot claim a BALP profile. Each is one constraint of the profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    /// The action has no RESTful profile here: an execute, or a delete, which clinical data never has.
    NoProfileForAction,
    /// A search: `Query` requires the query itself, and the trail keeps criteria names only.
    QueryNotHeld,
    /// `subtype`: the row does not hold the REST interaction, or holds one its action does not allow.
    NoInteraction,
    /// `agent:client.network`: the row does not hold the client's IP address.
    NoClientAddress,
    /// `agent:server`: the row does not hold the server's address.
    NoServer,
    /// `entity:patient.what`: the row names no patient.
    NoPatient,
    /// `entity:data.what`: the row names the patient and not the record, unless the record is the patient.
    NotThePatientRecord,
}

/// A row's interaction, as held in its `detail`.
fn held<'a>(row: &'a AuditRow, name: &str) -> Option<&'a str> {
    row.detail.as_ref()?.get(name)?.as_str()
}

/// Every constraint the row breaks. Empty means [`to_fhir`] may claim the profile.
#[must_use]
pub fn violations(row: &AuditRow) -> Vec<Violation> {
    let mut out = Vec::new();
    let interaction = held(row, "interaction");
    if !matches!(row.action.as_str(), "R" | "C" | "U") {
        out.push(Violation::NoProfileForAction);
    } else if interaction == Some("search-type") {
        out.push(Violation::QueryNotHeld);
    } else if !matches!(
        (row.action.as_str(), interaction),
        ("R", Some("read")) | ("C", Some("create")) | ("U", Some("update" | "patch"))
    ) {
        out.push(Violation::NoInteraction);
    }
    let address = row.network_address.as_deref();
    if address.and_then(|a| a.parse::<IpAddr>().ok()).is_none() {
        out.push(Violation::NoClientAddress);
    }
    if held(row, "server").is_none() {
        out.push(Violation::NoServer);
    }
    let patient = row
        .patient
        .as_deref()
        .and_then(|p| p.strip_prefix("Patient/"));
    if patient.is_none_or(str::is_empty) {
        out.push(Violation::NoPatient);
    }
    if row.resource.as_deref() != Some("Patient") {
        out.push(Violation::NotThePatientRecord);
    }
    out
}

/// A `Coding` as a `CodeableConcept`.
fn concept(system: &str, code: &str, display: &str) -> Value {
    json!({ "coding": [{ "system": system, "code": code, "display": display }] })
}

/// Rewrite the event as a BALP Patient profile. Call only when [`violations`] is empty. In a read, data
/// flows from the server to the client, so the client is the destination; in a write, the reverse.
fn claim(map: &mut Map<String, Value>, row: &AuditRow) -> Result<(), String> {
    const GONE: &str = "a row without violations lacks a field";
    let (profile, reads) = match row.action.as_str() {
        "R" => ("PatientRead", true),
        "C" => ("PatientCreate", false),
        _ => ("PatientUpdate", false),
    };
    let source = concept(DICOM, "110153", "Source Role ID");
    let destination = concept(DICOM, "110152", "Destination Role ID");
    let (client, server) = if reads {
        (destination, source)
    } else {
        (source, destination)
    };
    let user = if reads {
        concept(PARTICIPATION, "IRCP", "information recipient")
    } else {
        concept(PARTICIPATION, "AUT", "author (originator)")
    };
    let address = row.network_address.as_deref().ok_or(GONE)?;
    let server_address = held(row, "server").ok_or(GONE)?;
    let patient = row.patient.as_deref().ok_or(GONE)?;
    let interaction = held(row, "interaction").ok_or(GONE)?;
    map.insert(
        "meta".into(),
        json!({ "profile": [format!("{BALP}{profile}")] }),
    );
    map.insert(
        "agent".into(),
        json!([
            {
                "type": client,
                "who": { "display": address },
                "requestor": false,
                "network": { "address": address, "type": "2" },
            },
            {
                "type": server,
                "who": { "display": "electronic-patient-record" },
                "requestor": false,
                "network": { "address": server_address, "type": "5" },
            },
            {
                "type": user,
                "who": { "identifier": { "value": row.agent } },
                "requestor": true,
            },
        ]),
    );
    map.insert(
        "entity".into(),
        json!([
            {
                "what": { "reference": patient },
                "type": { "system": ENTITY_TYPE, "code": "2", "display": "System Object" },
                "role": { "system": OBJECT_ROLE, "code": "4", "display": "Domain Resource" },
            },
            {
                "what": { "reference": patient },
                "type": { "system": ENTITY_TYPE, "code": "1", "display": "Person" },
                "role": { "system": OBJECT_ROLE, "code": "1", "display": "Patient" },
            },
        ]),
    );
    let subtype = map
        .get_mut("subtype")
        .and_then(Value::as_array_mut)
        .ok_or(GONE)?;
    subtype.insert(
        0,
        json!({ "system": "http://hl7.org/fhir/restful-interaction", "code": interaction }),
    );
    Ok(())
}

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
    if violations(row).is_empty() {
        claim(map, row)?;
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
            digest: None,
            prev_digest: None,
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

    /// A row that meets every constraint of `IHE.BasicAudit.PatientRead`.
    fn claiming() -> AuditRow {
        let mut r = row();
        r.network_address = Some("192.0.2.7".into());
        r.detail = Some(json!({ "interaction": "read", "server": "https://epr.example.org" }));
        r
    }

    fn profile_of(r: &AuditRow) -> Value {
        json_of(r)["meta"]["profile"][0].clone()
    }

    /// `r` breaks `v` and nothing else, and is exported all the same, with no claim.
    fn only(r: &AuditRow, v: Violation) {
        assert_eq!(violations(r), vec![v]);
        assert!(json_of(r).get("meta").is_none(), "{v:?} still claimed");
    }

    #[test]
    fn a_row_that_meets_the_constraints_claims_the_patient_read_profile() {
        let r = claiming();
        assert!(violations(&r).is_empty());
        assert_eq!(profile_of(&r), format!("{BALP}PatientRead"));
    }

    #[test]
    fn each_write_claims_its_own_profile() {
        for (action, interaction, profile) in [
            ("C", "create", "PatientCreate"),
            ("U", "update", "PatientUpdate"),
            ("U", "patch", "PatientUpdate"),
        ] {
            let mut r = claiming();
            r.action = action.into();
            r.detail = Some(json!({ "interaction": interaction, "server": "https://e.example" }));
            assert_eq!(profile_of(&r), format!("{BALP}{profile}"));
        }
    }

    #[test]
    fn the_subtype_is_the_interaction_and_the_local_code_stays() {
        let j = json_of(&claiming());
        assert_eq!(
            j["subtype"][0]["system"],
            "http://hl7.org/fhir/restful-interaction"
        );
        assert_eq!(j["subtype"][0]["code"], "read");
        assert_eq!(j["subtype"][1]["code"], "Patient");
    }

    #[test]
    fn client_and_server_swap_roles_with_the_direction_of_the_data() {
        let read = json_of(&claiming());
        let code = |j: &Value, n: usize| j["agent"][n]["type"]["coding"][0]["code"].clone();
        assert_eq!(
            (code(&read, 0), code(&read, 1)),
            (json!("110152"), json!("110153"))
        );
        assert_eq!(code(&read, 2), "IRCP");
        let mut w = claiming();
        w.action = "U".into();
        w.detail = Some(json!({ "interaction": "update", "server": "https://e.example" }));
        let write = json_of(&w);
        assert_eq!(
            (code(&write, 0), code(&write, 1)),
            (json!("110153"), json!("110152"))
        );
        assert_eq!(code(&write, 2), "AUT");
    }

    #[test]
    fn the_client_and_the_server_have_a_network_and_only_the_user_is_the_requestor() {
        let j = json_of(&claiming());
        assert_eq!(
            j["agent"][0]["network"],
            json!({ "address": "192.0.2.7", "type": "2" })
        );
        assert_eq!(
            j["agent"][1]["network"],
            json!({ "address": "https://epr.example.org", "type": "5" })
        );
        let requestors: Vec<_> = (0..3).map(|n| j["agent"][n]["requestor"].clone()).collect();
        assert_eq!(requestors, [json!(false), json!(false), json!(true)]);
        assert_eq!(
            j["agent"][2]["who"]["identifier"]["value"],
            "607b683d-fd04-4a73-ba47-f4d05df3d046"
        );
    }

    #[test]
    fn the_entities_are_the_data_and_the_patient_each_with_a_role() {
        let j = json_of(&claiming());
        assert_eq!(j["entity"][0]["type"]["code"], "2");
        assert_eq!(j["entity"][0]["role"]["code"], "4");
        assert_eq!(j["entity"][1]["type"]["code"], "1");
        assert_eq!(j["entity"][1]["role"]["code"], "1");
        assert_eq!(j["entity"][0]["what"], j["entity"][1]["what"]);
        assert!(!j["entity"][0]["what"]["reference"].is_null());
    }

    #[test]
    fn an_execute_or_a_delete_has_no_profile() {
        for action in ["E", "D"] {
            let mut r = claiming();
            r.action = action.into();
            only(&r, Violation::NoProfileForAction);
        }
    }

    #[test]
    fn a_search_cannot_claim_because_the_query_is_not_held() {
        let mut r = claiming();
        r.detail = Some(json!({ "interaction": "search-type", "server": "https://e.example" }));
        only(&r, Violation::QueryNotHeld);
    }

    #[test]
    fn no_interaction_or_one_the_action_does_not_allow_cannot_claim() {
        let mut r = claiming();
        r.detail = Some(json!({ "server": "https://e.example" }));
        only(&r, Violation::NoInteraction);
        r.detail = Some(json!({ "interaction": "update", "server": "https://e.example" }));
        only(&r, Violation::NoInteraction);
    }

    #[test]
    fn no_client_address_or_one_that_is_not_an_ip_cannot_claim() {
        let mut r = claiming();
        r.network_address = None;
        only(&r, Violation::NoClientAddress);
        r.network_address = Some("10.0.0.7:2575".into());
        only(&r, Violation::NoClientAddress);
    }

    #[test]
    fn no_server_address_cannot_claim() {
        let mut r = claiming();
        r.detail = Some(json!({ "interaction": "read" }));
        only(&r, Violation::NoServer);
    }

    #[test]
    fn no_patient_cannot_claim() {
        let mut r = claiming();
        r.patient = None;
        only(&r, Violation::NoPatient);
        r.patient = Some("Patient/".into());
        only(&r, Violation::NoPatient);
    }

    #[test]
    fn a_row_about_another_record_cannot_claim() {
        let mut r = claiming();
        r.resource = Some("Observation".into());
        only(&r, Violation::NotThePatientRecord);
        r.resource = None;
        only(&r, Violation::NotThePatientRecord);
    }

    #[test]
    fn a_row_from_before_this_change_is_exported_as_it_was() {
        let r = row();
        assert!(!violations(&r).is_empty());
        let j = json_of(&r);
        assert!(j.get("meta").is_none());
        assert_eq!(j["agent"].as_array().unwrap().len(), 1);
        assert_eq!(j["subtype"][0]["system"], "urn:epr:audited-resource");
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
