//! The service's allergy records as FHIR R4 `AllergyIntolerance` (crate `fhir`, R4 model).
//!
//! One direction only: a stored allergy goes out as FHIR. Nothing reads FHIR in, because an
//! incoming resource would have to be fitted to a record that cannot hold most of its elements.
//!
//! What is *not* said is as deliberate as what is. The service records neither whether an
//! allergy is confirmed nor whether it is an allergy or an intolerance, so `verificationStatus`
//! and `type` are left out rather than guessed. A criticality nobody stated is left out, never
//! `low`. An allergy entered in error is not exported at all: it never happened.
//!
//! "No known allergies" is exported as the SNOMED CT concept `716186003`, and only while the
//! summary says it still holds ([`crate::allergies::Summary::NoKnown`]).

use crate::allergies::{AllergyRecord, Category, Criticality, Status, StoredAllergy};
use fhir::r4::resources::Resource;
use fhir::r4::validate::Validate;
use serde_json::{Value, json};

/// SNOMED CT: |No known allergy|.
pub const NO_KNOWN_ALLERGY: &str = "716186003";

const CLINICAL_STATUS: &str = "http://terminology.hl7.org/CodeSystem/allergyintolerance-clinical";
const VERIFICATION_STATUS: &str =
    "http://terminology.hl7.org/CodeSystem/allergyintolerance-verification";

fn category(c: Category) -> &'static str {
    match c {
        Category::Food => "food",
        Category::Medication => "medication",
        Category::Environment => "environment",
        Category::Biologic => "biologic",
    }
}

fn criticality(c: Criticality) -> &'static str {
    match c {
        Criticality::Low => "low",
        Criticality::High => "high",
        Criticality::UnableToAssess => "unable-to-assess",
    }
}

fn status_coding(system: &str, code: &str) -> Value {
    json!({ "coding": [{ "system": system, "code": code }] })
}

fn parse(json: Value) -> Result<Resource, String> {
    let resource: Resource = serde_json::from_value(json).map_err(|e| e.to_string())?;
    let issues = resource.validate();
    if issues.is_empty() {
        Ok(resource)
    } else {
        Err(format!("the FHIR model rejects the result: {issues:?}"))
    }
}

/// One stored allergy as an `AllergyIntolerance` for `patient` (the record the caller asked
/// about: an allergy merged in from another record is reported under the surviving one).
///
/// Returns `Ok(None)` for an allergy entered in error.
///
/// # Errors
///
/// A message when the typed model refuses the result.
pub fn to_fhir(stored: &StoredAllergy, patient: &str) -> Result<Option<Resource>, String> {
    let a: &AllergyRecord = &stored.allergy;
    let clinical = match a.status {
        Status::Active => "active",
        Status::Inactive => "inactive",
        Status::EnteredInError => return Ok(None),
    };
    let mut code = json!({ "text": a.substance });
    if let Some(c) = &a.coding {
        code["coding"] = json!([{ "system": c.system, "code": c.code }]);
    }
    let mut out = json!({
        "resourceType": "AllergyIntolerance",
        "id": stored.id,
        "meta": { "versionId": stored.version.to_string() },
        "clinicalStatus": status_coding(CLINICAL_STATUS, clinical),
        "code": code,
        "patient": { "reference": format!("Patient/{patient}") },
        "recordedDate": stored.recorded_at,
    });
    if let Some(c) = a.category {
        out["category"] = json!([category(c)]);
    }
    if let Some(c) = a.criticality {
        out["criticality"] = json!(criticality(c));
    }
    if let Some(onset) = &a.onset {
        out["onsetDateTime"] = json!(onset);
    }
    if let Some(reaction) = &a.reaction {
        out["reaction"] = json!([{ "manifestation": [{ "text": reaction }] }]);
    }
    parse(out).map(Some)
}

