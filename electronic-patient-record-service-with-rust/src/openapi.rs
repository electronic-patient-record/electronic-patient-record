//! The OpenAPI document (task OS-7), served at `GET /api/openapi.json` and kept in
//! `openapi.json` at the crate root.
//!
//! It is **derived**, never written by hand: the paths and methods are the routes the service
//! really serves ([`crate::controllers::all_routes`]), and each operation carries the resource and
//! action of its row in the policy table ([`crate::routes::TABLE`]). A route that is served but
//! missing from the document is therefore not possible, and `tests/openapi_census.rs` fails when
//! the committed copy differs from what this module produces.
//!
//! **What it describes of bodies:** the error bodies, which `api_error.rs` builds in one place
//! (`components.schemas.Error`, `ErrorList` and `OperationOutcome`, referenced by the 401 and 403
//! responses; a unit test checks real `ApiError` bodies against the required keys). **What it does
//! not describe:** any request body or success response except the patient create and update pair below, so a client cannot be generated from it.
//! The patient identity record (`Patient`, `Stored`, `CreatePatient`, `UpdatePatient`) is described for
//! `POST /api/patients` and `PUT /api/patients/{id}`; unit tests compare each schema's property names
//! with the fields the real types serialise, in both directions. It is an inventory of the surface with its authentication and authorization requirements, not
//! a contract. FHIR resource shapes are in `/fhir/R4/metadata`.

use crate::{auth, policy::Resource, routes};
use axa::prelude::*;
use axa::problem::PROBLEM_JSON;
use axum::{Json, response::Response};
use serde_json::{Map, Value, json};

/// The served `(method, path template)` pairs, normalised to `{name}` segments.
#[must_use]
pub fn served() -> Vec<(String, String)> {
    routes::mounted()
        .into_iter()
        .map(|(method, path)| (method, routes::normalise(&path)))
        .collect()
}

/// The operation id of a route: the lower-case method followed by the path's segments in capitals with the braces
/// removed, so `GET /api/patients/{id}` is `getApiPatientsId`. The census requires them to be unique.
fn operation_id(method: &str, path: &str) -> String {
    let words: Vec<String> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let s = s.trim_start_matches('{').trim_end_matches('}');
            let mut c = s.chars();
            c.next().map_or_else(String::new, |f| {
                f.to_uppercase().collect::<String>() + c.as_str()
            })
        })
        .collect();
    format!("{}{}", method.to_ascii_lowercase(), words.concat())
}

/// The tag a route is grouped under: the segment after `api` (`/api/patients/{id}` is `patients`), `fhir-r4` or `fhir-r5`
/// for the FHIR routes, else the first segment with any leading underscore removed, else `root`.
fn tag(path: &str) -> String {
    let mut parts = path.split('/').filter(|s| !s.is_empty());
    match (parts.next(), parts.next()) {
        (Some("api"), Some(second)) => second.to_string(),
        (Some("fhir"), Some(release)) => format!("fhir-{}", release.to_ascii_lowercase()),
        (Some(first), _) => first.trim_start_matches('_').to_string(),
        _ => "root".to_string(),
    }
}

