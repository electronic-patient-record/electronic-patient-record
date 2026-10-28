//! The service's observations (vital signs) as FHIR R4 `Observation` (crate `fhir`, R4 model).
//!
//! One direction only, like the allergy mapping. An observation entered in error is not exported:
//! it never happened. A missing value is `dataAbsentReason`, never a quantity of zero. The flag is
//! the one computed when read ([`crate::observations::flag`]) and goes out as `interpretation`;
//! no flag, no `interpretation`. The critical range has no element in `Observation`, so it is not
//! exported (its effect survives as the `LL`/`HH` interpretation).
//!
//! Systolic and diastolic pressure stay two observations, as stored; they are not folded into a
//! FHIR blood-pressure panel.

use crate::observations::{AbsentReason, Flag, Range};
use crate::observations_store::{Status, StoredObservation};
use fhir::r4::resources::Resource;
use fhir::r4::validate::Validate;
use serde_json::{Value, json};

const LOINC: &str = "http://loinc.org";
const UCUM: &str = "http://unitsofmeasure.org";
const CATEGORY: &str = "http://terminology.hl7.org/CodeSystem/observation-category";
const INTERPRETATION: &str = "http://terminology.hl7.org/CodeSystem/v3-ObservationInterpretation";
const ABSENT: &str = "http://terminology.hl7.org/CodeSystem/data-absent-reason";

fn absent_code(r: AbsentReason) -> &'static str {
    match r {
        AbsentReason::NotDone => "not-performed",
        AbsentReason::Declined => "asked-declined",
        AbsentReason::Failed => "error",
        AbsentReason::Unknown => "unknown",
    }
}

fn interpretation(f: Flag) -> (&'static str, &'static str) {
    match f {
        Flag::CriticalLow => ("LL", "Critically low"),
        Flag::Low => ("L", "Low"),
        Flag::Normal => ("N", "Normal"),
        Flag::High => ("H", "High"),
        Flag::CriticalHigh => ("HH", "Critically high"),
    }
}

fn quantity(value: f64, unit: &str) -> Value {
    json!({ "value": value, "unit": unit, "system": UCUM, "code": unit })
}

