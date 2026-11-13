//! FHIR R4 `Patient` <-> the service's identity record (crate `fhir`, R4 model).
//!
//! The identity record is deliberately thin (names, birth date, sex, identifiers, postcode,
//! contact points), so the mapping is lossy in one direction. Writing FHIR from a stored record
//! loses nothing; reading FHIR into one **drops** whatever the record has no place for (marital
//! status, photo, extensions, a full address line...). [`FromFhir::dropped`] names every top-level
//! element that was dropped, so a caller can refuse or log instead of silently discarding.

use crate::death::Answer;
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

/// The `meta.tag` system that marks a drill or test record.
pub const DRILL_TAG_SYSTEM: &str = "urn:epr:record-kind";

/// The `meta.tag` code, in [`DRILL_TAG_SYSTEM`], that marks a temporary identity (TT-1). Read as well
/// as written, so a read, change and write through FHIR keeps it.
pub const TEMPORARY_IDENTITY_TAG: &str = "temporary-identity";
/// The tag on a patient with a report of death that no person has confirmed.
pub const DEATH_REPORTED_TAG: &str = "death-reported";

/// What `Patient.deceased[x]` holds for an answer: `(element, value)`, or `None`. Only a death a person
/// has confirmed is sent here, because any other system takes `deceased[x]` as a fact. A date goes in
/// `deceasedDateTime`; with no date, or dates that disagree, `deceasedBoolean` says only that the person
/// has died (never both, as FHIR requires). An unconfirmed report is a tag: see [`with_death`].
#[must_use]
pub fn deceased_element(answer: &Answer) -> Option<(&'static str, Value)> {
    match answer {
        Answer::Asserted {
            confirmed: true,
            occurred: Some(at),
            dates_differ: false,
            ..
        } => Some(("deceasedDateTime", json!(at))),
        Answer::Asserted {
            confirmed: true, ..
        } => Some(("deceasedBoolean", json!(true))),
        _ => None,
    }
}