/// The `parameters` entries for the `{name}` segments of a path: each a required string in the path.
fn path_parameters(path: &str) -> Vec<Value> {
    path.split('/')
        .filter_map(|s| s.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
        .map(|name| {
            json!({"name": name, "in": "path", "required": true, "schema": {"type": "string"}})
        })
        .collect()
}

/// The route table's name for a resource kind, as the document's `x-resource` extension writes it.
fn resource_name(resource: Resource) -> String {
    format!("{resource:?}")
}

/// The media type and schema of an error body on `path`: an `OperationOutcome` under `/fhir`.
fn error_ref(path: &str) -> (&'static str, Value) {
    if path.starts_with("/fhir/") {
        (
            "application/fhir+json",
            json!({"$ref": "#/components/schemas/OperationOutcome"}),
        )
    } else {
        (PROBLEM_JSON, json!({"$ref": "#/components/schemas/Error"}))
    }
}

/// A named body schema under its media type: `application/problem+json` for a problem, the error
/// bodies `api_error.rs` builds, and `application/json` for anything else.
fn body_ref(description: &str, name: &str) -> Value {
    let media = if matches!(
        name,
        "Error" | "ErrorList" | "DuplicateConflict" | "VersionConflict"
    ) {
        PROBLEM_JSON
    } else {
        "application/json"
    };
    json!({"description": description, "content": {media: {
        "schema": {"$ref": format!("#/components/schemas/{name}")}}}})
}

/// An RFC 9457 body: the members `ApiError::problem` always sends, then this schema's own.
fn problem_schema(description: &str, required: &[&str], properties: &Value) -> Value {
    let mut all = json!({
        "type": {"type": "string"},
        "title": {"type": "string"},
        "status": {"type": "integer"},
        "detail": {"type": "string"}
    });
    all.as_object_mut()
        .expect("an object")
        .extend(properties.as_object().expect("an object").clone());
    let mut names = vec!["type", "title", "status", "detail"];
    names.extend_from_slice(required);
    json!({
        "type": "object", "required": names, "properties": all,
        "additionalProperties": true, "description": description
    })
}

/// The patient identity schemas, matching the matcher's `Patient`, `patients_store::Stored` and the
/// request types in `controllers/patients.rs`.
fn identity_schemas() -> Value {
    let pair = json!({"type": "object", "required": ["system", "value"],
        "properties": {"system": {"type": "string"}, "value": {"type": "string"}}});
    json!({
        "Patient": {
            "type": "object",
            "description": "The thin identity record (the matcher's Patient). Absent evidence is null or empty, never zero.",
            "properties": {
                "id": {"type": ["string", "null"]},
                "names": {"type": "array", "items": {"type": "object", "properties": {
                    "family": {"type": ["string", "null"]},
                    "given": {"type": "array", "items": {"type": "string"}}}}},
                "birth_date": {"type": ["string", "null"], "description": "ISO 8601; may be partial (YYYY, YYYY-MM, YYYY-MM-DD)."},
                "sex": {"enum": ["male", "female", "other", "unknown", null]},
                "identifiers": {"type": "array", "items": pair.clone()},
                "addresses": {"type": "array", "items": {"type": "object", "properties": {
                    "postal_code": {"type": ["string", "null"]}}}},
                "telecom": {"type": "array", "items": pair},
                "temporary_identity": {"type": "boolean", "default": false, "description": "Set by a person for someone who cannot yet be identified. The names and birth date are then not evidence (matcher rule 7), and the record can never be linked automatically. Absent means false. Every write replaces the whole record, so a write that drops this needs a reason, or it is refused with temporary_identity_reason_required."}
            }
        },
        "Stored": {
            "type": "object", "required": ["id", "version", "author", "patient"],
            "properties": {
                "id": {"type": "string", "description": "Server-assigned UUID."},
                "version": {"type": "integer", "description": "1 for the first version."},
                "author": {"type": "string"},
                "patient": {"$ref": "#/components/schemas/Patient"},
                "drill": {"type": "boolean"}
            }
        },
        "CreatePatient": {
            "type": "object", "required": ["patient"],
            "properties": {
                "patient": {"$ref": "#/components/schemas/Patient"},
                "reason": {"type": ["string", "null"]},
                "override_reason": {"type": ["string", "null"], "description": "Register despite likely duplicates (409 otherwise); a reason is required."},
                "drill": {"type": "boolean"}
            }
        },
        "UpdatePatient": {
            "type": "object", "required": ["patient", "expected_version"],
            "properties": {
                "patient": {"$ref": "#/components/schemas/Patient"},
                "expected_version": {"type": "integer", "description": "Any other current version is a 409."},
                "reason": {"type": ["string", "null"]}
            }
        },
        "PatientView": {
            "type": "object",
            "required": ["id", "version", "author", "patient", "replaces", "site_time_zone"],
            "description": "A patient as read: the stored version plus the photo version, merge status and the site time zone.",
            "properties": {
                "id": {"type": "string"},
                "version": {"type": "integer"},
                "author": {"type": "string"},
                "patient": {"$ref": "#/components/schemas/Patient"},
                "drill": {"type": "boolean"},
                "photo_version": {"type": ["integer", "null"], "description": "Null when there is no photo or it was withdrawn."},
                "replaced_by": {"type": ["string", "null"], "description": "The surviving record, when this one was merged away."},
                "replaces": {"type": "array", "items": {"type": "string"}},
                "site_time_zone": {"type": "string"}
            }
        },
        "PatientSearchResults": {
            "type": "object", "required": ["results", "limit"],
            "properties": {
                "results": {"type": "array", "items": {"$ref": "#/components/schemas/Stored"}},
                "limit": {"type": "integer", "description": "The most results returned."}
            }
        },
        "DuplicateConflict": problem_schema(
            "409 on create: likely duplicates. A candidate the caller may not see is {\"restricted\": true}.",
            &["candidates", "candidates_cut"],
            &json!({
                "candidates": {"type": "array", "items": {"type": "object"}},
                "candidates_cut": {"type": "boolean"}
            }),
        ),
        "DependencyReport": {
            "type": "object", "required": ["ok", "checked", "degraded"],
            "description": "What was asked and what did not answer, as labels: database, keycloak, paseto-keys.",
            "properties": {
                "ok": {"type": "boolean"},
                "checked": {"type": "array", "items": {"type": "string"}},
                "degraded": {"type": "array", "items": {"type": "string"}}
            }
        },
        "PhotoVersion": {
            "type": "object", "required": ["version", "withdrawn"],
            "description": "What a photo write answers: the new version, and whether it withdrew the photo.",
            "properties": {"version": {"type": "integer"}, "withdrawn": {"type": "boolean"}}
        },
        "VersionConflict": problem_schema(
            "409 on update: expected_version is not the current one.",
            &["current_version"],
            &json!({"current_version": {"type": "integer"}}),
        )
    })
}

/// The schemas for allergies: the record, the stored version, the create and update bodies, the "no known allergies"
/// statement and the list. A unit test compares their property names with what the real types serialise.
fn allergy_schemas() -> Value {
    json!({
        "AllergyRecord": {
            "type": "object", "required": ["substance", "status"],
            "description": "One allergy as recorded. An absent optional field means not stated; criticality is never defaulted to low.",
            "properties": {
                "substance": {"type": "string", "maxLength": 200, "description": "In words; required."},
                "coding": {"type": "object", "required": ["system", "code"], "properties": {
                    "system": {"type": "string", "description": "A URI; a SNOMED CT code is checked."},
                    "code": {"type": "string", "maxLength": 64}}},
                "category": {"enum": ["food", "medication", "environment", "biologic"]},
                "criticality": {"enum": ["low", "high", "unable-to-assess"]},
                "reaction": {"type": "string", "maxLength": 500},
                "onset": {"type": "string", "description": "YYYY, YYYY-MM or YYYY-MM-DD."},
                "status": {"enum": ["active", "inactive", "entered-in-error"]}
            }
        },
        "StoredAllergy": {
            "type": "object",
            "required": ["id", "patient_id", "version", "author", "recorded_at", "allergy"],
            "properties": {
                "id": {"type": "string"}, "patient_id": {"type": "string"},
                "version": {"type": "integer"}, "author": {"type": "string"},
                "recorded_at": {"type": "string", "description": "UTC."},
                "allergy": {"$ref": "#/components/schemas/AllergyRecord"}
            }
        },
        "CreateAllergy": {
            "type": "object", "required": ["allergy"],
            "properties": {
                "allergy": {"$ref": "#/components/schemas/AllergyRecord"},
                "reason": {"type": ["string", "null"]}
            }
        },
        "UpdateAllergy": {
            "type": "object", "required": ["allergy", "expected_version"],
            "properties": {
                "allergy": {"$ref": "#/components/schemas/AllergyRecord"},
                "expected_version": {"type": "integer"},
                "reason": {"type": ["string", "null"]}
            }
        },
        "CreateAllergyStatement": {
            "type": "object", "required": ["none_known", "expected_version"],
            "description": "State, or withdraw, \"no known allergies\". Refused with 422 if an active allergy exists, or if there is no statement to withdraw.",
            "properties": {
                "none_known": {"type": "boolean", "description": "true states it; false withdraws an earlier statement."},
                "expected_version": {"type": "integer", "description": "The statement version last seen; 0 for none."},
                "reason": {"type": ["string", "null"]}
            }
        },
        "AllergyStatement": {
            "type": "object", "required": ["version", "author", "recorded_at", "no_known_allergies"],
            "properties": {
                "version": {"type": "integer", "description": "1 for the first statement."},
                "author": {"type": "string"},
                "recorded_at": {"type": "string", "description": "UTC."},
                "no_known_allergies": {"type": "boolean", "description": "false when the statement was withdrawn."}
            }
        },
        "AllergyList": {
            "type": "object", "required": ["allergies", "statement", "summary", "amendment_notices"],
            "description": "Every allergy in every status, the no-known-allergies statement, the banner summary and amendment notices by allergy id. The statement, summary and notices are described only as objects.",
            "properties": {
                "allergies": {"type": "array", "items": {"$ref": "#/components/schemas/StoredAllergy"}},
                "statement": {"type": ["object", "null"]},
                "summary": {"type": "object", "required": ["state"], "additionalProperties": true, "properties": {"state": {"type": "string"}}},
                "amendment_notices": {"type": "object"}
            }
        }
    })
}

/// The schemas for referrals: the record, a stored version (with its `next` events), the send and step bodies, and the
/// list and history. The state and event words are checked against `State::ALL` and `Event::ALL` by a unit test.
fn referral_schemas() -> Value {
    json!({
        "ReferralRecord": {
            "type": "object", "required": ["recipient", "reason", "question", "priority"],
            "description": "The request. It does not change after it is made; the steps are in the versions.",
            "properties": {
                "recipient": {"type": "string", "description": "The service or clinician, as the referrer names it."},
                "reason": {"type": "string"},
                "question": {"type": "string"},
                "priority": {"enum": ["routine", "urgent", "asap", "stat"]},
                "external_ids": {"type": "array", "items": {"type": "object", "required": ["system", "value"], "properties": {
                    "system": {"type": "string"}, "value": {"type": "string"}}}}
            }
        },
        "StoredReferral": {
            "type": "object",
            "required": ["id", "version", "author", "recorded_at", "patient_id", "state", "referral", "last_event", "next"],
            "description": "One version of a referral. States marked provisional in spec/interoperability/closed-loop-referral.md are information-requested, deferred and redirected.",
            "properties": {
                "id": {"type": "string"}, "version": {"type": "integer"},
                "author": {"type": "string", "description": "Who wrote this version."},
                "recorded_at": {"type": "string", "description": "UTC."},
                "patient_id": {"type": "string"},
                "state": {"enum": ["requested","accepted","declined","appointment-booked","no-show","interim-report","completed","cancel-requested","cancelled","information-requested","deferred","redirected"]},
                "referral": {"$ref": "#/components/schemas/ReferralRecord"},
                "last_event": {"type": "object", "description": "The event, who sent it, in which role, when, and the note."},
                "next": {"type": "array", "description": "What the state table allows next; empty in an end state.", "items": {"type": "object", "required": ["event", "by", "needs_note"], "properties": {
                    "event": {"type": "string"}, "by": {"enum": ["referrer", "recipient"]}, "needs_note": {"type": "boolean"}}}}
            }
        },
        "CreateReferral": {
            "type": "object", "required": ["referral"],
            "properties": {"referral": {"$ref": "#/components/schemas/ReferralRecord"}}
        },
        "ReferralEvent": {
            "type": "object", "required": ["event", "expected_version"],
            "description": "The next step. The sender of the referral acts as the referrer; any other subject with the permission acts as the recipient (OQ-PC2, assumed). Refused with 422 when the state table does not allow the event, the role is wrong, or a needed note is missing.",
            "properties": {
                "event": {"enum": ["accept","decline","book","record-no-show","interim-note","outcome","request-cancel","confirm-cancel","request-information","provide-information","defer","resume","redirect"]},
                "expected_version": {"type": "integer"},
                "note": {"type": ["string", "null"]}
            }
        },
        "ReferralList": {
            "type": "object", "required": ["referrals"],
            "properties": {"referrals": {"type": "array", "items": {"$ref": "#/components/schemas/StoredReferral"}}}
        },
        "ReferralHistory": {
            "type": "object", "required": ["versions"],
            "properties": {"versions": {"type": "array", "items": {"$ref": "#/components/schemas/StoredReferral"}}}
        }
    })
}

/// The schemas for observations (vital signs): the record, the stored version, the create and update bodies, the list and
/// the version history.
fn observation_schemas() -> Value {
    json!({
        "ObservationRecord": {
            "type": "object", "required": ["code", "effective_at", "status"],
            "description": "A vital sign. A value needs a unit; no value needs an absent_reason; absence is never zero.",
            "properties": {
                "code": {"type": "string", "description": "LOINC."},
                "value": {"type": "number"},
                "unit": {"type": "string", "description": "UCUM; required with a value."},
                "absent_reason": {"enum": ["not-done", "declined", "failed", "unknown"]},
                "range": {"type": "object", "properties": {"low": {"type": "number"}, "high": {"type": "number"}}, "description": "Either end may be absent (unbounded); inclusive."}, "critical": {"type": "object", "properties": {"low": {"type": "number"}, "high": {"type": "number"}}, "description": "Either end may be absent (unbounded); inclusive."},
                "effective_at": {"type": "string", "description": "RFC 3339; stored and returned in UTC."},
                "status": {"enum": ["final", "entered-in-error"]}
            }
        },
        "StoredObservation": {
            "type": "object",
            "required": ["id", "patient_id", "version", "author", "recorded_at", "site_zone", "code", "effective_at", "status", "flag"],
            "description": "The ObservationRecord fields sit beside these, not under a key.",
            "properties": {
                "id": {"type": "string"}, "patient_id": {"type": "string"},
                "version": {"type": "integer"}, "author": {"type": "string"},
                "recorded_at": {"type": "string"}, "site_zone": {"type": "string"},
                "flag": {"enum": ["critical-low", "low", "normal", "high", "critical-high", null],
                         "description": "Computed now; null when it cannot be said."},
                "code": {"type": "string"}, "value": {"type": "number"}, "unit": {"type": "string"},
                "absent_reason": {"enum": ["not-done", "declined", "failed", "unknown"]},
                "range": {"type": "object", "properties": {"low": {"type": "number"}, "high": {"type": "number"}}, "description": "Either end may be absent (unbounded); inclusive."}, "critical": {"type": "object", "properties": {"low": {"type": "number"}, "high": {"type": "number"}}, "description": "Either end may be absent (unbounded); inclusive."},
                "effective_at": {"type": "string"},
                "status": {"enum": ["final", "entered-in-error"]}
            }
        },
        "CreateObservation": {
            "type": "object", "required": ["observation"],
            "properties": {
                "observation": {"$ref": "#/components/schemas/ObservationRecord"},
                "reason": {"type": ["string", "null"]}
            }
        },
        "UpdateObservation": {
            "type": "object", "required": ["observation", "expected_version"],
            "properties": {
                "observation": {"$ref": "#/components/schemas/ObservationRecord"},
                "expected_version": {"type": "integer"},
                "reason": {"type": ["string", "null"]}
            }
        },
        "ObservationList": {
            "type": "object", "required": ["observations", "amendment_notices"],
            "properties": {
                "observations": {"type": "array", "items": {"$ref": "#/components/schemas/StoredObservation"}},
                "amendment_notices": {"type": "object", "description": "By observation id; described only as an object."}
            }
        },
        "ObservationVersions": {
            "type": "object", "required": ["versions"],
            "description": "Oldest first.",
            "properties": {"versions": {"type": "array", "items": {"$ref": "#/components/schemas/ObservationVersion"}}}
        }
    })
}

/// The schemas for assessments under versioned forms (WL-3).
fn assessment_schemas() -> Value {
    json!({
        "AssessmentForm": {
            "type": "object", "required": ["id", "version", "title", "approved_by", "approved_on", "question"],
            "description": "One version of a form, from `src/assessment_forms.toml`. Empty approver and day mean a draft.",
            "properties": {
                "id": {"type": "string"}, "version": {"type": "integer"}, "title": {"type": "string"},
                "approved_by": {"type": "string"}, "approved_on": {"type": "string"},
                "question": {"type": "array", "items": {
                    "type": "object", "required": ["id", "text", "kind"],
                    "properties": {
                        "id": {"type": "string"}, "text": {"type": "string"},
                        "kind": {"enum": ["yes-no", "choice"]},
                        "choices": {"type": "array", "items": {"type": "string"}}
                    }
                }}
            }
        },
        "AssessmentRecord": {
            "type": "object", "required": ["form", "form_version", "answers"], "additionalProperties": false,
            "properties": {
                "form": {"type": "string"}, "form_version": {"type": "integer"},
                "answers": {"type": "array", "items": {
                    "type": "object", "required": ["question"], "additionalProperties": false,
                    "description": "Every question once: a value (a boolean for yes-no, a listed string for a choice), or null with absent_reason.",
                    "properties": {
                        "question": {"type": "string"},
                        "value": {},
                        "absent_reason": {"type": ["string", "null"], "maxLength": 200}
                    }
                }}
            }
        },
        "CreateAssessment": {
            "type": "object", "required": ["assessment"], "additionalProperties": false,
            "properties": {"assessment": {"$ref": "#/components/schemas/AssessmentRecord"}}
        },
        "StoredAssessment": {
            "type": "object", "required": ["id", "patient_id", "version", "author", "recorded_at", "form", "answers"],
            "properties": {
                "id": {"type": "string"}, "patient_id": {"type": "string"},
                "version": {"type": "integer"}, "author": {"type": "string"}, "recorded_at": {"type": "string"},
                "form": {
                    "type": "object",
                    "required": ["id", "version", "title", "approved", "approved_by", "approved_on", "latest_version"],
                    "description": "The form version the answers were given under, and the newest version.",
                    "properties": {
                        "id": {"type": "string"}, "version": {"type": "integer"}, "title": {"type": "string"},
                        "approved": {"type": "boolean"},
                        "approved_by": {"type": ["string", "null"]}, "approved_on": {"type": ["string", "null"]},
                        "latest_version": {"type": "integer"}
                    }
                },
                "answers": {"type": "array", "items": {
                    "type": "object", "required": ["question", "text", "value", "absent_reason"],
                    "properties": {
                        "question": {"type": "string"},
                        "text": {"type": "string", "description": "As that form version asked it."},
                        "value": {}, "absent_reason": {"type": ["string", "null"]}
                    }
                }}
            }
        },
        "AssessmentList": {
            "type": "object", "required": ["forms", "assessments"],
            "properties": {
                "forms": {"type": "array", "items": {"$ref": "#/components/schemas/AssessmentForm"}},
                "assessments": {"type": "array", "items": {"$ref": "#/components/schemas/StoredAssessment"}}
            }
        }
    })
}

/// The schemas for results and their sign-off (OR-4 first slice, WL-1).
fn result_schemas() -> Value {
    let record = json!({
        "type": "object", "required": ["test", "status", "issued"], "additionalProperties": false,
        "description": "One version of a result, as the reporting service sent it. Text fields at most 500 characters.",
        "properties": {
            "test": {"type": "string"},
            "value": {"type": ["string", "null"], "description": "As reported, text, so that \"<0.5\" survives."},
            "unit": {"type": ["string", "null"]},
            "reference_range": {"type": ["string", "null"]},
            "interpretation": {"enum": ["normal", "abnormal", "critical", null], "description": "Absent means the service said nothing, never normal."},
            "status": {"enum": ["preliminary", "final", "amended", "corrected", "entered-in-error"]},
            "issued": {"type": "string", "description": "RFC 3339."},
            "performer": {"type": ["string", "null"]}
        }
    });
    json!({
        "ResultRecord": record,
        "StoredResult": {
            "type": "object", "required": ["id", "patient_id", "version", "author", "recorded_at", "reason", "result"],
            "properties": {
                "id": {"type": "string"}, "patient_id": {"type": "string"},
                "version": {"type": "integer"}, "author": {"type": "string"},
                "recorded_at": {"type": "string"},
                "reason": {"type": ["string", "null"], "description": "Why it changed; null on a first version."},
                "result": {"$ref": "#/components/schemas/ResultRecord"}
            }
        },
        "ResultSignOff": {
            "type": "object", "required": ["id", "result_id", "version", "signed_by", "signed_at"],
            "properties": {
                "id": {"type": "string"}, "result_id": {"type": "string"},
                "version": {"type": "integer", "description": "The version that was signed off."},
                "signed_by": {"type": "string"}, "signed_at": {"type": "string"}
            }
        },
        "ResultView": {
            "type": "object", "required": ["current", "sign_off", "last_sign_off", "signed_version"],
            "properties": {
                "current": {"$ref": "#/components/schemas/StoredResult"},
                "sign_off": {"enum": ["unsigned", "signed", "changed-since-sign-off", "not-applicable"]},
                "last_sign_off": {"oneOf": [{"$ref": "#/components/schemas/ResultSignOff"}, {"type": "null"}]},
                "signed_version": {"oneOf": [{"$ref": "#/components/schemas/StoredResult"}, {"type": "null"}],
                                    "description": "When changed since sign-off: the version that was signed, to show beside the current."}
            }
        },
        "ResultList": {
            "type": "object", "required": ["results"],
            "properties": {"results": {"type": "array", "items": {"$ref": "#/components/schemas/ResultView"}}}
        },
        "CreateResult": {
            "type": "object", "required": ["result"], "additionalProperties": false,
            "properties": {"result": {"$ref": "#/components/schemas/ResultRecord"}}
        },
        "UpdateResult": {
            "type": "object", "required": ["result", "expected_version", "reason"], "additionalProperties": false,
            "properties": {
                "result": {"$ref": "#/components/schemas/ResultRecord"},
                "expected_version": {"type": "integer"},
                "reason": {"type": "string"}
            }
        },
        "SignOffResult": {
            "type": "object", "required": ["version"], "additionalProperties": false,
            "properties": {"version": {"type": "integer", "description": "The version the signer read; an older one is a 409."}}
        }
    })
}

/// The schemas for consents: the record with its provisions, the stored version, the create and update bodies and the list.
fn consent_schemas() -> Value {
    json!({
        "ConsentRecord": {
            "type": "object", "required": ["status"],
            "description": "Provisions are required for a general consent and none for a national-data-opt-out. The opt-out is held by this service only; no national check is built.",
            "properties": {
                "kind": {"enum": ["general", "national-data-opt-out"]},
                "status": {"enum": ["active", "inactive", "entered-in-error"]},
                "provisions": {"type": "array", "items": {"type": "object", "required": ["effect"], "properties": {
                    "effect": {"enum": ["permit", "deny"]},
                    "actors": {"type": "array", "items": {"type": "string"}}, "purposes": {"type": "array", "items": {"type": "string"}}, "resources": {"type": "array", "items": {"type": "string"}}, "labels": {"type": "array", "items": {"type": "string"}},
                    "period": {"type": ["object", "null"], "properties": {"start": {"type": ["string", "null"]}, "end": {"type": ["string", "null"]}}, "description": "YYYY-MM-DD (UTC), both ends optional and inclusive."}}, "description": "An empty list means any."}},
                "period": {"type": ["object", "null"], "properties": {"start": {"type": ["string", "null"]}, "end": {"type": ["string", "null"]}}, "description": "YYYY-MM-DD (UTC), both ends optional and inclusive."}
            }
        },
        "StoredConsent": {
            "type": "object", "required": ["id", "patient_id", "version", "author", "consent"],
            "properties": {
                "id": {"type": "string"}, "patient_id": {"type": "string"},
                "version": {"type": "integer"}, "author": {"type": "string"},
                "consent": {"$ref": "#/components/schemas/ConsentRecord"}
            }
        },
        "CreateConsent": {
            "type": "object", "required": ["consent"],
            "properties": {"consent": {"$ref": "#/components/schemas/ConsentRecord"}, "reason": {"type": ["string", "null"]}}
        },
        "UpdateConsent": {
            "type": "object", "required": ["consent", "expected_version"],
            "properties": {"consent": {"$ref": "#/components/schemas/ConsentRecord"},
                "expected_version": {"type": "integer"}, "reason": {"type": ["string", "null"]}}
        },
        "ConsentList": {
            "type": "object", "required": ["consents"],
            "properties": {"consents": {"type": "array", "items": {"$ref": "#/components/schemas/StoredConsent"}}}
        }
    })
}

/// The schemas for care relationships: the record (with a proxy's terms), the stored version, the bodies and the list.
fn relationship_schemas() -> Value {
    json!({
        "RelationshipRecord": {
            "type": "object", "required": ["subject", "kind", "status"],
            "description": "Who may treat or act for a patient. proxy is required for kind proxy and refused for every other kind.",
            "properties": {
                "subject": {"type": "string", "description": "The token sub of the person."},
                "kind": {"enum": ["care-team", "active-encounter", "self", "proxy"]},
                "status": {"enum": ["active", "ended"]},
                "period": {"type": ["object", "null"], "properties": {"start": {"type": ["string", "null"]}, "end": {"type": ["string", "null"]}}, "description": "YYYY-MM-DD (UTC), both ends optional and inclusive."},
                "proxy": {"type": ["object", "null"], "required": ["basis", "scopes", "expires_at"], "properties": {
                    "basis": {"enum": ["parental-responsibility", "power-of-attorney", "deputyship", "patient-request"]},
                    "scopes": {"type": "array", "items": {"enum": ["identity", "allergies", "photo", "access-log", "adjustments"]}},
                    "expires_at": {"type": "integer", "description": "Unix seconds."}}}
            }
        },
        "StoredRelationship": {
            "type": "object", "required": ["id", "patient_id", "version", "author", "relationship"],
            "properties": {
                "id": {"type": "string"}, "patient_id": {"type": "string"},
                "version": {"type": "integer"}, "author": {"type": "string"},
                "relationship": {"$ref": "#/components/schemas/RelationshipRecord"}
            }
        },
        "CreateRelationship": {
            "type": "object", "required": ["relationship"],
            "properties": {"relationship": {"$ref": "#/components/schemas/RelationshipRecord"},
                "reason": {"type": ["string", "null"], "description": "For self or proxy, how identity was checked."}}
        },
        "UpdateRelationship": {
            "type": "object", "required": ["relationship", "expected_version"],
            "properties": {"relationship": {"$ref": "#/components/schemas/RelationshipRecord"},
                "expected_version": {"type": "integer"}, "reason": {"type": ["string", "null"]}}
        },
        "RelationshipList": {
            "type": "object", "required": ["relationships"],
            "properties": {"relationships": {"type": "array", "items": {"$ref": "#/components/schemas/StoredRelationship"}}}
        }
    })
}

/// The schemas for amendment requests: the create body, the step body, the view with its notice and due time, and the list.
fn amendment_schemas() -> Value {
    json!({
        "CreateAmendment": {
            "type": "object", "required": ["kind", "target_id", "version", "text"],
            "description": "A patient asks for a record to be corrected. Who asks is the token, never the body.",
            "properties": {
                "kind": {"enum": ["Observation", "AllergyIntolerance"]},
                "target_id": {"type": "string"},
                "version": {"type": "integer", "description": "The version the patient read."},
                "text": {"type": "string", "description": "What should change, in the patient's words."}
            }
        },
        "UpdateAmendment": {
            "type": "object", "required": ["expected_version", "step"],
            "description": "One step. accept, deny, apply and extend need the clinician role; withdraw and disagree need the patient role. Each step carries its own fields: accept and extend a reason; deny a ground and a reason; apply a version; disagree a statement; withdraw none.",
            "additionalProperties": true,
            "properties": {
                "expected_version": {"type": "integer"},
                "step": {"enum": ["accept", "deny", "apply", "extend", "withdraw", "disagree"]},
                "reason": {"type": "string"},
                "ground": {"enum": ["not-ours", "accurate-and-complete", "not-available-to-requester", "other"]},
                "version": {"type": "integer"},
                "statement": {"type": "string"}
            }
        },
        "AmendmentView": {
            "type": "object",
            "required": ["id", "patient_id", "version", "author", "recorded_at", "request", "notice", "due", "overdue"],
            "properties": {
                "id": {"type": "string"}, "patient_id": {"type": "string"},
                "version": {"type": "integer"}, "author": {"type": "string"},
                "recorded_at": {"type": "string"},
                "request": {"type": "object", "required": ["target", "requester", "text", "received_at", "extension", "state"],
                    "properties": {
                        "target": {"type": "object", "required": ["kind", "id", "version"], "properties": {
                            "kind": {"type": "string"}, "id": {"type": "string"}, "version": {"type": "integer"}}},
                        "requester": {"type": "string"},
                        "text": {"type": "string"},
                        "received_at": {"type": "string"},
                        "extension": {"type": ["string", "null"], "description": "The written reason for the one extension, once taken."},
                        "state": {"type": "object", "required": ["state"], "additionalProperties": true,
                            "description": "Tagged by state; accepted, applied and denied carry their own fields.",
                            "properties": {"state": {"enum": ["requested", "accepted", "applied", "denied", "withdrawn"]}}}
                    }},
                "notice": {"type": ["object", "null"], "required": ["notice"], "additionalProperties": true,
                    "description": "What a reader of the record must be told; null when there is nothing to tell (seen after a withdrawal).",
                    "properties": {"notice": {"enum": ["pending", "amended", "refused"]}}},
                "due": {"type": "string", "description": "When a decision is due (RFC 3339)."},
                "overdue": {"type": "boolean"}
            }
        },
        "AmendmentList": {
            "type": "object", "required": ["amendments"],
            "properties": {"amendments": {"type": "array", "items": {"$ref": "#/components/schemas/AmendmentView"}}}
        },
        "DeathOccurred": {
            "type": "object", "required": ["kind"],
            "description": "When the person died (`known`, with `at`), or why that is not known (`not-known`, with `reason`).",
            "properties": {
                "kind": {"enum": ["known", "not-known"]},
                "at": {"type": "string", "description": "`YYYY`, `YYYY-MM`, `YYYY-MM-DD` or an RFC 3339 instant."},
                "reason": {"type": "string"}
            }
        },
        "ReportDeath": {
            "type": "object", "required": ["source", "occurred", "reason"], "additionalProperties": false,
            "properties": {
                "source": {"enum": ["registrar", "national-demographics", "hospital", "general-practice", "care-home", "coroner", "family-member", "other-system"]},
                "occurred": {"$ref": "#/components/schemas/DeathOccurred"},
                "reason": {"type": "string"}
            }
        },
        "StepDeathAssertion": {
            "type": "object", "required": ["expected_version", "to", "reason"], "additionalProperties": false,
            "properties": {
                "expected_version": {"type": "integer"},
                "to": {"enum": ["verified", "certified", "registered", "withdrawn-in-error"], "description": "Withdrawal needs the clinician role."},
                "reason": {"type": "string"}
            }
        },
        "DeathAssertion": {
            "type": "object", "required": ["id", "patient_id", "version", "author", "recorded_at", "assertion"],
            "properties": {
                "id": {"type": "string"},
                "patient_id": {"type": "string"},
                "version": {"type": "integer"},
                "author": {"type": "string"},
                "recorded_at": {"type": "string"},
                "assertion": {
                    "type": "object", "required": ["source", "reported_by", "occurred", "status", "reason"],
                    "properties": {
                        "source": {"type": "string"},
                        "reported_by": {"type": "object", "required": ["kind", "id"],
                            "properties": {"kind": {"enum": ["person", "feed"]}, "id": {"type": "string"}}},
                        "occurred": {"$ref": "#/components/schemas/DeathOccurred"},
                        "status": {"enum": ["reported", "verified", "certified", "registered", "withdrawn-in-error"]},
                        "reason": {"type": "string"}
                    }
                }
            }
        },
        "DeathAssertionList": {
            "type": "object", "required": ["assertions"],
            "properties": {"assertions": {"type": "array", "items": {"$ref": "#/components/schemas/DeathAssertion"}}}
        },
        "DeathAnswer": {
            "type": "object", "required": ["answer", "holds_contact"],
            "description": "`not-known-to-have-died` never means the person is known to be living.",
            "properties": {
                "answer": {"enum": ["not-known-to-have-died", "asserted"]},
                "status": {"enum": ["reported", "verified", "certified", "registered"]},
                "confirmed": {"type": "boolean"},
                "occurred": {"type": ["string", "null"]},
                "dates_differ": {"type": "boolean"},
                "holds_contact": {"type": "boolean"}
            }
        },
        "RecordAdjustment": {
            "type": "object", "required": ["code", "reason"], "additionalProperties": false,
            "properties": {
                "code": {"type": "string", "description": "A code from the catalogue in `src/adjustments.rs`."},
                "note": {"type": ["string", "null"], "maxLength": 200, "description": "A short note. `interpreter-spoken` needs the language here."},
                "review_by": {"type": ["string", "null"], "description": "`YYYY-MM-DD`, today or later."},
                "reason": {"type": "string"}
            }
        },
        "ChangeAdjustment": {
            "type": "object", "required": ["expected_version", "action", "reason"], "additionalProperties": false,
            "properties": {
                "expected_version": {"type": "integer"},
                "action": {"enum": ["review", "end"], "description": "`end` is final."},
                "review_by": {"type": ["string", "null"], "description": "For a review, the next date, `YYYY-MM-DD`."},
                "reason": {"type": "string"}
            }
        },
        "Adjustment": {
            "type": "object",
            "required": ["id", "patient_id", "version", "author", "recorded_at", "adjustment", "category", "display", "overdue"],
            "properties": {
                "id": {"type": "string"},
                "patient_id": {"type": "string"},
                "version": {"type": "integer"},
                "author": {"type": "string"},
                "recorded_at": {"type": "string"},
                "adjustment": {
                    "type": "object", "required": ["code", "note", "status", "review_by", "reason"],
                    "properties": {
                        "code": {"type": "string"},
                        "note": {"type": ["string", "null"]},
                        "status": {"enum": ["active", "ended"]},
                        "review_by": {"type": ["string", "null"]},
                        "reason": {"type": "string"}
                    }
                },
                "category": {"type": "string"},
                "display": {"type": "string"},
                "overdue": {"type": "boolean", "description": "The date to check it with the person has passed."}
            }
        },
        "AdjustmentList": {
            "type": "object", "required": ["adjustments"],
            "properties": {"adjustments": {"type": "array", "items": {"$ref": "#/components/schemas/Adjustment"}}}
        }
    })
}

/// The schemas for the hazard register: a hazard with its controls and evidence, the stored version, the create and step
/// bodies and the list.
fn hazard_schemas() -> Value {
    let evidence = json!({"type": "array", "maxItems": 20, "items": {"$ref": "#/components/schemas/HazardEvidence"}});
    json!({
        "CreateHazard": {
            "type": "object", "required": ["title", "severity", "likelihood"],
            "description": "Raise a hazard. Refused (422) if the title is blank or longer than 200 characters, or a rating is outside 1 to 5.",
            "properties": {
                "title": {"type": "string", "maxLength": 200},
                "severity": {"type": "integer", "minimum": 1, "maximum": 5, "description": "Before controls."},
                "likelihood": {"type": "integer", "minimum": 1, "maximum": 5, "description": "Before controls."}
            }
        },
        "UpdateHazard": {
            "type": "object", "required": ["expected_version", "event"],
            "description": "One step on a hazard, on the version last seen. `control` needs `text`; `retire` needs `evidence` and `note` and is refused with 403 unless the token holds the clinical-safety-officer role (the body never says who); `reopen` needs `reason`. Other fields for the event are refused.",
            "properties": {
                "expected_version": {"type": "integer"},
                "event": {"enum": ["control", "retire", "reopen"]},
                "text": {"type": "string", "maxLength": 2000, "description": "For control."},
                "evidence": evidence,
                "note": {"type": "string", "maxLength": 2000, "description": "For retire."},
                "reason": {"type": "string", "maxLength": 2000, "description": "For reopen."}
            }
        },
        "HazardEvidence": {
            "type": "object", "required": ["kind", "reference", "taken_at", "sha256"],
            "properties": {
                "kind": {"enum": ["removal-change", "test-run", "review-record", "census"]},
                "reference": {"type": "string"},
                "taken_at": {"type": "string", "description": "UTC; not in the future."},
                "sha256": {"type": "string", "description": "64 lowercase hex digits."}
            }
        },
        "HazardControl": {
            "type": "object", "required": ["by", "text"],
            "properties": {"by": {"type": "string"}, "text": {"type": "string"}}
        },
        "Hazard": {
            "type": "object",
            "required": ["title", "severity", "likelihood", "state", "raised_by", "controls", "evidence", "retired_by", "note", "reopened"],
            "properties": {
                "title": {"type": "string"},
                "severity": {"type": "integer"}, "likelihood": {"type": "integer"},
                "state": {"enum": ["open", "retired"]},
                "raised_by": {"type": "string"},
                "controls": {"type": "array", "items": {"$ref": "#/components/schemas/HazardControl"}},
                "evidence": {"type": "array", "items": {"$ref": "#/components/schemas/HazardEvidence"}},
                "retired_by": {"type": ["string", "null"]},
                "note": {"type": ["string", "null"]},
                "reopened": {"type": "integer", "description": "How many times it was reopened."}
            }
        },
        "StoredHazard": {
            "type": "object", "required": ["id", "version", "author", "recorded_at", "hazard"],
            "properties": {
                "id": {"type": "string"},
                "version": {"type": "integer", "description": "1 for the hazard as raised."},
                "author": {"type": "string"},
                "recorded_at": {"type": "string", "description": "UTC."},
                "hazard": {"$ref": "#/components/schemas/Hazard"}
            }
        },
        "HazardList": {
            "type": "object", "required": ["hazards"],
            "properties": {"hazards": {"type": "array", "description": "Newest first.", "items": {"$ref": "#/components/schemas/StoredHazard"}}}
        }
    })
}

const AUDIT_CRITERIA: &[&str] = &[
    "agent",
    "patient",
    "action",
    "outcome",
    "purpose",
    "break_glass",
    "from",
    "to",
    "after",
    "through",
    "before",
];

/// The schemas for the feed-refusal queue: an entry, its resolution, the resolve body, the list and the count.
fn feed_schemas() -> Value {
    json!({
        "ResolveFeedRefusal": {
            "type": "object", "required": ["outcome", "reason"],
            "description": "Close a refused feed message. The reason (1 to 500 characters) is required; a refusal already closed is a 409.",
            "properties": {
                "outcome": {"enum": ["handled", "dismissed"], "description": "handled: the cause was fixed or the content entered another way. dismissed: it needs nothing done."},
                "reason": {"type": "string", "maxLength": 500}
            }
        },
        "FeedResolution": {
            "type": ["object", "null"], "description": "Null while the refusal is open.",
            "required": ["resolved_by", "resolved_at", "outcome", "reason"],
            "properties": {
                "resolved_by": {"type": "string"},
                "resolved_at": {"type": "string", "description": "UTC."},
                "outcome": {"enum": ["handled", "dismissed"]},
                "reason": {"type": "string"}
            }
        },
        "FeedRefusal": {
            "type": "object",
            "required": ["id", "received_at", "agent", "peer", "event", "control_id", "kind", "reasons", "resolution"],
            "description": "One message the feed refused. `message`, the text as received, is present only when one refusal is read, never in a listing.",
            "properties": {
                "id": {"type": "string"},
                "received_at": {"type": "string", "description": "UTC."},
                "agent": {"type": "string", "description": "The feed's audit agent."},
                "peer": {"type": ["string", "null"], "description": "The sender's address."},
                "event": {"type": "string", "description": "The HL7 event such as A08, or ?."},
                "control_id": {"type": "string", "description": "The sender's message control id (MSH-10), or ?."},
                "kind": {"enum": ["unsupported", "conflict"]},
                "reasons": {"type": "array", "items": {"type": "string"}, "description": "In words; names no patient data."},
                "message": {"type": "string"},
                "resolution": {"$ref": "#/components/schemas/FeedResolution"}
            }
        },
        "FeedRefusalList": {
            "type": "object", "required": ["refusals", "limit", "more", "site_time_zone"],
            "description": "Open refusals, up to `limit`.",
            "properties": {
                "refusals": {"type": "array", "items": {"$ref": "#/components/schemas/FeedRefusal"}},
                "limit": {"type": "integer"}, "more": {"type": "boolean", "description": "True when more entries are waiting than `limit` allowed: the list is cut short and the newest are not shown."},
                "site_time_zone": {"type": "string"}
            }
        },
        "FeedRefusalCount": {
            "type": "object", "required": ["open"],
            "properties": {"open": {"type": "integer"}}
        }
    })
}

/// The schemas for inbound referral messages and the unmatched queue: the message, its result, the entry, the clear body,
/// the list and the count.
fn referral_queue_schemas() -> Value {
    json!({
        "ReferralMessage": {
            "type": "object", "required": ["message"],
            "description": "An inbound referral message (an outcome, a decline, an interim note). It moves the referral it names, or is queued for a person; it is never dropped and never matched by patient and date. 200 either way.",
            "properties": {
                "referral_ref": {"type": ["string", "null"], "description": "The referral id as the sender wrote it. Absent or blank is queued as no-id."},
                "event": {"type": ["string", "null"], "description": "A recipient event word such as outcome, decline or interim-note."},
                "note": {"type": ["string", "null"], "description": "Needed by the events that need one."},
                "message": {"type": "string", "description": "The message as received; required, cut at 1 MiB."}
            }
        },
        "ReferralMessageResult": {
            "type": "object", "required": ["result"],
            "description": "moved: the referral and its new version and state. queued: the queue id and why. The message is not repeated.",
            "properties": {
                "result": {"enum": ["moved", "queued"]},
                "referral": {"type": "string"}, "version": {"type": "integer"}, "state": {"type": "string"},
                "queue_id": {"type": "string"},
                "kind": {"enum": ["no-id", "unknown-id", "refused-by-table"]}
            }
        },
        "ClearReferralEntry": {
            "type": "object", "required": ["how", "reason"],
            "description": "Clear a queued message. matched needs referral_id and records the link without changing the referral; dismissed names none. The reason (1 to 500 characters) is required; an entry already cleared is a 409.",
            "properties": {
                "how": {"enum": ["matched", "dismissed"]},
                "referral_id": {"type": ["string", "null"]},
                "reason": {"type": "string", "maxLength": 500}
            }
        },
        "ReferralQueueEntry": {
            "type": "object",
            "required": ["id", "received_at", "agent", "kind", "referral_ref", "event", "reasons", "cleared"],
            "description": "One message that did not match. `note` and `message`, as received, are present only when one entry is read, never in a listing or after a write.",
            "properties": {
                "id": {"type": "string"}, "received_at": {"type": "string", "description": "UTC."},
                "agent": {"type": "string"},
                "kind": {"enum": ["no-id", "unknown-id", "refused-by-table"]},
                "referral_ref": {"type": ["string", "null"]}, "event": {"type": ["string", "null"]},
                "reasons": {"type": "array", "items": {"type": "string"}, "description": "In words; names no patient data."},
                "note": {"type": "string"}, "message": {"type": "string"},
                "cleared": {"type": ["object", "null"], "description": "Null while open.", "required": ["cleared_by", "cleared_at", "how", "referral_id", "reason"],
                    "properties": {
                        "cleared_by": {"type": "string"}, "cleared_at": {"type": "string", "description": "UTC."},
                        "how": {"enum": ["matched", "dismissed"]}, "referral_id": {"type": ["string", "null"]},
                        "reason": {"type": "string"}}}
            }
        },
        "ReferralQueueList": {
            "type": "object", "required": ["entries", "limit", "more", "site_time_zone"],
            "description": "Open entries, up to `limit`.",
            "properties": {
                "entries": {"type": "array", "items": {"$ref": "#/components/schemas/ReferralQueueEntry"}},
                "limit": {"type": "integer"}, "more": {"type": "boolean", "description": "True when more entries are waiting than `limit` allowed: the list is cut short and the newest are not shown."}, "site_time_zone": {"type": "string"}
            }
        },
        "ReferralQueueCount": {
            "type": "object", "required": ["open"],
            "properties": {"open": {"type": "integer"}}
        },
        "DeathReportItem": {
            "type": "object", "required": ["assertion_id", "patient_id", "version", "reported_at"],
            "description": "A report of a death that waits for a person. A pointer: no reason, source, reporter or date; a clinician or nurse reads those through the patient's route.",
            "properties": {
                "assertion_id": {"type": "string"},
                "patient_id": {"type": "string", "description": "The record to open: the surviving record, if the report was made on one merged away."},
                "version": {"type": "integer"},
                "reported_at": {"type": "string", "description": "UTC."}
            }
        },
        "DeathReportList": {
            "type": "object", "required": ["items", "limit", "more", "accountable_role", "site_time_zone"],
            "description": "Every report still at `reported`, oldest first, up to `limit`. The same for everyone who may read it.",
            "properties": {
                "items": {"type": "array", "items": {"$ref": "#/components/schemas/DeathReportItem"}},
                "limit": {"type": "integer"},
                "more": {"type": "boolean", "description": "True when more are waiting than `limit` allowed: the list is cut short and the newest are not shown."},
                "accountable_role": {"enum": ["on-duty-clinician"], "description": "The role accountable for what waits here. It names no person: the service does not know who holds it now."},
                "site_time_zone": {"type": "string"}
            }
        },
        "OverdueReviewItem": {
            "type": "object", "required": ["adjustment_id", "patient_id", "version", "review_by", "contact_held"],
            "description": "A need whose date to check it with the person has passed. A pointer: no code, note or reason; open the patient for those.",
            "properties": {
                "adjustment_id": {"type": "string"},
                "patient_id": {"type": "string", "description": "The record to open: the surviving record, if the need was recorded on one merged away."},
                "version": {"type": "integer"},
                "review_by": {"type": "string", "description": "`YYYY-MM-DD`, the day that has passed."},
                "contact_held": {"type": "boolean", "description": "A report of this person's death stands, confirmed or not, so nobody should contact them to check."}
            }
        },
        "OverdueReviewList": {
            "type": "object", "required": ["items", "limit", "more", "accountable_role"],
            "description": "Every active need whose date has passed, oldest date first, up to `limit`. The day is worked out in UTC.",
            "properties": {
                "items": {"type": "array", "items": {"$ref": "#/components/schemas/OverdueReviewItem"}},
                "limit": {"type": "integer"},
                "more": {"type": "boolean", "description": "True when more are overdue than `limit` allowed: the list is cut short and the most recent dates are not shown."},
                "accountable_role": {"enum": ["on-duty-clinician"], "description": "The role that owns this list. It names no person: the rota belongs to workforce planning."}
            }
        },
        "ReferralWorklistItem": {
            "type": "object", "required": ["referral_id", "patient_id", "version", "state", "why", "recorded_at"],
            "description": "One referral the caller sent that waits for them. A pointer: no recipient, reason, question or note; open the referral through the patient's route.",
            "properties": {
                "referral_id": {"type": "string"}, "patient_id": {"type": "string"},
                "version": {"type": "integer", "description": "The version that waits; acknowledge this one."},
                "state": {"enum": ["requested", "accepted", "declined", "appointment-booked", "no-show", "interim-report", "completed", "information-requested", "deferred", "redirected"]},
                "why": {"enum": ["reply", "no-response", "no-outcome"], "description": "reply: acknowledge it. no-response and no-outcome are timeouts (PC-3): they cannot be acknowledged and leave when the referral moves."},
                "recorded_at": {"type": "string", "description": "UTC."}
            }
        },
        "ReferralWorklistList": {
            "type": "object", "required": ["items", "limit", "more", "untimed", "site_time_zone"],
            "description": "The caller's waiting referrals, oldest first, up to `limit`. `untimed` names the priorities for which no timeout is configured (OQ-PC3): a referral of that priority never goes quiet on this list.",
            "properties": {
                "items": {"type": "array", "items": {"$ref": "#/components/schemas/ReferralWorklistItem"}},
                "limit": {"type": "integer"},
                "more": {"type": "boolean", "description": "True when more are waiting than `limit` allowed: the list is cut short and the newest are not shown."},
                "untimed": {"type": "object", "required": ["no_response", "no_outcome"], "properties": {
                    "no_response": {"type": "array", "items": {"enum": ["routine", "urgent", "asap", "stat"]}},
                    "no_outcome": {"type": "array", "items": {"enum": ["routine", "urgent", "asap", "stat"]}}}},
                "site_time_zone": {"type": "string"}
            }
        },
        "AcknowledgeReferral": {
            "type": "object", "required": ["version"],
            "description": "The referrer has seen this version. Anything but the current version is a 409; so is a repeat. A referral the caller did not send is a 404.",
            "properties": {"version": {"type": "integer"}}
        },
        "ReferralAcknowledged": {
            "type": "object", "required": ["referral", "version", "acknowledged"],
            "properties": {"referral": {"type": "string"}, "version": {"type": "integer"}, "acknowledged": {"type": "boolean"}}
        }
    })
}

/// The schemas for `/api/me`: the caller's own records and the access-log page.
fn me_schemas() -> Value {
    json!({
        "OwnRecord": {
            "type": "object", "required": ["relationship", "patient", "photo_version"],
            "description": "One record the caller is, or is a proxy for.",
            "properties": {
                "relationship": {"enum": ["self", "proxy"]},
                "patient": {"$ref": "#/components/schemas/Stored"},
                "photo_version": {"type": ["integer", "null"], "description": "Null when there is no photo or it was withdrawn."}
            }
        },
        "OwnRecordList": {
            "type": "object", "required": ["records"],
            "description": "Only records the full record-level decision allows; a refused one is left out (and the refusal audited).",
            "properties": {"records": {"type": "array", "items": {"$ref": "#/components/schemas/OwnRecord"}}}
        },
        "AccessLogEntry": {
            "type": "object",
            "required": ["id", "at", "by", "roles", "organisations", "did", "what", "refused", "purpose", "break_glass"],
            "description": "One access in words a patient can read. A staff identifier is never in it.",
            "properties": {
                "id": {"type": "integer"},
                "at": {"type": "string", "description": "UTC."},
                "by": {"enum": ["you", "staff"]},
                "roles": {"type": "array", "items": {"type": "string"}, "description": "Empty for `you` and for rows older than role recording."},
                "organisations": {"type": "array", "items": {"type": "string"}},
                "did": {"enum": ["was refused access to", "created", "viewed", "changed", "removed", "used"]},
                "what": {"enum": ["consent choices", "care relationships", "identity record", "record"]},
                "refused": {"type": "boolean"},
                "purpose": {"type": ["string", "null"]},
                "break_glass": {"type": "boolean"}
            }
        },
        "AccessLogPage": {
            "type": "object", "required": ["entries", "next"],
            "description": "Newest first, 50 a page. `next` is the `before` for the older page, or null.",
            "properties": {
                "entries": {"type": "array", "items": {"$ref": "#/components/schemas/AccessLogEntry"}},
                "next": {"type": ["integer", "null"]}
            }
        }
    })
}

/// The schemas for the audit routes: a row, a search page, the chain head and the three verify answers (intact, broken,
/// unchecked).
fn audit_schemas() -> Value {
    json!({
        "AuditRow": {
            "type": "object",
            "required": ["id", "at", "action", "outcome", "agent", "patient", "resource", "purpose", "break_glass", "network_address", "detail"],
            "description": "One audit row as an auditor sees it. A search leaves the digests out; an export adds `digest` and `prev_digest` so the chain can be re-verified elsewhere.",
            "properties": {
                "id": {"type": "integer"},
                "at": {"type": "string", "description": "UTC, with microseconds."},
                "action": {"enum": ["C", "R", "U", "D", "E"]},
                "outcome": {"enum": [0, 4, 8, 12]},
                "agent": {"type": "string", "description": "The token sub."},
                "patient": {"type": ["string", "null"], "description": "Patient/<id> when the access named one."},
                "resource": {"type": ["string", "null"]},
                "purpose": {"type": ["string", "null"]},
                "break_glass": {"type": "boolean"},
                "network_address": {"type": ["string", "null"]},
                "detail": {"type": ["object", "null"], "additionalProperties": true, "description": "Structured detail; never demographics or consent content."},
                "digest": {"type": "string"},
                "prev_digest": {"type": "string"}
            }
        },
        "AuditSearch": {
            "type": "object", "required": ["events", "next"],
            "description": "Up to one page, newest first. `next` is the `before` for the following page, or null at the end.",
            "properties": {
                "events": {"type": "array", "items": {"$ref": "#/components/schemas/AuditRow"}},
                "next": {"type": ["integer", "null"]}
            }
        },
        "AuditHead": {
            "type": ["object", "null"],
            "description": "The chain's head: publish it where the database's owner cannot write, and pass it back as a checkpoint.",
            "required": ["id", "digest"],
            "properties": {"id": {"type": "integer"}, "digest": {"type": "string"}}
        },
        "AuditVerified": {
            "type": "object",
            "required": ["intact", "rows", "archive_rows", "keyed", "head", "checkpoint_confirmed", "assurance"],
            "description": "200: the chain checked out. `assurance` says in words what that does and does not prove.",
            "properties": {
                "intact": {"enum": [true]},
                "rows": {"type": "integer"}, "archive_rows": {"type": "integer"},
                "keyed": {"type": "boolean"},
                "head": {"$ref": "#/components/schemas/AuditHead"},
                "checkpoint_confirmed": {"type": "boolean"},
                "assurance": {"type": "string"}
            }
        },
        "AuditBroken": {
            "type": "object", "required": ["intact", "first_broken_id", "reason", "keyed", "assurance"],
            "description": "409: a row was found altered. A monitor that reads only the status still notices.",
            "properties": {
                "intact": {"enum": [false]},
                "first_broken_id": {"type": "integer"},
                "reason": {"type": "string"},
                "keyed": {"type": "boolean"}, "assurance": {"type": "string"}
            }
        },
        "AuditUnchecked": {
            "type": "object",
            "required": ["intact", "tampering_found", "error", "rows", "unchecked", "first_unchecked_id", "causes", "keyed", "head", "assurance"],
            "description": "424: nothing was found wrong, but some rows could not be checked because this service does not hold the key that made their digest. A key problem, not evidence of tampering.",
            "properties": {
                "intact": {"enum": [false]}, "tampering_found": {"enum": [false]},
                "error": {"enum": ["key-not-held"]},
                "rows": {"type": "integer"}, "unchecked": {"type": "integer"},
                "first_unchecked_id": {"type": "integer"},
                "causes": {"type": "array", "items": {"type": "string"}},
                "keyed": {"type": "boolean"},
                "head": {"$ref": "#/components/schemas/AuditHead"},
                "assurance": {"type": "string"}
            }
        }
    })
}

/// The schemas for break-the-glass: the declaration, the review body and the review, the grant, the review list and the
/// count.
fn break_glass_schemas() -> Value {
    json!({
        "OpenBreakGlass": {
            "type": "object", "required": ["reason"],
            "description": "Declare an emergency to read a record without a relationship. The reason is required; a blank one is refused (422). Asking again while a grant is open answers 200 with the same grant and does not extend it.",
            "properties": {"reason": {"type": "string"}}
        },
        "ReviewBreakGlass": {
            "type": "object", "required": ["outcome", "note"],
            "description": "Someone else's verdict on a grant. The note is required. A grant already reviewed is a 409.",
            "properties": {
                "outcome": {"enum": ["appropriate", "inappropriate"]},
                "note": {"type": "string"}
            }
        },
        "BreakGlassReview": {
            "type": ["object", "null"], "description": "Null until someone reviews the grant.",
            "required": ["reviewed_by", "reviewed_at", "outcome", "note"],
            "properties": {
                "reviewed_by": {"type": "string"},
                "reviewed_at": {"type": "string", "description": "UTC."},
                "outcome": {"enum": ["appropriate", "inappropriate"]},
                "note": {"type": "string"}
            }
        },
        "BreakGlassGrant": {
            "type": "object",
            "required": ["id", "patient_id", "subject", "reason", "granted_at", "expires_at", "review", "overdue"],
            "properties": {
                "id": {"type": "string"},
                "patient_id": {"type": "string"},
                "subject": {"type": "string", "description": "The token subject who opened it."},
                "reason": {"type": "string"},
                "granted_at": {"type": "string", "description": "UTC."},
                "expires_at": {"type": "string", "description": "UTC."},
                "review": {"$ref": "#/components/schemas/BreakGlassReview"},
                "overdue": {"type": "boolean", "description": "Unreviewed for longer than the review window; never true once reviewed."}
            }
        },
        "BreakGlassList": {
            "type": "object", "required": ["grants", "limit", "more", "site_time_zone"],
            "description": "Unreviewed grants, oldest first, up to `limit`.",
            "properties": {
                "grants": {"type": "array", "items": {"$ref": "#/components/schemas/BreakGlassGrant"}},
                "limit": {"type": "integer"},
                "more": {"type": "boolean", "description": "True when more grants are waiting than `limit` allowed: the list is cut short and the newest are not shown."},
                "site_time_zone": {"type": "string", "description": "How the site reads times, for the review screen."}
            }
        },
        "BreakGlassCount": {
            "type": "object", "required": ["unreviewed", "overdue"],
            "properties": {"unreviewed": {"type": "integer"}, "overdue": {"type": "integer"}}
        }
    })
}

/// The schemas for merge and unmerge: the request, a link and the list of links.
fn link_schemas() -> Value {
    json!({
        "LinkRequest": {
            "type": "object", "required": ["source", "reason"],
            "description": "Merge, or unmerge, another record with the one in the path (which survives). The reason is the provenance of the change and is required. A refusal is a 422 ErrorList that may also carry `refusal` (a stable code), `refusal_into` and `conflict_codes`.",
            "properties": {
                "source": {"type": "string", "description": "The record merged away, or to be separated again."},
                "reason": {"type": "string"}
            }
        },
        "PatientLink": {
            "type": "object",
            "required": ["id", "version", "author", "reason", "source", "target", "status", "match_score", "match_decision", "recorded_at"],
            "properties": {
                "id": {"type": "string"},
                "version": {"type": "integer", "description": "1 when merged; 2 when unmerged."},
                "author": {"type": "string"},
                "reason": {"type": "string"},
                "source": {"type": "string", "description": "The record merged away."},
                "target": {"type": "string", "description": "The record that survives."},
                "status": {"type": "string", "description": "active or reversed."},
                "match_score": {"type": ["number", "null"], "description": "The matcher's score at merge time, if computed."},
                "match_decision": {"type": ["string", "null"], "description": "auto_link or review, if computed."},
                "recorded_at": {"type": "string", "description": "UTC."}
            }
        },
        "PatientLinkList": {
            "type": "object", "required": ["links"],
            "properties": {"links": {"type": "array", "items": {"$ref": "#/components/schemas/PatientLink"}}}
        }
    })
}

/// Every body schema the service describes, joined from the parts above in order (a name used by two parts would
/// replace the earlier one), plus `ObservationVersion`, a `StoredObservation` with the reason beside it.
fn patient_schemas() -> Value {
    let mut all = Map::new();
    for part in [
        me_schemas(),
        feed_schemas(),
        referral_queue_schemas(),
        audit_schemas(),
        break_glass_schemas(),
        hazard_schemas(),
        link_schemas(),
        amendment_schemas(),
        identity_schemas(),
        allergy_schemas(),
        referral_schemas(),
        observation_schemas(),
        result_schemas(),
        assessment_schemas(),
        consent_schemas(),
        relationship_schemas(),
    ] {
        all.extend(part.as_object().expect("an object").clone());
    }
    // A version is a StoredObservation with the reason it was written beside it.
    let mut version = all["StoredObservation"].clone();
    version["properties"]["reason"] = json!({"type": ["string", "null"]});
    version["required"]
        .as_array_mut()
        .expect("required list")
        .push(json!("reason"));
    version["description"] = json!("A StoredObservation plus the reason this version was written.");
    all.insert("ObservationVersion".into(), version);
    Value::Object(all)
}

/// `(method, path, request schema, success status, success schema, 409 schema)` for the routes
/// whose bodies are described; an empty 409 schema means the route has none. Each also answers 422
/// with `ErrorList`.
const BODIES: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "POST",
        "/api/patients",
        "CreatePatient",
        "201",
        "Stored",
        "DuplicateConflict",
    ),
    (
        "PUT",
        "/api/patients/{id}",
        "UpdatePatient",
        "200",
        "Stored",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/allergies",
        "CreateAllergy",
        "201",
        "StoredAllergy",
        "",
    ),
    (
        "PUT",
        "/api/patients/{id}/allergies/{allergy_id}",
        "UpdateAllergy",
        "200",
        "StoredAllergy",
        "VersionConflict",
    ),
    (
        "PUT",
        "/api/patients/{id}/allergy-statement",
        "CreateAllergyStatement",
        "200",
        "AllergyStatement",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/referrals",
        "CreateReferral",
        "201",
        "StoredReferral",
        "",
    ),
    (
        "POST",
        "/api/patients/{id}/referrals/{referral_id}/events",
        "ReferralEvent",
        "200",
        "StoredReferral",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/observations",
        "CreateObservation",
        "201",
        "StoredObservation",
        "",
    ),
    (
        "PUT",
        "/api/patients/{id}/observations/{observation_id}",
        "UpdateObservation",
        "200",
        "StoredObservation",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/assessments",
        "CreateAssessment",
        "201",
        "StoredAssessment",
        "",
    ),
    (
        "POST",
        "/api/patients/{id}/results",
        "CreateResult",
        "201",
        "StoredResult",
        "",
    ),
    (
        "PUT",
        "/api/patients/{id}/results/{result_id}",
        "UpdateResult",
        "200",
        "StoredResult",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/results/{result_id}/sign-off",
        "SignOffResult",
        "201",
        "ResultSignOff",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/consents",
        "CreateConsent",
        "201",
        "StoredConsent",
        "",
    ),
    (
        "PUT",
        "/api/patients/{id}/consents/{consent_id}",
        "UpdateConsent",
        "200",
        "StoredConsent",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/relationships",
        "CreateRelationship",
        "201",
        "StoredRelationship",
        "",
    ),
    (
        "PUT",
        "/api/patients/{id}/relationships/{relationship_id}",
        "UpdateRelationship",
        "200",
        "StoredRelationship",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/break-glass",
        "OpenBreakGlass",
        "201",
        "BreakGlassGrant",
        "",
    ),
    (
        "POST",
        "/api/break-glass/reviews/{id}",
        "ReviewBreakGlass",
        "200",
        "BreakGlassGrant",
        "Error",
    ),
    (
        "POST",
        "/api/feed/refusals/{id}/resolve",
        "ResolveFeedRefusal",
        "200",
        "FeedRefusal",
        "Error",
    ),
    (
        "POST",
        "/api/referral-messages",
        "ReferralMessage",
        "200",
        "ReferralMessageResult",
        "Error",
    ),
    (
        "POST",
        "/api/referrals/{referral_id}/acknowledge",
        "AcknowledgeReferral",
        "200",
        "ReferralAcknowledged",
        "Error",
    ),
    (
        "POST",
        "/api/referral-queue/{id}/clear",
        "ClearReferralEntry",
        "200",
        "ReferralQueueEntry",
        "Error",
    ),
    (
        "POST",
        "/api/hazards",
        "CreateHazard",
        "201",
        "StoredHazard",
        "",
    ),
    (
        "PUT",
        "/api/hazards/{hazard_id}",
        "UpdateHazard",
        "200",
        "StoredHazard",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/merge",
        "LinkRequest",
        "201",
        "PatientLink",
        "",
    ),
    (
        "POST",
        "/api/patients/{id}/unmerge",
        "LinkRequest",
        "200",
        "PatientLink",
        "",
    ),
    (
        "POST",
        "/api/patients/{id}/adjustments",
        "RecordAdjustment",
        "201",
        "Adjustment",
        "",
    ),
    (
        "PUT",
        "/api/patients/{id}/adjustments/{adjustment_id}",
        "ChangeAdjustment",
        "200",
        "Adjustment",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/death/assertions",
        "ReportDeath",
        "201",
        "DeathAssertion",
        "",
    ),
    (
        "PUT",
        "/api/patients/{id}/death/assertions/{assertion_id}",
        "StepDeathAssertion",
        "200",
        "DeathAssertion",
        "VersionConflict",
    ),
    (
        "POST",
        "/api/patients/{id}/amendments",
        "CreateAmendment",
        "201",
        "AmendmentView",
        "",
    ),
    (
        "PUT",
        "/api/patients/{id}/amendments/{amendment_id}",
        "UpdateAmendment",
        "200",
        "AmendmentView",
        "VersionConflict",
    ),
];

