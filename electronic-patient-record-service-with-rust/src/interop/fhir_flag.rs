//! The service's reasonable adjustments as FHIR R4 `Flag` (crate `fhir`, R4 model), one direction only
//! (PX-3).
//!
//! An adjustment that stands is an `active` flag and an ended one `inactive`, with `period.end` the time
//! its last version was written. An active flag has no `period`, because a current version's time is its
//! last change, not when the need began. The code is this project's own: no national code is used,
//! because none has had its licence checked. The note goes out as the code's text. The reason does not,
//! because it says how we came to know and is not part of the need.

use crate::adjustments::Status;
use crate::adjustments_store::StoredAdjustment;
use fhir::r4::resources::Resource;
use fhir::r4::validate::Validate;
use serde_json::{Value, json};

/// This project's own code system for the catalogue in [`crate::adjustments`].
pub const CODE_SYSTEM: &str = "urn:epr:reasonable-adjustment";
/// This project's own code system for its categories.
pub const CATEGORY_SYSTEM: &str = "urn:epr:reasonable-adjustment-category";

/// One adjustment as a `Flag`, checked against the FHIR model.
///
/// # Errors
///
/// When the FHIR model refuses the result.
pub fn to_fhir(stored: &StoredAdjustment, patient: &str) -> Result<Value, String> {
    let a = &stored.adjustment;
    let mut code =
        json!({ "coding": [{ "system": CODE_SYSTEM, "code": a.code, "display": stored.display }] });
    if let Some(note) = &a.note {
        code["text"] = json!(note);
    }
    let ended = a.status == Status::Ended;
    let mut out = json!({
        "resourceType": "Flag",
        "id": stored.id,
        "meta": { "versionId": stored.version.to_string() },
        "status": if ended { "inactive" } else { "active" },
        "category": [{ "coding": [{ "system": CATEGORY_SYSTEM, "code": stored.category }] }],
        "code": code,
        "subject": { "reference": format!("Patient/{patient}") },
    });
    if ended {
        out["period"] = json!({ "end": stored.recorded_at });
    }
    let resource: Resource = serde_json::from_value(out.clone()).map_err(|e| e.to_string())?;
    let issues = resource.validate();
    if issues.is_empty() {
        Ok(out)
    } else {
        Err(format!("the FHIR model rejects the result: {issues:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adjustments::Adjustment;

    fn stored(status: Status, note: Option<&str>) -> StoredAdjustment {
        StoredAdjustment {
            id: "44444444-4444-4444-8444-444444444444".into(),
            patient_id: "11111111-1111-4111-8111-111111111111".into(),
            version: 2,
            author: "someone".into(),
            recorded_at: "2026-11-05T09:31:00.123456Z".into(),
            adjustment: Adjustment {
                code: "interpreter-spoken".into(),
                note: note.map(str::to_string),
                status,
                review_by: Some("2027-01-01".into()),
                reason: "the daughter telephoned".into(),
            },
            category: "interpreting".into(),
            display: "Spoken language interpreter".into(),
            overdue: false,
        }
    }

    #[test]
    fn an_adjustment_that_stands_is_an_active_flag() {
        let j = to_fhir(&stored(Status::Active, Some("Polish")), "P1").unwrap();
        assert_eq!(j["resourceType"], "Flag");
        assert_eq!(j["status"], "active");
        assert_eq!(j["code"]["coding"][0]["system"], CODE_SYSTEM);
        assert_eq!(j["code"]["coding"][0]["code"], "interpreter-spoken");
        assert_eq!(j["code"]["text"], "Polish");
        assert_eq!(j["category"][0]["coding"][0]["code"], "interpreting");
        assert_eq!(j["subject"]["reference"], "Patient/P1");
        assert_eq!(j["meta"]["versionId"], "2");
        assert!(j.get("period").is_none());
    }

    #[test]
    fn an_ended_adjustment_is_inactive_with_its_end() {
        let j = to_fhir(&stored(Status::Ended, None), "P1").unwrap();
        assert_eq!(j["status"], "inactive");
        assert_eq!(j["period"]["end"], "2026-11-05T09:31:00.123456Z");
        assert!(j["code"].get("text").is_none());
    }

    #[test]
    fn the_reason_stays_here() {
        let text = to_fhir(&stored(Status::Active, None), "P1")
            .unwrap()
            .to_string();
        assert!(!text.contains("daughter"), "{text}");
    }
}
