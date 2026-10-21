//! FHIR R4 `Patient` <-> the service's identity record (crate `fhir`, R4 model).
//!
//! The identity record is deliberately thin (names, birth date, sex, identifiers, postcode,
//! contact points), so the mapping is lossy in one direction. Writing FHIR from a stored record
//! loses nothing; reading FHIR into one **drops** whatever the record has no place for (marital
//! status, photo, extensions, a full address line...). [`FromFhir::dropped`] names every top-level
//! element that was dropped, so a caller can refuse or log instead of silently discarding.

use crate::patients_store::Stored;
use electronic_patient_record_matcher::{
    Address, AdministrativeSex, HumanName, Identifier, Patient, Telecom,
};
use fhir::r4::resources::Resource;
use fhir::r4::validate::Validate;
use serde_json::{Value, json};

/// The elements of a FHIR `Patient` that map to the identity record. Everything else is dropped.
const MAPPED: &[&str] = &[
    "resourceType",
    "id",
    "meta",
    "identifier",
    "name",
    "gender",
    "birthDate",
    "address",
    "telecom",
];

/// A FHIR patient read into the identity record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FromFhir {
    /// The identity record. Its `id` is never taken from the FHIR resource: the server assigns it.
    pub patient: Patient,
    /// Top-level FHIR elements that had no place in the record and were dropped.
    pub dropped: Vec<String>,
}

/// A FHIR resource that could not be read as a patient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable(pub Vec<String>);