/// `(method, path, success schema, query parameters)` for the described reads.
const READS: &[(&str, &str, &str, &[&str])] = &[
    ("GET", "/api/patients/{id}", "PatientView", &[]),
    ("GET", "/api/patients/{id}/allergies", "AllergyList", &[]),
    ("GET", "/api/patients/{id}/referrals", "ReferralList", &[]),
    (
        "GET",
        "/api/patients/{id}/referrals/{referral_id}",
        "ReferralHistory",
        &[],
    ),
    ("GET", "/api/patients/{id}/consents", "ConsentList", &[]),
    ("GET", "/api/patients/{id}/amendments", "AmendmentList", &[]),
    (
        "GET",
        "/api/patients/{id}/adjustments",
        "AdjustmentList",
        &[],
    ),
    ("GET", "/api/patients/{id}/death", "DeathAnswer", &[]),
    (
        "GET",
        "/api/patients/{id}/death/assertions",
        "DeathAssertionList",
        &[],
    ),
    ("GET", "/api/patients/{id}/links", "PatientLinkList", &[]),
    ("GET", "/api/hazards", "HazardList", &[]),
    ("GET", "/api/me/patients", "OwnRecordList", &[]),
    (
        "GET",
        "/api/me/patients/{id}/access-log",
        "AccessLogPage",
        &["before", "from", "to", "by", "refused", "purpose", "format"],
    ),
    ("GET", "/api/feed/refusals", "FeedRefusalList", &[]),
    ("GET", "/api/feed/refusals/count", "FeedRefusalCount", &[]),
    ("GET", "/api/feed/refusals/{id}", "FeedRefusal", &[]),
    ("GET", "/api/referral-queue", "ReferralQueueList", &[]),
    (
        "GET",
        "/api/referral-queue/count",
        "ReferralQueueCount",
        &[],
    ),
    ("GET", "/api/referral-queue/{id}", "ReferralQueueEntry", &[]),
    ("GET", "/api/death-reports", "DeathReportList", &[]),
    ("GET", "/api/death-reports/count", "ReferralQueueCount", &[]),
    ("GET", "/api/overdue-reviews", "OverdueReviewList", &[]),
    (
        "GET",
        "/api/overdue-reviews/count",
        "ReferralQueueCount",
        &[],
    ),
    (
        "GET",
        "/api/referrals/worklist",
        "ReferralWorklistList",
        &[],
    ),
    (
        "GET",
        "/api/referrals/worklist/count",
        "ReferralQueueCount",
        &[],
    ),
    (
        "GET",
        "/api/audit",
        "AuditSearch",
        &[
            "agent",
            "patient",
            "action",
            "outcome",
            "purpose",
            "break_glass",
            "from",
            "to",
            "after",
            "through",
            "before",
        ],
    ),
    ("GET", "/api/break-glass/reviews", "BreakGlassList", &[]),
    (
        "GET",
        "/api/break-glass/reviews/count",
        "BreakGlassCount",
        &[],
    ),
    (
        "GET",
        "/api/patients/{id}/relationships",
        "RelationshipList",
        &[],
    ),
    (
        "GET",
        "/api/patients/{id}/observations",
        "ObservationList",
        &[],
    ),
    ("GET", "/api/patients/{id}/results", "ResultList", &[]),
    (
        "GET",
        "/api/patients/{id}/assessments",
        "AssessmentList",
        &[],
    ),
    (
        "GET",
        "/api/patients/{id}/observations/{observation_id}/versions",
        "ObservationVersions",
        &[],
    ),
    (
        "GET",
        "/api/patients",
        "PatientSearchResults",
        &["family", "birth_date", "identifier", "drill"],
    ),
];