/// Add a patient's death answer to their FHIR JSON: `deceased[x]` for a confirmed death, a `meta.tag`
/// for a report that holds contact and is not confirmed, and nothing when no assertion stands, as an
/// absent `deceased[x]` says nothing either way. The result is checked against the typed R4 model.
///
/// # Errors
///
/// A message when the typed model refuses the result.
pub fn with_death(mut patient: Value, answer: &Answer) -> Result<Value, String> {
    if let Some((element, value)) = deceased_element(answer) {
        patient[element] = value;
    } else if answer.holds_contact() {
        let tag = json!({
            "system": DRILL_TAG_SYSTEM,
            "code": DEATH_REPORTED_TAG,
            "display": "DEATH REPORTED - NOT CONFIRMED - HOLD CONTACT",
        });
        match patient["meta"]["tag"].as_array_mut() {
            Some(tags) => tags.push(tag),
            None => patient["meta"]["tag"] = json!([tag]),
        }
    }
    serde_json::from_value::<Resource>(patient.clone()).map_err(|e| e.to_string())?;
    Ok(patient)
}

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
    let mut tags = Vec::new();
    if stored.drill {
        // A drill or test record says so in the resource itself (EC-10).
        tags.push(json!({
            "system": DRILL_TAG_SYSTEM,
            "code": "drill",
            "display": "DRILL - NOT A REAL PATIENT",
        }));
    }
    if p.temporary_identity {
        // So does a temporary identity, for any reader that is not the banner (TT-1).
        tags.push(json!({
            "system": DRILL_TAG_SYSTEM,
            "code": TEMPORARY_IDENTITY_TAG,
            "display": "TEMPORARY IDENTITY - NAME AND BIRTH DATE ARE NOT CONFIRMED",
        }));
    }
    if !tags.is_empty() {
        out["meta"]["tag"] = Value::Array(tags);
    }
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
    let temporary_identity = raw
        .pointer("/meta/tag")
        .and_then(Value::as_array)
        .is_some_and(|tags| {
            tags.iter().any(|t| {
                t.get("system").and_then(Value::as_str) == Some(DRILL_TAG_SYSTEM)
                    && t.get("code").and_then(Value::as_str) == Some(TEMPORARY_IDENTITY_TAG)
            })
        });
    Ok(FromFhir {
        patient: Patient {
            death_date: None,
            id: None,
            names,
            birth_date: text_of(&raw, "birthDate"),
            sex,
            identifiers,
            addresses,
            telecom,
            temporary_identity,
        },
        dropped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use electronic_patient_record_matcher::NATIONAL_NUMBER_SYSTEM;

    fn stored() -> Stored {
        Stored {
            drill: false,
            id: "11111111-1111-4111-8111-111111111111".into(),
            version: 3,
            author: "a".into(),
            patient: Patient {
                death_date: None,
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
                    system: NATIONAL_NUMBER_SYSTEM.into(),
                    value: "9434765919".into(),
                }],
                addresses: vec![Address {
                    postal_code: Some("AB1 2CD".into()),
                }],
                temporary_identity: false,
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
        assert_eq!(json["identifier"][0]["system"], NATIONAL_NUMBER_SYSTEM);
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
    fn a_temporary_identity_carries_a_tag_that_survives_a_read_and_write() {
        let text = |s: &Stored| serde_json::to_string(&to_fhir(s).unwrap()).unwrap();
        let mut s = stored();
        assert!(!text(&s).contains(TEMPORARY_IDENTITY_TAG));
        s.patient.temporary_identity = true;
        s.drill = true;
        let text = text(&s);
        let json: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["meta"]["tag"][0]["code"], "drill");
        assert_eq!(json["meta"]["tag"][1]["code"], TEMPORARY_IDENTITY_TAG);
        assert!(from_fhir_json(&text).unwrap().patient.temporary_identity);
    }

    #[test]
    fn a_tag_from_another_system_does_not_make_a_temporary_identity() {
        let text = r#"{"resourceType":"Patient","meta":{"tag":[
            {"system":"urn:other","code":"temporary-identity"}]}}"#;
        assert!(!from_fhir_json(text).unwrap().patient.temporary_identity);
    }

    #[test]
    fn a_drill_record_carries_a_tag_and_a_real_one_does_not() {
        let mut s = stored();
        assert!(
            serde_json::to_value(to_fhir(&s).unwrap()).unwrap()["meta"]
                .get("tag")
                .is_none()
        );
        s.drill = true;
        let json = serde_json::to_value(to_fhir(&s).unwrap()).unwrap();
        assert_eq!(json["meta"]["tag"][0]["system"], DRILL_TAG_SYSTEM);
        assert_eq!(json["meta"]["tag"][0]["code"], "drill");
        assert_eq!(json["meta"]["versionId"], "3");
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

    fn asserted(confirmed: bool, occurred: Option<&str>, dates_differ: bool) -> Answer {
        use crate::death::Status;
        Answer::Asserted {
            status: if confirmed {
                Status::Verified
            } else {
                Status::Reported
            },
            confirmed,
            occurred: occurred.map(str::to_string),
            dates_differ,
        }
    }

    #[test]
    fn only_a_confirmed_death_is_sent_as_deceased() {
        let patient = || serde_json::to_value(to_fhir(&stored()).unwrap()).unwrap();
        // Nothing stands: nothing is said, either way.
        let none = with_death(patient(), &Answer::NotKnownToHaveDied).unwrap();
        assert_eq!(none, patient());
        // A report holds contact, but no other system may read it as a fact.
        let reported = with_death(patient(), &asserted(false, Some("2026-11-01"), false)).unwrap();
        assert!(reported.get("deceasedDateTime").is_none());
        assert!(reported.get("deceasedBoolean").is_none());
        assert_eq!(reported["meta"]["tag"][0]["code"], DEATH_REPORTED_TAG);
        // Confirmed, with a date as given, partial dates included.
        for at in ["2026", "2026-11", "2026-11-01", "2026-11-01T09:00:00Z"] {
            let dated = with_death(patient(), &asserted(true, Some(at), false)).unwrap();
            assert_eq!(dated["deceasedDateTime"], at);
            assert!(dated.get("deceasedBoolean").is_none());
        }
        // Confirmed with no date, or dates that disagree: that the person has died, and no date.
        for answer in [
            asserted(true, None, false),
            asserted(true, Some("2026-11-01"), true),
        ] {
            let bare = with_death(patient(), &answer).unwrap();
            assert_eq!(bare["deceasedBoolean"], true);
            assert!(bare.get("deceasedDateTime").is_none());
        }
    }
}