/// The stored record as a FHIR R4 `Patient` (a `Resource::Patient`, so it serializes with its
/// `resourceType`), with `meta.versionId` set to the record version. The
/// result is parsed through the typed R4 model, so it is known to be shaped as R4 requires.
///
/// # Errors
///
/// A message when the typed model refuses the result (a bug here, or a value the model rejects).
pub fn to_fhir(stored: &Stored) -> Result<Resource, String> {
    let p = &stored.patient;
    let mut out = json!({
        "resourceType": "Patient",
        "id": stored.id,
        "meta": { "versionId": stored.version.to_string() },
    });
    let map = out.as_object_mut().ok_or("not an object")?;
    if !p.identifiers.is_empty() {
        map.insert(
            "identifier".into(),
            p.identifiers
                .iter()
                .map(|i| json!({ "system": i.system, "value": i.value }))
                .collect(),
        );
    }
    if !p.names.is_empty() {
        map.insert(
            "name".into(),
            p.names
                .iter()
                .map(|n| {
                    let mut name = serde_json::Map::new();
                    if let Some(f) = &n.family {
                        name.insert("family".into(), json!(f));
                    }
                    if !n.given.is_empty() {
                        name.insert("given".into(), json!(n.given));
                    }
                    Value::Object(name)
                })
                .collect(),
        );
    }
    if let Some(sex) = p.sex {
        map.insert(
            "gender".into(),
            serde_json::to_value(sex).map_err(|e| e.to_string())?,
        );
    }
    if let Some(d) = &p.birth_date {
        map.insert("birthDate".into(), json!(d));
    }
    if !p.addresses.is_empty() {
        map.insert(
            "address".into(),
            p.addresses
                .iter()
                .map(|a| match &a.postal_code {
                    Some(code) => json!({ "postalCode": code }),
                    None => json!({}),
                })
                .collect(),
        );
    }
    if !p.telecom.is_empty() {
        map.insert(
            "telecom".into(),
            p.telecom
                .iter()
                .map(|t| json!({ "system": t.system, "value": t.value }))
                .collect(),
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

/// Read FHIR R4 `Patient` JSON into the identity record.
///
/// The text is parsed through the typed R4 model first, so a malformed resource, a wrong
/// `resourceType` or a value of the wrong type is refused rather than half-read.
///
/// # Errors
///
/// [`Unreadable`], listing every problem found.
pub fn from_fhir_json(text: &str) -> Result<FromFhir, Unreadable> {
    let raw: Value = serde_json::from_str(text).map_err(|e| Unreadable(vec![e.to_string()]))?;
    if raw.get("resourceType").and_then(Value::as_str) != Some("Patient") {
        return Err(Unreadable(vec!["resourceType must be \"Patient\"".into()]));
    }
    let typed: Resource =
        serde_json::from_value(raw.clone()).map_err(|e| Unreadable(vec![e.to_string()]))?;
    let issues = typed.validate();
    if !issues.is_empty() {
        return Err(Unreadable(
            issues.iter().map(|i| format!("{i:?}")).collect(),
        ));
    }
    let mut problems = Vec::new();
    let text_of = |v: &Value, key: &str| v.get(key).and_then(Value::as_str).map(str::to_string);

    let names = raw
        .get("name")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|n| HumanName {
                    family: text_of(n, "family"),
                    given: n
                        .get("given")
                        .and_then(Value::as_array)
                        .map(|g| {
                            g.iter()
                                .filter_map(|x| x.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut identifiers = Vec::new();
    for (i, id) in raw
        .get("identifier")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        match (text_of(id, "system"), text_of(id, "value")) {
            (Some(system), Some(value)) => identifiers.push(Identifier { system, value }),
            _ => problems.push(format!("identifier {} needs a system and a value", i + 1)),
        }
    }

    let sex = match raw.get("gender").and_then(Value::as_str) {
        None => None,
        Some("male") => Some(AdministrativeSex::Male),
        Some("female") => Some(AdministrativeSex::Female),
        Some("other") => Some(AdministrativeSex::Other),
        Some("unknown") => Some(AdministrativeSex::Unknown),
        Some(other) => {
            problems.push(format!(
                "gender {other:?} is not male, female, other or unknown"
            ));
            None
        }
    };

    let addresses = raw
        .get("address")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|x| Address {
                    postal_code: text_of(x, "postalCode"),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut telecom = Vec::new();
    for (i, t) in raw
        .get("telecom")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        match (text_of(t, "system"), text_of(t, "value")) {
            (Some(system), Some(value)) => telecom.push(Telecom { system, value }),
            _ => problems.push(format!("telecom {} needs a system and a value", i + 1)),
        }
    }

    if !problems.is_empty() {
        return Err(Unreadable(problems));
    }
    let dropped = raw
        .as_object()
        .map(|o| {
            o.keys()
                .filter(|k| !MAPPED.contains(&k.as_str()))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    Ok(FromFhir {
        patient: Patient {
            id: None,
            names,
            birth_date: text_of(&raw, "birthDate"),
            sex,
            identifiers,
            addresses,
            telecom,
        },
        dropped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use electronic_patient_record_matcher::NHS_NUMBER_SYSTEM;

    fn stored() -> Stored {
        Stored {
            id: "11111111-1111-4111-8111-111111111111".into(),
            version: 3,
            author: "a".into(),
            patient: Patient {
                id: Some("11111111-1111-4111-8111-111111111111".into()),
                names: vec![
                    HumanName {
                        family: Some("Smith".into()),
                        given: vec!["Ann".into(), "Marie".into()],
                    },
                    HumanName {
                        family: Some("Jones".into()),
                        given: vec![],
                    },
                ],
                birth_date: Some("1980-03".into()),
                sex: Some(AdministrativeSex::Female),
                identifiers: vec![Identifier {
                    system: NHS_NUMBER_SYSTEM.into(),
                    value: "9434765919".into(),
                }],
                addresses: vec![Address {
                    postal_code: Some("AB1 2CD".into()),
                }],
                telecom: vec![Telecom {
                    system: "phone".into(),
                    value: "01632 960001".into(),
                }],
            },
        }
    }

    #[test]
    fn a_stored_record_becomes_a_valid_r4_patient_with_its_version() {
        let fhir = to_fhir(&stored()).unwrap();
        let json = serde_json::to_value(&fhir).unwrap();
        assert_eq!(json["resourceType"], "Patient");
        assert_eq!(json["id"], "11111111-1111-4111-8111-111111111111");
        assert_eq!(json["meta"]["versionId"], "3");
        assert_eq!(json["gender"], "female");
        assert_eq!(json["birthDate"], "1980-03");
        assert_eq!(json["name"][0]["given"], json!(["Ann", "Marie"]));
        assert_eq!(json["name"][1]["family"], "Jones");
        assert_eq!(json["identifier"][0]["system"], NHS_NUMBER_SYSTEM);
        assert_eq!(json["address"][0]["postalCode"], "AB1 2CD");
    }

    #[test]
    fn what_is_written_reads_back_the_same_without_the_server_id() {
        let s = stored();
        let text = serde_json::to_string(&to_fhir(&s).unwrap()).unwrap();
        let back = from_fhir_json(&text).unwrap();
        let mut expected = s.patient.clone();
        expected.id = None;
        assert_eq!(back.patient, expected);
        assert!(back.dropped.is_empty(), "{:?}", back.dropped);
    }

    #[test]
    fn a_sparse_record_omits_what_it_lacks_instead_of_writing_nulls() {
        let s = Stored {
            patient: Patient {
                names: vec![HumanName {
                    family: Some("Solo".into()),
                    given: vec![],
                }],
                ..Default::default()
            },
            ..stored()
        };
        let json = serde_json::to_value(to_fhir(&s).unwrap()).unwrap();
        for absent in ["gender", "birthDate", "identifier", "address", "telecom"] {
            assert!(
                json.get(absent).is_none(),
                "{absent} should be absent: {json}"
            );
        }
    }

    #[test]
    fn elements_with_no_place_in_the_record_are_named_not_hidden() {
        let text = r#"{"resourceType":"Patient","active":true,"maritalStatus":{"text":"M"},
            "photo":[{"contentType":"image/png"}],"name":[{"family":"Chalmers","given":["Peter"]}],
            "birthDate":"1974-12-25","gender":"male"}"#;
        let r = from_fhir_json(text).unwrap();
        assert_eq!(r.patient.names[0].family.as_deref(), Some("Chalmers"));
        assert_eq!(r.patient.sex, Some(AdministrativeSex::Male));
        let mut dropped = r.dropped.clone();
        dropped.sort();
        assert_eq!(dropped, ["active", "maritalStatus", "photo"]);
    }

    #[test]
    fn a_client_chosen_id_is_ignored() {
        let r = from_fhir_json(r#"{"resourceType":"Patient","id":"mine","name":[{"family":"X"}]}"#)
            .unwrap();
        assert_eq!(r.patient.id, None);
        assert!(!r.dropped.contains(&"id".to_string()));
    }

    #[test]
    fn unreadable_input_is_refused_with_reasons() {
        let cases = [
            ("not json", "expected"),
            (r#"{"resourceType":"Observation"}"#, "resourceType"),
            (
                r#"{"resourceType":"Patient","identifier":[{"value":"1"}]}"#,
                "identifier 1",
            ),
            (
                r#"{"resourceType":"Patient","telecom":[{"system":"phone"}]}"#,
                "telecom 1",
            ),
            (
                r#"{"resourceType":"Patient","name":"Smith"}"#,
                "invalid type",
            ),
        ];
        for (text, expect) in cases {
            let err = from_fhir_json(text).unwrap_err();
            assert!(
                err.0
                    .iter()
                    .any(|m| m.to_lowercase().contains(&expect.to_lowercase())),
                "{text}: {err:?}"
            );
        }
    }
}