/// The error schemas, matching `api_error.rs`.
fn error_schemas() -> Value {
    json!({
        "Error": problem_schema(
            "api_error.rs: RFC 9457; `detail` is fixed wording, and extra fields such as current_version or break_glass_offer may follow.",
            &[],
            &json!({}),
        ),
        "ErrorList": problem_schema(
            "422: the record was refused, one line per reason in `errors`.",
            &["errors"],
            &json!({"errors": {"type": "array", "items": {"type": "string"}}}),
        ),
        "OperationOutcome": {
            "type": "object", "required": ["resourceType", "issue"],
            "properties": {
                "resourceType": {"const": "OperationOutcome"},
                "issue": {"type": "array", "items": {"type": "object"}}
            },
            "additionalProperties": true
        }
    })
}

/// The error schemas together with the body schemas: the document's `components.schemas`.
fn all_schemas() -> Value {
    let mut all = error_schemas();
    all.as_object_mut()
        .expect("an object")
        .extend(patient_schemas().as_object().expect("an object").clone());
    all
}

/// The audit export is a file and the verify answer has three shapes by status, so neither fits the
/// tables above. Search gets its 400 here.
fn audit_operation(
    method: &str,
    path: &str,
    op: &mut Map<String, Value>,
    responses: &mut Map<String, Value>,
) {
    match (method, path) {
        ("GET", "/api/audit") => {
            responses.insert(
                "400".into(),
                body_ref("A criterion that is refused: a date or instant that does not parse, or a value of the wrong type (outcome or before not a number, break_glass not true or false). Always the JSON `Error`.", "Error"),
            );
        }
        ("GET", "/api/audit/export") => {
            let mut parameters = Vec::new();
            parameters.push(json!({"name": "format", "in": "query", "required": false,
                "description": "csv (the default) or ndjson.", "schema": {"enum": ["csv", "ndjson"]}}));
            for name in AUDIT_CRITERIA {
                let schema = match *name {
                    "break_glass" => json!({"type": "boolean"}),
                    "outcome" | "before" => json!({"type": "integer"}),
                    _ => json!({"type": "string"}),
                };
                parameters.push(
                    json!({"name": name, "in": "query", "required": false, "schema": schema}),
                );
            }
            op.insert("parameters".into(), Value::Array(parameters));
            let file = json!({"schema": {"type": "string"}});
            responses.insert(
                "200".into(),
                json!({"description": "The rows with their digests, as an attachment. The export is itself audited. Up to 10000 rows; when there are more, `x-next-before` is the `before` for the next file.",
                       "headers": {"x-next-before": {"schema": {"type": "integer"}}},
                       "content": {"text/csv": file.clone(), "application/x-ndjson": file}}),
            );
            responses.insert(
                "400".into(),
                body_ref(
                    "A criterion that does not parse, or a format that is neither.",
                    "Error",
                ),
            );
        }
        ("GET", "/api/audit/verify") => {
            op.insert(
                "parameters".into(),
                json!([
                    {"name": "checkpoint_id", "in": "query", "required": false, "schema": {"type": "integer"},
                     "description": "The head id from an earlier answer; give both checkpoint parts or neither."},
                    {"name": "checkpoint_digest", "in": "query", "required": false, "schema": {"type": "string"}}
                ]),
            );
            responses.insert(
                "200".into(),
                body_ref("The chain is intact.", "AuditVerified"),
            );
            responses.insert(
                "400".into(),
                body_ref("Only one of the two checkpoint parts was given.", "Error"),
            );
            responses.insert(
                "409".into(),
                body_ref("A row was found altered.", "AuditBroken"),
            );
            responses.insert(
                "424".into(),
                body_ref(
                    "Not fully checked: a key is missing. Not evidence of tampering.",
                    "AuditUnchecked",
                ),
            );
        }
        _ => {}
    }
}