fn reference_range(r: &Range, unit: Option<&str>) -> Option<Value> {
    // A bound without a unit would be a bare number; leave the range out instead.
    let unit = unit?;
    let mut out = json!({});
    if let Some(l) = r.low {
        out["low"] = quantity(l, unit);
    }
    if let Some(h) = r.high {
        out["high"] = quantity(h, unit);
    }
    (r.low.is_some() || r.high.is_some()).then_some(out)
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

/// One stored observation as an `Observation` for `patient` (the record the caller asked about: an
/// observation merged in from another record is reported under the surviving one).
///
/// Returns `Ok(None)` for an observation entered in error.
///
/// # Errors
///
/// A message when the typed model refuses the result.
pub fn to_fhir(
    stored: &StoredObservation,
    patient: &str,
    notices: &[crate::amendment::Notice],
) -> Result<Option<Resource>, String> {
    let r = &stored.record;
    if r.status == Status::EnteredInError {
        return Ok(None);
    }
    let mut out = json!({
        "resourceType": "Observation",
        "id": stored.id,
        "meta": { "versionId": stored.version.to_string() },
        "status": "final",
        "category": [{ "coding": [{ "system": CATEGORY, "code": "vital-signs", "display": "Vital Signs" }] }],
        "code": { "coding": [{ "system": LOINC, "code": r.code }] },
        "subject": { "reference": format!("Patient/{patient}") },
        "effectiveDateTime": r.effective_at,
        "issued": stored.recorded_at,
    });
    match (r.value, r.unit.as_deref(), r.absent_reason) {
        (Some(v), Some(u), _) => out["valueQuantity"] = quantity(v, u),
        (_, _, Some(reason)) => {
            out["dataAbsentReason"] =
                json!({ "coding": [{ "system": ABSENT, "code": absent_code(reason) }] });
        }
        // Stored records always have one or the other; refuse rather than guess.
        _ => return Err("an observation has neither a value with a unit nor a reason".into()),
    }
    if let Some(f) = stored.flag {
        let (code, display) = interpretation(f);
        out["interpretation"] =
            json!([{ "coding": [{ "system": INTERPRETATION, "code": code, "display": display }] }]);
    }
    if let Some(range) = r
        .range
        .as_ref()
        .and_then(|x| reference_range(x, r.unit.as_deref()))
    {
        out["referenceRange"] = json!([range]);
    }
    // A reader of the export is told the record is disputed, in the same words as a reader of the page.
    if !notices.is_empty() {
        out["note"] = notices
            .iter()
            .map(crate::amendment::Notice::fhir_annotation)
            .collect();
    }
    parse(out).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observations_store::ObservationRecord;

    fn stored() -> StoredObservation {
        StoredObservation {
            id: "33333333-3333-4333-8333-333333333333".into(),
            patient_id: "11111111-1111-4111-8111-111111111111".into(),
            version: 3,
            author: "someone".into(),
            recorded_at: "2026-10-26T09:31:00.123456Z".into(),
            site_zone: "Europe/London".into(),
            record: ObservationRecord {
                code: "8867-4".into(),
                value: Some(130.0),
                unit: Some("/min".into()),
                absent_reason: None,
                range: Some(Range {
                    low: Some(60.0),
                    high: Some(100.0),
                }),
                critical: Some(Range {
                    low: Some(40.0),
                    high: Some(150.0),
                }),
                effective_at: "2026-10-26T09:30:00Z".into(),
                status: Status::Final,
            },
            flag: Some(Flag::High),
        }
    }

    fn json_of(s: &StoredObservation) -> Value {
        serde_json::to_value(to_fhir(s, "P1", &[]).unwrap().unwrap()).unwrap()
    }

    #[test]
    fn a_vital_sign_carries_value_unit_flag_and_range() {
        let j = json_of(&stored());
        assert_eq!(j["resourceType"], "Observation");
        assert_eq!(j["meta"]["versionId"], "3");
        assert_eq!(j["status"], "final");
        assert_eq!(j["category"][0]["coding"][0]["code"], "vital-signs");
        assert_eq!(j["code"]["coding"][0]["code"], "8867-4");
        assert_eq!(j["subject"]["reference"], "Patient/P1");
        assert_eq!(j["valueQuantity"]["value"], 130.0);
        assert_eq!(j["valueQuantity"]["code"], "/min");
        assert_eq!(j["interpretation"][0]["coding"][0]["code"], "H");
        assert_eq!(j["referenceRange"][0]["low"]["value"], 60.0);
        assert_eq!(j["referenceRange"][0]["high"]["value"], 100.0);
    }

    #[test]
    fn critical_flags_map_to_ll_and_hh_and_the_critical_range_is_not_exported() {
        let mut s = stored();
        s.flag = Some(Flag::CriticalHigh);
        let j = json_of(&s);
        assert_eq!(j["interpretation"][0]["coding"][0]["code"], "HH");
        s.flag = Some(Flag::CriticalLow);
        assert_eq!(json_of(&s)["interpretation"][0]["coding"][0]["code"], "LL");
        assert!(!j.to_string().contains("150"));
    }

    #[test]
    fn a_missing_value_is_a_reason_never_zero() {
        let mut s = stored();
        s.record.value = None;
        s.record.unit = None;
        s.record.absent_reason = Some(AbsentReason::Declined);
        s.record.range = None;
        s.flag = None;
        let j = json_of(&s);
        assert!(j.get("valueQuantity").is_none());
        assert_eq!(j["dataAbsentReason"]["coding"][0]["code"], "asked-declined");
        assert!(j.get("interpretation").is_none());
        assert!(j.get("referenceRange").is_none());
    }

    #[test]
    fn every_absent_reason_has_its_own_code() {
        let codes: Vec<_> = [
            AbsentReason::NotDone,
            AbsentReason::Declined,
            AbsentReason::Failed,
            AbsentReason::Unknown,
        ]
        .into_iter()
        .map(absent_code)
        .collect();
        assert_eq!(
            codes,
            ["not-performed", "asked-declined", "error", "unknown"]
        );
    }

    #[test]
    fn no_flag_means_no_interpretation() {
        let mut s = stored();
        s.flag = None;
        assert!(json_of(&s).get("interpretation").is_none());
    }

    #[test]
    fn one_entered_in_error_is_not_exported() {
        let mut s = stored();
        s.record.status = Status::EnteredInError;
        assert!(to_fhir(&s, "P1", &[]).unwrap().is_none());
    }

    #[test]
    fn a_record_with_neither_value_nor_reason_is_refused() {
        let mut s = stored();
        s.record.value = None;
        s.record.unit = None;
        assert!(to_fhir(&s, "P1", &[]).is_err());
    }

    #[test]
    fn a_disputed_vital_sign_carries_the_notice_as_a_note_and_an_undisputed_one_carries_none() {
        use crate::amendment::{Ground, Notice};
        let none = serde_json::to_value(to_fhir(&stored(), "P1", &[]).unwrap().unwrap()).unwrap();
        assert!(none.get("note").is_none());
        let notices = [
            Notice::Pending,
            Notice::Refused {
                ground: Ground::AccurateAndComplete,
                disagreement: Some("I was not feverish.".into()),
            },
        ];
        let j = serde_json::to_value(to_fhir(&stored(), "P1", &notices).unwrap().unwrap()).unwrap();
        let notes = j["note"].as_array().unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(
            notes[0]["extension"][0]["url"],
            crate::amendment::AMENDMENT_NOTICE_URL
        );
        assert_eq!(
            notes[1]["extension"][0]["extension"][1]["valueCode"],
            "accurate-and-complete"
        );
        assert!(
            notes[0]["text"]
                .as_str()
                .unwrap()
                .contains("No decision has been recorded.")
        );
        assert!(
            notes[1]["text"]
                .as_str()
                .unwrap()
                .contains("I was not feverish.")
        );
    }
}
