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

use crate::{auth, controllers, policy::Resource, routes};
use axum::{Json, response::Response};
use loco_rs::prelude::*;
use serde_json::{Map, Value, json};

/// The served `(method, path template)` pairs, normalised to `{name}` segments.
#[must_use]
pub fn served() -> Vec<(String, String)> {
    controllers::all_routes()
        .collect()
        .into_iter()
        .flat_map(|route| {
            let uri = routes::normalise(&route.uri);
            route
                .actions
                .into_iter()
                .map(move |m| (m.to_string(), uri.clone()))
        })
        .collect()
}

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

fn tag(path: &str) -> String {
    let mut parts = path.split('/').filter(|s| !s.is_empty());
    match (parts.next(), parts.next()) {
        (Some("api"), Some(second)) => second.to_string(),
        (Some("fhir"), Some(release)) => format!("fhir-{}", release.to_ascii_lowercase()),
        (Some(first), _) => first.trim_start_matches('_').to_string(),
        _ => "root".to_string(),
    }
}

fn path_parameters(path: &str) -> Vec<Value> {
    path.split('/')
        .filter_map(|s| s.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
        .map(|name| {
            json!({"name": name, "in": "path", "required": true, "schema": {"type": "string"}})
        })
        .collect()
}

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
        (
            "application/json",
            json!({"$ref": "#/components/schemas/Error"}),
        )
    }
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
                "telecom": {"type": "array", "items": pair}
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
        "DuplicateConflict": {
            "type": "object", "required": ["error", "candidates", "candidates_cut"],
            "description": "409 on create: likely duplicates. A candidate the caller may not see is {\"restricted\": true}.",
            "properties": {
                "error": {"type": "string"},
                "candidates": {"type": "array", "items": {"type": "object"}},
                "candidates_cut": {"type": "boolean"}
            }
        },
        "PhotoVersion": {
            "type": "object", "required": ["version", "withdrawn"],
            "description": "What a photo write answers: the new version, and whether it withdrew the photo.",
            "properties": {"version": {"type": "integer"}, "withdrawn": {"type": "boolean"}}
        },
        "VersionConflict": {
            "type": "object", "required": ["error", "current_version"],
            "description": "409 on update: expected_version is not the current one.",
            "properties": {"error": {"type": "string"}, "current_version": {"type": "integer"}}
        }
    })
}

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
                    "scopes": {"type": "array", "items": {"enum": ["identity", "allergies", "photo", "access-log"]}},
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
        }
    })
}

fn patient_schemas() -> Value {
    let mut all = Map::new();
    for part in [
        amendment_schemas(),
        identity_schemas(),
        allergy_schemas(),
        observation_schemas(),
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
    ("GET", "/api/patients/{id}/consents", "ConsentList", &[]),
    ("GET", "/api/patients/{id}/amendments", "AmendmentList", &[]),
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
        "Error": {
            "type": "object", "required": ["error"],
            "properties": {"error": {"type": "string"}},
            "additionalProperties": true,
            "description": "api_error.rs: fixed wording; extra fields such as current_version or break_glass_offer may follow."
        },
        "ErrorList": {
            "type": "object", "required": ["errors"],
            "properties": {"errors": {"type": "array", "items": {"type": "string"}}},
            "additionalProperties": true,
            "description": "422: the record was refused, one line per reason."
        },
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

fn all_schemas() -> Value {
    let mut all = error_schemas();
    all.as_object_mut()
        .expect("an object")
        .extend(patient_schemas().as_object().expect("an object").clone());
    all
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
    let error = |d: &str, name: &str| {
        json!({"description": d, "content": {"application/json": {
            "schema": {"$ref": format!("#/components/schemas/{name}")}}}})
    };
    if method == "GET" {
        let image = json!({"schema": {"type": "string", "format": "binary"}});
        responses.insert(
            "200".into(),
            json!({"description": "The current photo, never cached (private, no-store).",
                   "content": {"image/png": image.clone(), "image/jpeg": image}}),
        );
        responses.insert(
            "404".into(),
            error(
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
        error("The new photo version.", "PhotoVersion"),
    );
    responses.insert(
        "400".into(),
        error("expected_version was not given.", "Error"),
    );
    responses.insert(
        "409".into(),
        error("A stale expected_version.", "VersionConflict"),
    );
    responses.insert("422".into(), error("The file was refused (not a PNG or JPEG, or too large), or there was no photo to withdraw.", "ErrorList"));
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
                    json!({"description": "A likely duplicate or a stale expected_version.",
                           "content": {"application/json": {"schema": {"$ref": format!("#/components/schemas/{conflict}")}}}}),
                );
            }
            responses.insert(
                "422".into(),
                json!({"description": "The record was refused, one line per reason.",
                       "content": {"application/json": {"schema": {"$ref": "#/components/schemas/ErrorList"}}}}),
            );
            responses.insert(
                (*status).into(),
                json!({"description": "The stored patient.", "content": {"application/json": {
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
                let schema = if *name == "drill" {
                    json!({"type": "boolean"})
                } else {
                    json!({"type": "string"})
                };
                parameters.push(
                    json!({"name": name, "in": "query", "required": false, "schema": schema}),
                );
            }
            if !parameters.is_empty() {
                op.insert("parameters".into(), Value::Array(parameters));
            }
        }
        photo_operation(method, path, &mut op, &mut responses);
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

    #[test]
    fn real_error_bodies_carry_the_keys_the_schemas_require() {
        let plain = ApiError::forbidden().with("break_glass_offer", true).body();
        for key in required("Error") {
            assert!(plain.get(&key).is_some(), "Error body lacks {key}");
        }
        let list = ApiError::invalid(vec!["x".into()]).body();
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
        let version =
            ApiError::from(&crate::patients_store::StoreError::Conflict { current: 3 }).body();
        let schema = patient_schemas();
        for key in schema["VersionConflict"]["required"].as_array().unwrap() {
            assert!(version.get(key.as_str().unwrap()).is_some(), "lacks {key}");
        }
        let duplicate = ApiError::conflict("possible duplicate")
            .with("candidates", serde_json::json!([]))
            .with("candidates_cut", false)
            .body();
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
                Scope::AccessLog
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