/// The photo routes carry image bytes and query parameters, not a JSON body, so the tables above
/// cannot describe them. `GET` answers the image; `PUT` takes one; `POST …/withdraw` takes none.
fn photo_operation(
    method: &str,
    path: &str,
    op: &mut Map<String, Value>,
    responses: &mut Map<String, Value>,
) {
    if path != "/api/patients/{id}/photo" && path != "/api/patients/{id}/photo/withdraw" {
        return;
    }
    if method == "GET" {
        let image = json!({"schema": {"type": "string", "format": "binary"}});
        responses.insert(
            "200".into(),
            json!({"description": "The current photo, never cached (private, no-store).",
                   "content": {"image/png": image.clone(), "image/jpeg": image}}),
        );
        responses.insert(
            "404".into(),
            body_ref(
                "No such patient, or no photo (none stored, or withdrawn).",
                "Error",
            ),
        );
        return;
    }
    let mut parameters = op
        .get("parameters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    parameters.push(json!({"name": "expected_version", "in": "query", "required": true,
        "description": "The photo version last seen; 0 when there is none. Without it the answer is 400.",
        "schema": {"type": "integer"}}));
    parameters.push(
        json!({"name": "reason", "in": "query", "required": false, "schema": {"type": "string"}}),
    );
    op.insert("parameters".into(), Value::Array(parameters));
    if method == "PUT" {
        let image = json!({"schema": {"type": "string", "format": "binary"}});
        op.insert(
            "requestBody".into(),
            json!({"required": true, "content": {"image/png": image.clone(), "image/jpeg": image}}),
        );
    }
    responses.insert(
        "200".into(),
        body_ref("The new photo version.", "PhotoVersion"),
    );
    responses.insert(
        "400".into(),
        body_ref("expected_version was not given.", "Error"),
    );
    responses.insert(
        "409".into(),
        body_ref("A stale expected_version.", "VersionConflict"),
    );
    responses.insert("422".into(), body_ref("The file was refused (not a PNG or JPEG, or too large), or there was no photo to withdraw.", "ErrorList"));
}