/// The statement "no known allergies" as an `AllergyIntolerance`. Call it only while the summary
/// is `NoKnown`: a withdrawn or outdated statement must not be exported.
///
/// # Errors
///
/// A message when the typed model refuses the result.
pub fn no_known_allergies(patient: &str, asserted_at: &str) -> Result<Resource, String> {
    parse(json!({
        "resourceType": "AllergyIntolerance",
        "id": format!("no-known-allergies-{patient}"),
        "clinicalStatus": status_coding(CLINICAL_STATUS, "active"),
        "verificationStatus": status_coding(VERIFICATION_STATUS, "confirmed"),
        "code": {
            "coding": [{
                "system": crate::interop::terminology::SNOMED_SYSTEM,
                "code": NO_KNOWN_ALLERGY,
                "display": "No known allergy",
            }],
            "text": "No known allergies",
        },
        "patient": { "reference": format!("Patient/{patient}") },
        "recordedDate": asserted_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::allergies::Coding;

    fn stored(status: Status) -> StoredAllergy {
        StoredAllergy {
            id: "22222222-2222-4222-8222-222222222222".into(),
            patient_id: "11111111-1111-4111-8111-111111111111".into(),
            version: 2,
            author: "someone".into(),
            recorded_at: "2026-10-21T09:30:00.123456Z".into(),
            allergy: AllergyRecord {
                substance: "Penicillin".into(),
                coding: Some(Coding {
                    system: "http://snomed.info/sct".into(),
                    code: "22298006".into(),
                }),
                category: Some(Category::Medication),
                criticality: Some(Criticality::High),
                reaction: Some("Hives".into()),
                onset: Some("2019-04".into()),
                status,
            },
        }
    }

    fn json_of(r: &Resource) -> Value {
        serde_json::to_value(r).unwrap()
    }

    #[test]
    fn an_active_allergy_carries_what_was_recorded() {
        let r = to_fhir(&stored(Status::Active), "P1").unwrap().unwrap();
        let j = json_of(&r);
        assert_eq!(j["resourceType"], "AllergyIntolerance");
        assert_eq!(j["meta"]["versionId"], "2");
        assert_eq!(j["clinicalStatus"]["coding"][0]["code"], "active");
        assert_eq!(j["criticality"], "high");
        assert_eq!(j["category"][0], "medication");
        assert_eq!(j["code"]["text"], "Penicillin");
        assert_eq!(j["code"]["coding"][0]["code"], "22298006");
        assert_eq!(j["patient"]["reference"], "Patient/P1");
        assert_eq!(j["onsetDateTime"], "2019-04");
        assert_eq!(j["reaction"][0]["manifestation"][0]["text"], "Hives");
    }

    #[test]
    fn what_was_not_recorded_is_not_invented() {
        let mut s = stored(Status::Active);
        s.allergy.criticality = None;
        s.allergy.category = None;
        s.allergy.coding = None;
        s.allergy.reaction = None;
        s.allergy.onset = None;
        let j = json_of(&to_fhir(&s, "P1").unwrap().unwrap());
        for absent in [
            "criticality",
            "category",
            "reaction",
            "onsetDateTime",
            // The service records neither of these, so it must not state them.
            "verificationStatus",
            "type",
        ] {
            assert!(j.get(absent).is_none(), "{absent} must be absent: {j}");
        }
        assert!(j["code"].get("coding").is_none());
    }

    #[test]
    fn an_unassessed_criticality_is_not_low() {
        let mut s = stored(Status::Active);
        s.allergy.criticality = Some(Criticality::UnableToAssess);
        let j = json_of(&to_fhir(&s, "P1").unwrap().unwrap());
        assert_eq!(j["criticality"], "unable-to-assess");
    }

    #[test]
    fn an_ended_allergy_is_inactive_and_one_entered_in_error_is_not_exported() {
        let j = json_of(&to_fhir(&stored(Status::Inactive), "P1").unwrap().unwrap());
        assert_eq!(j["clinicalStatus"]["coding"][0]["code"], "inactive");
        assert!(
            to_fhir(&stored(Status::EnteredInError), "P1")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn the_none_known_statement_is_a_confirmed_snomed_concept() {
        let j = json_of(&no_known_allergies("P1", "2026-10-21T09:30:00Z").unwrap());
        assert_eq!(j["code"]["coding"][0]["code"], NO_KNOWN_ALLERGY);
        assert_eq!(j["verificationStatus"]["coding"][0]["code"], "confirmed");
        // The concept id the export uses is itself a valid SCTID naming a concept.
        assert_eq!(
            crate::interop::terminology::validate_coding(
                crate::interop::terminology::SNOMED_SYSTEM,
                NO_KNOWN_ALLERGY
            ),
            Ok(())
        );
    }
}