/// The document for the given routes. Pure, so a test can plant a route and see it appear.
#[must_use]
pub fn document_for(served: &[(String, String)]) -> Value {
    let mut paths: Map<String, Value> = Map::new();
    for (method, path) in served {
        let public = auth::is_public_path(path);
        let mut op = Map::new();
        op.insert("operationId".into(), json!(operation_id(method, path)));
        op.insert("summary".into(), json!(format!("{method} {path}")));
        op.insert("tags".into(), json!([tag(path)]));
        let params = path_parameters(path);
        if !params.is_empty() {
            op.insert("parameters".into(), Value::Array(params));
        }
        let mut responses = Map::new();
        responses.insert(
            "default".into(),
            json!({"description": "Bodies are not described in this document; see spec/09-api-surface.md."}),
        );
        if public {
            op.insert("security".into(), json!([]));
            op.insert("x-epr-public".into(), json!(true));
        } else {
            op.insert("security".into(), json!([{"bearerAuth": []}]));
            let error = error_ref(path);
            responses.insert(
                "401".into(),
                json!({"description": "No valid token.", "content": {error.0: {"schema": error.1.clone()}}}),
            );
            responses.insert(
                "403".into(),
                json!({"description": "The policy engine refused the request; the refusal is audited.",
                       "content": {error.0: {"schema": error.1}}}),
            );
            let m = axum::http::Method::from_bytes(method.as_bytes());
            if let Some((_, req)) = m.ok().and_then(|m| routes::lookup(&m, path)) {
                op.insert("x-epr-resource".into(), json!(resource_name(req.resource)));
                op.insert("x-epr-action".into(), json!(format!("{:?}", req.action)));
            }
        }
        if let Some((_, _, request, status, success, conflict)) =
            BODIES.iter().find(|(m, p, ..)| m == method && p == path)
        {
            op.insert(
                "requestBody".into(),
                json!({"required": true, "content": {"application/json": {
                    "schema": {"$ref": format!("#/components/schemas/{request}")}}}}),
            );
            if !conflict.is_empty() {
                responses.insert(
                    "409".into(),
                    body_ref("A likely duplicate or a stale expected_version.", conflict),
                );
            }
            responses.insert(
                "422".into(),
                body_ref("The record was refused, one line per reason.", "ErrorList"),
            );
            responses.insert(
                (*status).into(),
                json!({"description": "The stored record.", "content": {"application/json": {
                    "schema": {"$ref": format!("#/components/schemas/{success}")}}}}),
            );
        }
        if let Some((_, _, success, query)) =
            READS.iter().find(|(m, p, ..)| m == method && p == path)
        {
            responses.insert(
                "200".into(),
                json!({"description": "OK.", "content": {"application/json": {
                    "schema": {"$ref": format!("#/components/schemas/{success}")}}}}),
            );
            let mut parameters = op
                .get("parameters")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for name in *query {
                let schema = match *name {
                    "drill" | "break_glass" | "refused" => json!({"type": "boolean"}),
                    "outcome" | "before" | "checkpoint_id" => json!({"type": "integer"}),
                    _ => json!({"type": "string"}),
                };
                parameters.push(
                    json!({"name": name, "in": "query", "required": false, "schema": schema}),
                );
            }
            if !parameters.is_empty() {
                op.insert("parameters".into(), Value::Array(parameters));
            }
        }
        if method == "GET" && path == "/api/me/patients/{id}/access-log" {
            responses.insert(
                "400".into(),
                body_ref("A filter the log does not understand: by is not you or staff, purpose is not a purpose-of-use code, format is not csv, or a date does not parse.", "Error"),
            );
            if let Some(ok) = responses.get_mut("200").and_then(Value::as_object_mut)
                && let Some(content) = ok.get_mut("content").and_then(Value::as_object_mut)
            {
                content.insert("text/csv".into(), json!({"schema": {"type": "string"}}));
                ok.insert("description".into(), json!("The JSON page, or with `format=csv` an attachment of up to 1000 rows (`x-next-before` is the `before` for the next file)."));
                ok.insert(
                    "headers".into(),
                    json!({"x-next-before": {"schema": {"type": "integer"}}}),
                );
            }
        }
        if method == "GET" && path == "/_dependencies" {
            responses.insert(
                "200".into(),
                body_ref("Always 200: it reports and refuses nothing, so a stopped key service never takes the API out of service. One answer is reused for 5 seconds. `checked` is what this deployment configures; a key service is asked by opening a connection, so it shows that it is listening and no more.", "DependencyReport"),
            );
        }
        if method == "GET" && path == "/metrics" {
            responses.insert(
                "200".into(),
                json!({"description": "The process counters in the Prometheus text format. Public, like the health probes; only when `EPR_METRICS` is on.",
                       "content": {"text/plain; version=0.0.4; charset=utf-8": {"schema": {"type": "string"}}}}),
            );
            responses.insert(
                "404".into(),
                json!({"description": "Metrics are off (`EPR_METRICS` is not set). The body is empty."}),
            );
        }
        photo_operation(method, path, &mut op, &mut responses);
        audit_operation(method, path, &mut op, &mut responses);
        // Asking again for a grant that is open answers 200 with the same grant.
        if path == "/api/patients/{id}/break-glass" && method == "POST" {
            responses.insert(
                "200".into(),
                json!({"description": "A grant is already open; it is returned and not extended.",
                       "content": {"application/json": {"schema": {"$ref": "#/components/schemas/BreakGlassGrant"}}}}),
            );
        }
        if method == "POST" && path.starts_with("/api/") {
            let mut parameters = op
                .get("parameters")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            parameters.push(json!({
                "name": "Idempotency-Key", "in": "header", "required": false,
                "description": "1 to 128 letters, digits or . _ : -. A retry with the same key and the same request returns the first answer (header `Idempotent-Replayed: true`) and writes nothing; the same key for a different request is a 422; a key whose first request is unfinished is a 409. Only a success is kept; keys expire after 24 hours.",
                "schema": {"type": "string", "minLength": 1, "maxLength": 128, "pattern": "^[A-Za-z0-9._:-]+$"}
            }));
            op.insert("parameters".into(), Value::Array(parameters));
        }
        op.insert("responses".into(), Value::Object(responses));
        paths
            .entry(path.clone())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .expect("an object was just inserted")
            .insert(method.to_ascii_lowercase(), Value::Object(op));
    }
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Electronic Patient Record service",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "NOT FOR CLINICAL USE. An inventory of the served routes with their authentication and policy requirements, derived from the code. Only error bodies are described; request bodies and success responses are not. The API is versioned by the Accepts-version header; no path carries a version segment.",
            "x-epr-derived-from": ["controllers::all_routes", "routes::TABLE"]
        },
        "paths": paths,
        "components": {
            "securitySchemes": {"bearerAuth": {"type": "http", "scheme": "bearer"}},
            "schemas": all_schemas()
        }
    })
}

/// The document for the routes this build serves.
#[must_use]
pub fn document() -> Value {
    document_for(&served())
}

/// The document as committed: pretty JSON with a final newline.
#[must_use]
pub fn rendered() -> String {
    let mut text = serde_json::to_string_pretty(&document()).expect("a Value always serialises");
    text.push('\n');
    text
}

/// `GET /api/openapi.json`: the document this module builds.
async fn serve() -> Response {
    Json(document()).into_response()
}

/// The route.
#[must_use]
pub fn routes() -> Routes {
    Routes::new().add("/api/openapi.json", get(serve))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_error::ApiError;

    fn required(name: &str) -> Vec<String> {
        error_schemas()[name]["required"]
            .as_array()
            .expect("required list")
            .iter()
            .map(|v| v.as_str().expect("key").to_string())
            .collect()
    }

    fn serialised(e: &ApiError) -> Value {
        serde_json::to_value(e.problem()).unwrap()
    }

    #[test]
    fn real_error_bodies_carry_the_keys_the_schemas_require() {
        let plain = serialised(&ApiError::forbidden().with("break_glass_offer", true));
        for key in required("Error") {
            assert!(plain.get(&key).is_some(), "Error body lacks {key}");
        }
        let list = serialised(&ApiError::invalid(vec!["x".into()]));
        for key in required("ErrorList") {
            assert!(list.get(&key).is_some(), "ErrorList body lacks {key}");
        }
    }

    #[test]
    fn every_error_response_points_at_a_schema_that_exists() {
        let doc = document();
        let schemas = doc["components"]["schemas"].as_object().expect("schemas");
        let mut seen = 0;
        for (_, methods) in doc["paths"].as_object().expect("paths") {
            for (_, op) in methods.as_object().expect("methods") {
                for code in ["401", "403"] {
                    let Some(r) = op["responses"].get(code) else {
                        continue;
                    };
                    let reference = r["content"]
                        .as_object()
                        .and_then(|c| c.values().next())
                        .and_then(|m| m["schema"]["$ref"].as_str())
                        .expect("a $ref");
                    let name = reference.rsplit('/').next().unwrap();
                    assert!(schemas.contains_key(name), "{reference} is not defined");
                    seen += 1;
                }
            }
        }
        assert!(seen > 0);
    }

    fn properties(name: &str) -> std::collections::BTreeSet<String> {
        patient_schemas()[name]["properties"]
            .as_object()
            .expect("properties")
            .keys()
            .cloned()
            .collect()
    }

    fn keys(value: &Value) -> std::collections::BTreeSet<String> {
        value
            .as_object()
            .expect("an object")
            .keys()
            .cloned()
            .collect()
    }

    fn full_patient() -> electronic_patient_record_matcher::Patient {
        use electronic_patient_record_matcher::{
            Address, AdministrativeSex, HumanName, Identifier, Patient, Telecom,
        };
        Patient {
            death_date: None,
            id: Some("i".into()),
            names: vec![HumanName {
                family: Some("f".into()),
                given: vec!["g".into()],
            }],
            birth_date: Some("2000".into()),
            sex: Some(AdministrativeSex::Other),
            identifiers: vec![Identifier {
                system: "s".into(),
                value: "v".into(),
            }],
            addresses: vec![Address {
                postal_code: Some("p".into()),
            }],
            telecom: vec![Telecom {
                system: "s".into(),
                value: "v".into(),
            }],
            temporary_identity: true,
        }
    }

    #[test]
    fn the_patient_schemas_name_exactly_the_fields_the_types_serialise() {
        let patient = serde_json::to_value(full_patient()).unwrap();
        assert_eq!(keys(&patient), properties("Patient"));
        let schema = patient_schemas();
        let inner = |field: &str, of: &str| -> std::collections::BTreeSet<String> {
            schema["Patient"]["properties"][field]["items"][of]
                .as_object()
                .expect("item properties")
                .keys()
                .cloned()
                .collect()
        };
        assert_eq!(keys(&patient["names"][0]), inner("names", "properties"));
        assert_eq!(
            keys(&patient["addresses"][0]),
            inner("addresses", "properties")
        );
        assert_eq!(
            keys(&patient["identifiers"][0]),
            inner("identifiers", "properties")
        );
        assert_eq!(keys(&patient["telecom"][0]), inner("telecom", "properties"));

        let stored = serde_json::to_value(crate::patients_store::Stored {
            id: "x".into(),
            version: 1,
            author: "a".into(),
            patient: full_patient(),
            drill: false,
        })
        .unwrap();
        assert_eq!(keys(&stored), properties("Stored"));
    }

    #[test]
    fn the_request_schemas_name_exactly_the_fields_the_handlers_read() {
        use crate::controllers::patients::{CreateRequest, UpdateRequest};
        let create: CreateRequest = serde_json::from_value(serde_json::json!({
            "patient": {}, "reason": "r", "override_reason": "o", "drill": true
        }))
        .unwrap();
        assert!(create.drill);
        let update: UpdateRequest = serde_json::from_value(serde_json::json!({
            "patient": {}, "expected_version": 1, "reason": "r"
        }))
        .unwrap();
        assert_eq!(update.expected_version, 1);
        // The required keys are the ones the types cannot default.
        assert!(serde_json::from_value::<CreateRequest>(serde_json::json!({})).is_err());
        assert!(
            serde_json::from_value::<UpdateRequest>(serde_json::json!({"patient": {}})).is_err()
        );
        let schema = patient_schemas();
        assert_eq!(
            schema["CreatePatient"]["required"],
            serde_json::json!(["patient"])
        );
        assert_eq!(
            schema["UpdatePatient"]["required"],
            serde_json::json!(["patient", "expected_version"])
        );
        assert_eq!(
            properties("CreatePatient"),
            ["patient", "reason", "override_reason", "drill"]
                .map(String::from)
                .into()
        );
        assert_eq!(
            properties("UpdatePatient"),
            ["patient", "expected_version", "reason"]
                .map(String::from)
                .into()
        );
    }

    #[test]
    fn the_conflict_schemas_match_what_the_service_sends() {
        let version = serialised(&ApiError::from(
            &crate::patients_store::StoreError::Conflict { current: 3 },
        ));
        let schema = patient_schemas();
        for key in schema["VersionConflict"]["required"].as_array().unwrap() {
            assert!(version.get(key.as_str().unwrap()).is_some(), "lacks {key}");
        }
        let duplicate = serialised(
            &ApiError::conflict("possible duplicate")
                .with("candidates", serde_json::json!([]))
                .with("candidates_cut", false),
        );
        for key in schema["DuplicateConflict"]["required"].as_array().unwrap() {
            assert!(
                duplicate.get(key.as_str().unwrap()).is_some(),
                "lacks {key}"
            );
        }
    }

    #[test]
    fn the_read_schemas_name_exactly_the_fields_the_service_sends() {
        use crate::controllers::patients::PatientView;
        use crate::patients_store::{SEARCH_LIMIT, Search, Stored};
        let stored = Stored {
            id: "x".into(),
            version: 1,
            author: "a".into(),
            patient: full_patient(),
            drill: false,
        };
        let view = serde_json::to_value(PatientView {
            stored: stored.clone(),
            photo_version: Some(1),
            replaced_by: Some("y".into()),
            replaces: vec!["z".into()],
            site_time_zone: "UTC".into(),
        })
        .unwrap();
        assert_eq!(keys(&view), properties("PatientView"));
        let results = serde_json::json!({"results": [stored], "limit": SEARCH_LIMIT});
        assert_eq!(keys(&results), properties("PatientSearchResults"));
        // Every documented query parameter is one `Search` reads, and each is accepted.
        let (_, _, _, query) = READS.iter().find(|r| r.1 == "/api/patients").unwrap();
        let all: serde_json::Map<String, Value> = query
            .iter()
            .map(|k| {
                (
                    (*k).to_string(),
                    if *k == "drill" {
                        true.into()
                    } else {
                        "v".into()
                    },
                )
            })
            .collect();
        let parsed: Search = serde_json::from_value(Value::Object(all)).unwrap();
        assert!(parsed.drill && parsed.family.is_some() && parsed.birth_date.is_some());
        assert!(parsed.identifier.is_some());
    }

    #[test]
    fn the_referral_queue_schemas_match_the_types() {
        use crate::referral_unmatched::{Cleared, Entry};
        let schema = patient_schemas();
        let entry = Entry {
            id: "i".into(),
            received_at: "t".into(),
            agent: "a".into(),
            kind: "no-id".into(),
            referral_ref: Some("r".into()),
            event: Some("e".into()),
            reasons: vec!["w".into()],
            note: Some("n".into()),
            message: Some("m".into()),
            cleared: Some(Cleared {
                cleared_by: "c".into(),
                cleared_at: "t".into(),
                how: "matched".into(),
                referral_id: Some("r".into()),
                reason: "w".into(),
            }),
        };
        let value = serde_json::to_value(&entry).unwrap();
        assert_eq!(keys(&value), properties("ReferralQueueEntry"));
        assert_eq!(
            keys(&value["cleared"]),
            schema["ReferralQueueEntry"]["properties"]["cleared"]["properties"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect()
        );
    }

    #[test]
    fn the_worklist_schema_matches_the_type_and_names_every_waiting_state() {
        use crate::referral::State;
        use crate::referral_worklist::{ATTENTION, AWAITING_OUTCOME, Item, Why};
        let schema = patient_schemas();
        let item = Item {
            referral_id: "r".into(),
            patient_id: "p".into(),
            version: 2,
            state: State::Declined,
            why: Why::Reply,
            recorded_at: "t".into(),
        };
        assert_eq!(
            keys(&serde_json::to_value(&item).unwrap()),
            properties("ReferralWorklistItem")
        );
        let listed: Vec<String> = schema["ReferralWorklistItem"]["properties"]["state"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        // The states a worklist item can be in: those that wait for the sender, those timed for an answer
        // (`requested`) and those timed for an outcome.
        let mut real: Vec<String> = ATTENTION.iter().map(|s| s.as_str().to_string()).collect();
        real.push("requested".into());
        real.extend(AWAITING_OUTCOME.iter().map(|s| s.as_str().to_string()));
        let (mut listed, mut real) = (listed, real);
        listed.sort();
        listed.dedup();
        real.sort();
        real.dedup();
        assert_eq!(listed, real);
    }

    #[test]
    fn the_referral_schemas_match_the_types() {
        use crate::referral::{Event, State};
        use crate::referrals_store::{ExternalId, Priority, Referral};
        let schema = patient_schemas();
        let record = Referral {
            recipient: "r".into(),
            reason: "r".into(),
            question: "q".into(),
            priority: Priority::Stat,
            external_ids: vec![ExternalId {
                system: "s".into(),
                value: "v".into(),
            }],
        };
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(keys(&value), properties("ReferralRecord"));
        let words = |schema: &Value| -> Vec<String> {
            schema["enum"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w.as_str().unwrap().to_string())
                .collect()
        };
        assert_eq!(
            words(&schema["StoredReferral"]["properties"]["state"]),
            State::ALL.map(|s| s.as_str().to_string())
        );
        assert_eq!(
            words(&schema["ReferralEvent"]["properties"]["event"]),
            Event::ALL.map(|e| e.as_str().to_string())
        );
        assert_eq!(
            words(&schema["ReferralRecord"]["properties"]["priority"]),
            [
                Priority::Routine,
                Priority::Urgent,
                Priority::Asap,
                Priority::Stat
            ]
            .map(|p| serde_json::to_value(p)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string())
        );
    }

    #[test]
    fn the_allergy_schemas_match_the_types() {
        use crate::allergies::{
            AllergyRecord, Category, Coding, Criticality, Status, StoredAllergy,
        };
        let schema = patient_schemas();
        let record = AllergyRecord {
            substance: "s".into(),
            coding: Some(Coding {
                system: "u".into(),
                code: "c".into(),
            }),
            category: Some(Category::Food),
            criticality: Some(Criticality::UnableToAssess),
            reaction: Some("r".into()),
            onset: Some("2000".into()),
            status: Status::EnteredInError,
        };
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(keys(&value), properties("AllergyRecord"));
        assert_eq!(
            keys(&value["coding"]),
            schema["AllergyRecord"]["properties"]["coding"]["properties"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect()
        );
        // The enum words are the ones serde writes, variant by variant.
        let words = |name: &str| -> Vec<Value> {
            schema["AllergyRecord"]["properties"][name]["enum"]
                .as_array()
                .unwrap()
                .clone()
        };
        let said = |v: &[Value]| v.to_vec();
        assert_eq!(
            words("category"),
            said(
                &[
                    Category::Food,
                    Category::Medication,
                    Category::Environment,
                    Category::Biologic
                ]
                .map(|c| serde_json::to_value(c).unwrap())
            )
        );
        assert_eq!(
            words("criticality"),
            said(
                &[
                    Criticality::Low,
                    Criticality::High,
                    Criticality::UnableToAssess
                ]
                .map(|c| serde_json::to_value(c).unwrap())
            )
        );
        assert_eq!(
            words("status"),
            said(
                &[Status::Active, Status::Inactive, Status::EnteredInError]
                    .map(|c| serde_json::to_value(c).unwrap())
            )
        );
        // A bare record omits what was not stated, and still has the required keys.
        let bare = serde_json::to_value(AllergyRecord {
            substance: "s".into(),
            coding: None,
            category: None,
            criticality: None,
            reaction: None,
            onset: None,
            status: Status::Active,
        })
        .unwrap();
        for key in schema["AllergyRecord"]["required"].as_array().unwrap() {
            assert!(bare.get(key.as_str().unwrap()).is_some());
        }
        let stored = serde_json::to_value(StoredAllergy {
            id: "i".into(),
            patient_id: "p".into(),
            version: 1,
            author: "a".into(),
            recorded_at: "t".into(),
            allergy: record,
        })
        .unwrap();
        assert_eq!(keys(&stored), properties("StoredAllergy"));
        let create: crate::controllers::allergies::CreateRequest =
            serde_json::from_value(serde_json::json!({"allergy": bare, "reason": "r"})).unwrap();
        assert!(create.reason.is_some());
        assert!(
            serde_json::from_value::<crate::controllers::allergies::UpdateRequest>(
                serde_json::json!({"allergy": bare})
            )
            .is_err()
        );
    }

    #[test]
    fn the_photo_routes_are_described_as_images_with_a_required_version() {
        let doc = document();
        let put = &doc["paths"]["/api/patients/{id}/photo"]["put"];
        assert!(put["requestBody"]["content"]["image/png"].is_object());
        assert!(put["requestBody"]["content"]["image/jpeg"].is_object());
        let version = put["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "expected_version")
            .expect("expected_version is a parameter");
        assert_eq!(version["required"], true);
        assert!(doc["paths"]["/api/patients/{id}/photo"]["get"]["responses"]["200"]["content"]["image/png"].is_object());
        let withdraw = &doc["paths"]["/api/patients/{id}/photo/withdraw"]["post"];
        assert!(withdraw.get("requestBody").is_none());
        assert_eq!(
            withdraw["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/PhotoVersion"
        );
    }

    #[test]
    fn the_allergy_statement_schemas_match_the_types() {
        use crate::allergies::Assertion;
        let value = serde_json::to_value(Assertion {
            version: 1,
            author: "a".into(),
            recorded_at: "t".into(),
            no_known_allergies: true,
        })
        .unwrap();
        assert_eq!(keys(&value), properties("AllergyStatement"));
        let schema = patient_schemas();
        let request = &schema["CreateAllergyStatement"];
        let mut required: Vec<_> = request["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        required.sort();
        assert_eq!(required, ["expected_version", "none_known"]);
        let parsed: crate::controllers::allergies::StatementRequest =
            serde_json::from_value(serde_json::json!({"none_known": true, "expected_version": 0}))
                .unwrap();
        assert!(parsed.none_known && parsed.reason.is_none());
        assert!(
            serde_json::from_value::<crate::controllers::allergies::StatementRequest>(
                serde_json::json!({"none_known": true})
            )
            .is_err()
        );
    }

    #[test]
    fn the_allergy_list_schema_names_the_fields_the_handler_sends() {
        use crate::allergies::Summary;
        use crate::controllers::allergies::AllergyList;
        let list = serde_json::to_value(AllergyList {
            allergies: vec![],
            statement: None,
            summary: Summary::Active { items: vec![] },
            amendment_notices: std::collections::BTreeMap::new(),
        })
        .unwrap();
        assert_eq!(keys(&list), properties("AllergyList"));
        assert_eq!(list["summary"]["state"], "active");
        assert!(list["statement"].is_null());
    }

    #[test]
    fn the_observation_schemas_match_the_types() {
        use crate::controllers::observations::{ObservationList, VersionList};
        use crate::observations::{AbsentReason, Flag, Range};
        use crate::observations_store::{
            ObservationRecord, ObservationVersion, Status, StoredObservation,
        };
        let schema = patient_schemas();
        let record = ObservationRecord {
            code: "8867-4".into(),
            value: Some(1.0),
            unit: Some("/min".into()),
            absent_reason: Some(AbsentReason::NotDone),
            range: Some(Range {
                low: Some(1.0),
                high: Some(2.0),
            }),
            critical: Some(Range {
                low: Some(0.0),
                high: Some(3.0),
            }),
            effective_at: "2026-01-01T00:00:00Z".into(),
            status: Status::EnteredInError,
        };
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(keys(&value), properties("ObservationRecord"));
        let range_props: std::collections::BTreeSet<String> =
            schema["ObservationRecord"]["properties"]["range"]["properties"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect();
        assert_eq!(keys(&value["range"]), range_props);
        let words = |schema_name: &str, field: &str| -> Vec<Value> {
            schema[schema_name]["properties"][field]["enum"]
                .as_array()
                .unwrap()
                .clone()
        };
        let said = |v: Vec<Value>| v;
        assert_eq!(
            words("ObservationRecord", "absent_reason"),
            said(
                [
                    AbsentReason::NotDone,
                    AbsentReason::Declined,
                    AbsentReason::Failed,
                    AbsentReason::Unknown
                ]
                .map(|r| serde_json::to_value(r).unwrap())
                .to_vec()
            )
        );
        assert_eq!(
            words("ObservationRecord", "status"),
            said(
                [Status::Final, Status::EnteredInError]
                    .map(|r| serde_json::to_value(r).unwrap())
                    .to_vec()
            )
        );
        let mut flags: Vec<Value> = [
            Flag::CriticalLow,
            Flag::Low,
            Flag::Normal,
            Flag::High,
            Flag::CriticalHigh,
        ]
        .map(|f| serde_json::to_value(f).unwrap())
        .to_vec();
        flags.push(Value::Null);
        assert_eq!(words("StoredObservation", "flag"), flags);

        let stored = StoredObservation {
            id: "i".into(),
            patient_id: "p".into(),
            version: 1,
            author: "a".into(),
            recorded_at: "t".into(),
            site_zone: "UTC".into(),
            record,
            flag: Some(Flag::Normal),
        };
        let flat = serde_json::to_value(&stored).unwrap();
        assert_eq!(keys(&flat), properties("StoredObservation"));
        let list = serde_json::to_value(ObservationList {
            observations: vec![stored.clone()],
            amendment_notices: std::collections::BTreeMap::new(),
        })
        .unwrap();
        assert_eq!(keys(&list), properties("ObservationList"));
        let versions = serde_json::to_value(VersionList {
            versions: vec![ObservationVersion {
                reason: None,
                stored,
            }],
        })
        .unwrap();
        assert_eq!(keys(&versions), properties("ObservationVersions"));
        let version = &versions["versions"][0];
        assert!(version.get("reason").is_some());
        // The version item is a StoredObservation plus `reason`.
        assert_eq!(keys(version), properties("ObservationVersion"));
    }

    fn nested(name: &str, field: &str) -> std::collections::BTreeSet<String> {
        patient_schemas()[name]["properties"][field]["properties"]
            .as_object()
            .expect("nested properties")
            .keys()
            .cloned()
            .collect()
    }

    fn enum_words(schema: &Value) -> Vec<Value> {
        schema["enum"].as_array().expect("an enum").clone()
    }

    fn said<T: serde::Serialize>(all: &[T]) -> Vec<Value> {
        all.iter()
            .map(|v| serde_json::to_value(v).unwrap())
            .collect()
    }

    #[test]
    fn the_consent_schemas_match_the_types() {
        use crate::consent::{ConsentRecord, Effect, Kind, Period, Provision, Status};
        use crate::consents_store::StoredConsent;
        let schema = patient_schemas();
        let period = Period {
            start: Some("2026-01-01".into()),
            end: None,
        };
        let record = ConsentRecord {
            kind: Kind::General,
            status: Status::Active,
            provisions: vec![Provision {
                effect: Effect::Deny,
                actors: vec![],
                purposes: vec![],
                resources: vec![],
                labels: vec![],
                period: Some(period.clone()),
            }],
            period: Some(period),
        };
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(keys(&value), properties("ConsentRecord"));
        let provision = &schema["ConsentRecord"]["properties"]["provisions"]["items"];
        let provision_keys: std::collections::BTreeSet<String> = provision["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(keys(&value["provisions"][0]), provision_keys);
        let period_keys: std::collections::BTreeSet<String> = schema["ConsentRecord"]["properties"]
            ["period"]["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(keys(&value["period"]), period_keys);
        let props = &schema["ConsentRecord"]["properties"];
        assert_eq!(
            enum_words(&props["kind"]),
            said(&[Kind::General, Kind::NationalDataOptOut])
        );
        assert_eq!(
            enum_words(&props["status"]),
            said(&[Status::Active, Status::Inactive, Status::EnteredInError])
        );
        assert_eq!(
            enum_words(&provision["properties"]["effect"]),
            said(&[Effect::Permit, Effect::Deny])
        );
        let stored = serde_json::to_value(StoredConsent {
            id: "i".into(),
            patient_id: "p".into(),
            version: 1,
            author: "a".into(),
            consent: record,
        })
        .unwrap();
        assert_eq!(keys(&stored), properties("StoredConsent"));
        let list = serde_json::json!({"consents": [stored]});
        assert_eq!(keys(&list), properties("ConsentList"));
        use crate::controllers::consents::{CreateRequest, UpdateRequest};
        let body = serde_json::json!({"status": "active"});
        assert!(
            serde_json::from_value::<CreateRequest>(
                serde_json::json!({"consent": body, "reason": "r"})
            )
            .is_ok()
        );
        assert!(
            serde_json::from_value::<UpdateRequest>(serde_json::json!({"consent": body})).is_err()
        );
    }

    #[test]
    fn the_relationship_schemas_match_the_types() {
        use crate::consent::Period;
        use crate::proxy::{Basis, Scope, Terms};
        use crate::relationships::{Kind, RelationshipRecord, Status, StoredRelationship};
        let schema = patient_schemas();
        let record = RelationshipRecord {
            subject: "s".into(),
            kind: Kind::Proxy,
            status: Status::Active,
            period: Some(Period {
                start: None,
                end: None,
            }),
            proxy: Some(Terms {
                basis: Basis::PatientRequest,
                scopes: [Scope::Identity].into(),
                expires_at: 1,
            }),
        };
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(keys(&value), properties("RelationshipRecord"));
        assert_eq!(keys(&value["proxy"]), nested("RelationshipRecord", "proxy"));
        let props = &schema["RelationshipRecord"]["properties"];
        assert_eq!(
            enum_words(&props["kind"]),
            said(&[
                Kind::CareTeam,
                Kind::ActiveEncounter,
                Kind::SelfPatient,
                Kind::Proxy
            ])
        );
        assert_eq!(
            enum_words(&props["status"]),
            said(&[Status::Active, Status::Ended])
        );
        assert_eq!(
            enum_words(&props["proxy"]["properties"]["basis"]),
            said(&[
                Basis::ParentalResponsibility,
                Basis::PowerOfAttorney,
                Basis::Deputyship,
                Basis::PatientRequest
            ])
        );
        assert_eq!(
            enum_words(&props["proxy"]["properties"]["scopes"]["items"]),
            said(&[
                Scope::Identity,
                Scope::Allergies,
                Scope::Photo,
                Scope::AccessLog,
                Scope::Adjustments
            ])
        );
        let stored = serde_json::to_value(StoredRelationship {
            id: "i".into(),
            patient_id: "p".into(),
            version: 1,
            author: "a".into(),
            relationship: record,
        })
        .unwrap();
        assert_eq!(keys(&stored), properties("StoredRelationship"));
        assert_eq!(
            keys(&serde_json::json!({"relationships": [stored]})),
            properties("RelationshipList")
        );
    }

    #[test]
    fn every_request_schema_is_used_by_a_request_body_and_every_named_schema_exists() {
        let doc = document();
        let schemas = doc["components"]["schemas"].as_object().expect("schemas");
        let text = doc["paths"].to_string();
        for name in schemas.keys() {
            if name.starts_with("Create") || name.starts_with("Update") {
                assert!(
                    text.contains(&format!("#/components/schemas/{name}\"")),
                    "{name} is defined but no operation uses it"
                );
            }
        }
        for (_, _, request, _, success, conflict) in BODIES {
            for name in [request, success, conflict] {
                assert!(
                    name.is_empty() || schemas.contains_key(*name),
                    "{name} is not defined"
                );
            }
        }
        for (_, _, success, _) in READS {
            assert!(schemas.contains_key(*success), "{success} is not defined");
        }
    }

    #[test]
    fn every_post_under_api_takes_the_idempotency_key_the_middleware_reads() {
        let doc = document();
        let mut posts = 0;
        for (path, item) in doc["paths"].as_object().unwrap() {
            let Some(op) = item.get("post") else { continue };
            if !path.starts_with("/api/") {
                continue;
            }
            posts += 1;
            let param = op["parameters"]
                .as_array()
                .and_then(|p| p.iter().find(|p| p["in"] == "header"))
                .unwrap_or_else(|| panic!("{path} has no header parameter"));
            assert_eq!(
                param["name"].as_str().unwrap().to_ascii_lowercase(),
                crate::idempotency::HEADER
            );
            assert_eq!(param["required"], false);
            // The documented pattern is the one the middleware enforces.
            let pattern = param["schema"]["pattern"].as_str().unwrap();
            assert_eq!(pattern, "^[A-Za-z0-9._:-]+$");
            assert_eq!(param["schema"]["maxLength"], 128);
        }
        assert!(posts >= 10, "only {posts} POST routes under /api");
    }

    #[test]
    fn the_metrics_route_is_described_with_the_content_type_it_sends() {
        let doc = document();
        let ok = &doc["paths"]["/metrics"]["get"]["responses"]["200"]["content"];
        let sent = crate::metrics::response(true);
        let sent = sent.headers()["content-type"].to_str().unwrap().to_string();
        assert!(ok.get(&sent).is_some(), "the document lacks {sent}");
        assert_eq!(
            crate::metrics::response(false).status(),
            axum::http::StatusCode::NOT_FOUND
        );
        assert!(
            doc["paths"]["/metrics"]["get"]["responses"]["404"]
                .get("content")
                .is_none()
        );
    }

    #[test]
    fn the_me_schemas_match_the_types() {
        use crate::audit::PatientLogRow;
        let row = |agent: &str, outcome: i16| PatientLogRow {
            id: 1,
            at: "t".into(),
            action: "R".into(),
            outcome,
            agent: agent.into(),
            resource: Some("Patient".into()),
            purpose: None,
            break_glass: false,
            roles: vec!["clinician".into()],
            organisations: vec![],
        };
        let schema = patient_schemas();
        let words = |v: &Value| -> Vec<String> {
            v["enum"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w.as_str().unwrap().to_string())
                .collect()
        };
        for (r, caller) in [
            (row("me", 0), "me"),
            (row("them", 0), "me"),
            (row("them", 4), "me"),
        ] {
            let e = crate::controllers::me::entry(&r, caller);
            assert_eq!(keys(&e), properties("AccessLogEntry"));
            for (field, name) in [("by", "by"), ("did", "did"), ("what", "what")] {
                let w = e[field].as_str().unwrap().to_string();
                assert!(
                    words(&schema["AccessLogEntry"]["properties"][name]).contains(&w),
                    "{field} {w} is not in the schema"
                );
            }
        }
        for kind in [
            crate::relationships::Kind::SelfPatient,
            crate::relationships::Kind::Proxy,
        ] {
            assert!(
                words(&schema["OwnRecord"]["properties"]["relationship"])
                    .contains(&kind.as_str().to_string())
            );
        }
    }

    #[test]
    fn the_feed_schemas_match_the_types() {
        use crate::feed_queue::{Entry, Kind, Outcome, Resolution};
        let entry = |message: Option<String>, resolution: Option<Resolution>| Entry {
            id: "i".into(),
            received_at: "t".into(),
            agent: "feed".into(),
            peer: None,
            event: "A08".into(),
            control_id: "1".into(),
            kind: "unsupported".into(),
            reasons: vec!["r".into()],
            message,
            resolution,
        };
        let resolution = Resolution {
            resolved_by: "a".into(),
            resolved_at: "t".into(),
            outcome: "handled".into(),
            reason: "r".into(),
        };
        let listed = serde_json::to_value(entry(None, None)).unwrap();
        assert!(
            listed["resolution"].is_null(),
            "open means a null resolution, not a missing key"
        );
        let mut without = properties("FeedRefusal");
        without.remove("message");
        assert_eq!(keys(&listed), without);
        let read = serde_json::to_value(entry(Some("m".into()), Some(resolution))).unwrap();
        assert_eq!(keys(&read), properties("FeedRefusal"));
        assert_eq!(keys(&read["resolution"]), properties("FeedResolution"));
        let schema = patient_schemas();
        let words = |v: &Value| v["enum"].as_array().unwrap().clone();
        assert_eq!(
            words(&schema["ResolveFeedRefusal"]["properties"]["outcome"]),
            [Outcome::Handled, Outcome::Dismissed]
                .map(|o| serde_json::to_value(o).unwrap())
                .to_vec()
        );
        assert_eq!(
            words(&schema["FeedRefusal"]["properties"]["kind"]),
            [Kind::Unsupported, Kind::Conflict]
                .map(|k| serde_json::to_value(k).unwrap())
                .to_vec()
        );
        assert!(
            serde_json::from_value::<crate::controllers::feed::ResolveRequest>(
                serde_json::json!({"outcome": "dismissed", "reason": "r"})
            )
            .is_ok()
        );
    }

    #[test]
    fn the_audit_schemas_match_the_types() {
        use crate::audit::{Action, AuditRow};
        let row = |digest: Option<String>| AuditRow {
            id: 1,
            at: "t".into(),
            action: "R".into(),
            outcome: 0,
            agent: "a".into(),
            patient: None,
            resource: None,
            purpose: None,
            break_glass: false,
            network_address: None,
            detail: None,
            prev_digest: digest.clone(),
            digest,
        };
        // A search leaves the digests out; absent evidence is still a null key, not a missing one.
        let search = serde_json::to_value(row(None)).unwrap();
        let export = serde_json::to_value(row(Some("d".into()))).unwrap();
        let all = properties("AuditRow");
        let mut without: std::collections::BTreeSet<String> = all.clone();
        without.remove("digest");
        without.remove("prev_digest");
        assert_eq!(keys(&search), without);
        assert_eq!(keys(&export), all);
        let schema = patient_schemas();
        let _ = &schema;
        let doc = all_schemas();
        let words: Vec<String> = doc["AuditRow"]["properties"]["action"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w.as_str().unwrap().to_string())
            .collect();
        let said: Vec<String> = [
            Action::Create,
            Action::Read,
            Action::Update,
            Action::Delete,
            Action::Execute,
        ]
        .iter()
        .map(|a| a.code().to_string())
        .collect();
        assert_eq!(words, said);
        // Every criterion the search takes is a documented query parameter.
        let (_, _, _, query) = READS.iter().find(|r| r.1 == "/api/audit").unwrap();
        assert_eq!(*query, AUDIT_CRITERIA);
    }

    #[test]
    fn the_break_glass_schemas_match_the_types() {
        use crate::break_glass::{Grant, Outcome, Review};
        let review = Review {
            reviewed_by: "a".into(),
            reviewed_at: "t".into(),
            outcome: "appropriate".into(),
            note: "n".into(),
        };
        let grant = |review| Grant {
            id: "i".into(),
            patient_id: "p".into(),
            subject: "s".into(),
            reason: "r".into(),
            granted_at: "t".into(),
            expires_at: "t".into(),
            expires_unix: 0,
            review,
            overdue: false,
        };
        let open = serde_json::to_value(grant(None)).unwrap();
        assert_eq!(keys(&open), properties("BreakGlassGrant"));
        assert!(
            open["review"].is_null(),
            "an unreviewed grant has a null review"
        );
        let reviewed = serde_json::to_value(grant(Some(review))).unwrap();
        assert_eq!(keys(&reviewed["review"]), properties("BreakGlassReview"));
        let schema = patient_schemas();
        assert_eq!(
            schema["ReviewBreakGlass"]["properties"]["outcome"]["enum"]
                .as_array()
                .unwrap()
                .clone(),
            [Outcome::Appropriate, Outcome::Inappropriate]
                .map(|o| serde_json::to_value(o).unwrap())
                .to_vec()
        );
        assert!(
            serde_json::from_value::<crate::controllers::break_glass::ReviewRequest>(
                serde_json::json!({"outcome": "appropriate", "note": "n"})
            )
            .is_ok()
        );
        assert!(
            serde_json::from_value::<crate::controllers::break_glass::OpenRequest>(
                serde_json::json!({})
            )
            .is_err()
        );
    }

    #[test]
    fn the_hazard_schemas_match_the_types() {
        use crate::hazard::{Control, Evidence, EvidenceKind, Hazard, State};
        let evidence = Evidence {
            kind: EvidenceKind::ReviewRecord,
            reference: "r".into(),
            taken_at: chrono::Utc::now(),
            sha256: "0".repeat(64),
        };
        let hazard = Hazard {
            title: "t".into(),
            severity: 1,
            likelihood: 1,
            state: State::Retired,
            raised_by: "a".into(),
            controls: vec![Control {
                by: "a".into(),
                text: "t".into(),
            }],
            evidence: vec![evidence.clone()],
            retired_by: None,
            note: None,
            reopened: 0,
        };
        let value = serde_json::to_value(&hazard).unwrap();
        assert_eq!(keys(&value), properties("Hazard"));
        assert_eq!(keys(&value["controls"][0]), properties("HazardControl"));
        assert_eq!(keys(&value["evidence"][0]), properties("HazardEvidence"));
        let stored = serde_json::to_value(crate::hazard_store::StoredHazard {
            id: "i".into(),
            version: 1,
            author: "a".into(),
            recorded_at: "t".into(),
            hazard,
        })
        .unwrap();
        assert_eq!(keys(&stored), properties("StoredHazard"));
        // The words are the ones serde writes, variant by variant.
        let schema = patient_schemas();
        let words = |v: &Value| v["enum"].as_array().unwrap().clone();
        assert_eq!(
            words(&schema["HazardEvidence"]["properties"]["kind"]),
            [
                EvidenceKind::RemovalChange,
                EvidenceKind::TestRun,
                EvidenceKind::ReviewRecord,
                EvidenceKind::Census
            ]
            .map(|k| serde_json::to_value(k).unwrap())
            .to_vec()
        );
        assert_eq!(
            words(&schema["Hazard"]["properties"]["state"]),
            [State::Open, State::Retired]
                .map(|k| serde_json::to_value(k).unwrap())
                .to_vec()
        );
        // Each event parses with exactly the fields the schema names for it.
        use crate::controllers::hazards::UpdateRequest;
        for body in [
            serde_json::json!({"expected_version": 1, "event": "control", "text": "t"}),
            serde_json::json!({"expected_version": 1, "event": "reopen", "reason": "r"}),
            serde_json::json!({"expected_version": 1, "event": "retire", "note": "n",
                "evidence": [serde_json::to_value(&evidence).unwrap()]}),
        ] {
            assert!(
                serde_json::from_value::<UpdateRequest>(body.clone()).is_ok(),
                "{body}"
            );
            for key in keys(&body) {
                assert!(properties("UpdateHazard").contains(&key), "{key}");
            }
        }
        assert_eq!(
            words(&schema["UpdateHazard"]["properties"]["event"]),
            ["control", "retire", "reopen"]
                .map(|w| serde_json::json!(w))
                .to_vec()
        );
    }

    #[test]
    fn the_link_schemas_match_the_types() {
        let value = serde_json::to_value(crate::links::StoredLink {
            id: "i".into(),
            version: 1,
            author: "a".into(),
            reason: "r".into(),
            source: "s".into(),
            target: "t".into(),
            status: "active".into(),
            match_score: None,
            match_decision: None,
            recorded_at: "t".into(),
        })
        .unwrap();
        // Absent evidence is null, not a missing key, so a null score still has its key.
        assert_eq!(keys(&value), properties("PatientLink"));
        let schema = patient_schemas();
        let mut required: Vec<_> = schema["PatientLink"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        required.sort();
        assert_eq!(
            required
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>(),
            properties("PatientLink")
        );
        assert!(
            serde_json::from_value::<crate::controllers::links::LinkRequest>(
                serde_json::json!({"source": "x", "reason": "y"})
            )
            .is_ok()
        );
        for missing in [
            serde_json::json!({"source": "x"}),
            serde_json::json!({"reason": "y"}),
        ] {
            assert!(
                serde_json::from_value::<crate::controllers::links::LinkRequest>(missing).is_err()
            );
        }
    }

    #[test]
    fn the_amendment_schemas_match_the_types() {
        use crate::amendment::{Ground, Notice, State};
        use crate::controllers::amendments::{CreateRequest, UpdateRequest};
        let schema = patient_schemas();
        let props = &schema["AmendmentView"]["properties"];
        assert_eq!(
            enum_words(&props["request"]["properties"]["state"]["properties"]["state"]),
            ["requested", "accepted", "applied", "denied", "withdrawn"]
                .map(Value::from)
                .to_vec()
        );
        // The tags are the ones serde writes.
        let tag = |state: State| serde_json::to_value(state).unwrap()["state"].clone();
        assert_eq!(tag(State::Requested), "requested");
        assert_eq!(tag(State::Applied { version: 2 }), "applied");
        assert_eq!(tag(State::Withdrawn), "withdrawn");
        assert_eq!(
            tag(State::Accepted {
                by: "b".into(),
                reason: "r".into()
            }),
            "accepted"
        );
        assert_eq!(
            tag(State::Denied {
                by: "b".into(),
                ground: Ground::Other,
                reason: "r".into(),
                disagreement: None
            }),
            "denied"
        );
        let notice = |n: Notice| serde_json::to_value(n).unwrap()["notice"].clone();
        assert_eq!(notice(Notice::Pending), "pending");
        assert_eq!(notice(Notice::Amended { version: 2 }), "amended");
        assert_eq!(
            notice(Notice::Refused {
                ground: Ground::Other,
                disagreement: None
            }),
            "refused"
        );
        assert_eq!(
            enum_words(&props["notice"]["properties"]["notice"]),
            ["pending", "amended", "refused"].map(Value::from).to_vec()
        );
        let grounds = [
            Ground::NotOurs,
            Ground::AccurateAndComplete,
            Ground::NotAvailableToRequester,
            Ground::Other,
        ];
        assert_eq!(
            enum_words(&schema["UpdateAmendment"]["properties"]["ground"]),
            said(&grounds)
        );
        // Every step word is accepted by the real request type, and the create keys are required.
        for step in ["accept", "deny", "apply", "extend", "withdraw", "disagree"] {
            let body = serde_json::json!({"expected_version": 1, "step": step, "reason": "r",
                "ground": "other", "version": 2, "statement": "s"});
            assert!(
                serde_json::from_value::<UpdateRequest>(body).is_ok(),
                "{step}"
            );
        }
        assert!(enum_words(&schema["UpdateAmendment"]["properties"]["step"]).len() == 6);
        for key in schema["CreateAmendment"]["required"].as_array().unwrap() {
            let mut body = serde_json::json!({"kind": "Observation", "target_id": "t", "version": 1, "text": "x"});
            body.as_object_mut().unwrap().remove(key.as_str().unwrap());
            assert!(
                serde_json::from_value::<CreateRequest>(body).is_err(),
                "{key} is required"
            );
        }
    }
}
