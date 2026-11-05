//! Label census (PI-7): every route that gives something out states what it does with a security label.
//! A new output route fails here until it is listed. The answer today is plain: **no route applies a
//! label.** A record cannot carry one, `authorize` always asks with none, and nothing acts on the policy
//! engine's `Mask` obligation. So a route with patient data is listed as [`Label::NotApplied`], which is a
//! gap and not a safe state, and the two tests at the end fail on the day that changes, so that each route
//! has to be looked at again.
//!
//! When labels are real, the protected information spec asks more than this: a test on every route that a
//! labelled record does not appear. A table cannot show that.

use electronic_patient_record::routes;

/// What a route does with a security label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Label {
    /// It gives out patient data and applies no label (the gap).
    NotApplied,
    /// It gives out no patient data, so there is nothing to label.
    NoPatientData(&'static str),
}

use Label::{NoPatientData, NotApplied};

/// Every route that gives something out, as spelled in `routes::TABLE`.
const OUTPUT: &[(&str, &str, Label)] = &[
    ("POST", "/api/patients/compare", NotApplied),
    ("GET", "/api/patients", NotApplied),
    ("GET", "/api/patients/{id}", NotApplied),
    ("GET", "/api/patients/{id}/photo", NotApplied),
    ("GET", "/api/patients/{id}/links", NotApplied),
    ("GET", "/api/patients/{id}/consents", NotApplied),
    ("GET", "/api/patients/{id}/relationships", NotApplied),
    ("GET", "/api/patients/{id}/allergies", NotApplied),
    ("GET", "/api/patients/{id}/observations", NotApplied),
    (
        "GET",
        "/api/patients/{id}/observations/{observation_id}/versions",
        NotApplied,
    ),
    ("GET", "/api/patients/{id}/amendments", NotApplied),
    ("GET", "/api/patients/{id}/referrals", NotApplied),
    (
        "GET",
        "/api/patients/{id}/referrals/{referral_id}",
        NotApplied,
    ),
    ("GET", "/api/patients/{id}/adjustments", NotApplied),
    ("GET", "/api/patients/{id}/death", NotApplied),
    ("GET", "/api/patients/{id}/death/assertions", NotApplied),
    ("GET", "/api/me/patients", NotApplied),
    ("GET", "/api/me/patients/{id}/access-log", NotApplied),
    ("GET", "/api/audit", NotApplied),
    ("GET", "/api/audit/export", NotApplied),
    ("GET", "/api/referrals/worklist", NotApplied),
    ("GET", "/api/referral-queue", NotApplied),
    ("GET", "/api/referral-queue/{id}", NotApplied),
    ("GET", "/api/feed/refusals", NotApplied),
    ("GET", "/api/feed/refusals/{id}", NotApplied),
    ("GET", "/api/break-glass/reviews", NotApplied),
    ("GET", "/fhir/R4/Patient", NotApplied),
    ("GET", "/fhir/R4/Patient/{id}", NotApplied),
    ("GET", "/fhir/R4/AllergyIntolerance", NotApplied),
    ("GET", "/fhir/R4/Observation", NotApplied),
    ("GET", "/fhir/R4/AuditEvent", NotApplied),
    ("GET", "/fhir/R5/Patient", NotApplied),
    ("GET", "/fhir/R5/Patient/{id}", NotApplied),
    ("GET", "/fhir/R5/AllergyIntolerance", NotApplied),
    ("GET", "/fhir/R5/Observation", NotApplied),
    (
        "GET",
        "/api/openapi.json",
        NoPatientData("the API's description"),
    ),
    (
        "GET",
        "/fhir/R4/metadata",
        NoPatientData("the capability statement"),
    ),
    (
        "GET",
        "/fhir/R5/metadata",
        NoPatientData("the capability statement"),
    ),
    (
        "GET",
        "/api/audit/verify",
        NoPatientData("whether the chain verifies"),
    ),
    (
        "GET",
        "/api/hazards",
        NoPatientData("the hazard register belongs to no patient"),
    ),
    (
        "GET",
        "/api/referrals/worklist/count",
        NoPatientData("a number"),
    ),
    (
        "GET",
        "/api/referral-queue/count",
        NoPatientData("a number"),
    ),
    ("GET", "/api/feed/refusals/count", NoPatientData("a number")),
    (
        "GET",
        "/api/break-glass/reviews/count",
        NoPatientData("a number"),
    ),
];

/// POST routes that answer with something read from the record. Every other POST or PUT answers with what
/// was written, which is listed by its read.
const POSTS_THAT_READ: &[&str] = &["/api/patients/compare"];

fn source(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[test]
fn every_output_route_states_its_label_behaviour() {
    let mut served: Vec<(String, String)> = routes::TABLE
        .iter()
        .filter(|(method, path, ..)| *method == "GET" || POSTS_THAT_READ.contains(path))
        .map(|(method, path, ..)| ((*method).to_string(), (*path).to_string()))
        .collect();
    served.sort();
    let mut listed: Vec<(String, String)> = OUTPUT
        .iter()
        .map(|(method, path, _)| ((*method).to_string(), (*path).to_string()))
        .collect();
    listed.sort();
    assert_eq!(
        served, listed,
        "an output route was added or removed: state what it does with a label, here"
    );
}

/// When a real label reaches the decision, this fails: look at every `NotApplied` above.
#[test]
fn the_decision_is_still_asked_with_no_labels() {
    let common = source("src/controllers/common.rs");
    // The consent lookup and the record decision.
    assert_eq!(
        common
            .matches("labels: std::collections::BTreeSet::new(),")
            .count(),
        2,
        "labels reach the decision now: revisit every route in this census"
    );
}

/// When a route acts on a mask, this fails: that route is no longer `NotApplied`.
#[test]
fn no_route_acts_on_a_mask_yet() {
    let mut files = Vec::new();
    let mut dirs = vec![std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs")
                && !path.ends_with("src/policy.rs")
            {
                files.push(path);
            }
        }
    }
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        // A unit test may look at a decision; only the code that serves a route counts.
        let code = text.split("#[cfg(test)]").next().unwrap();
        assert!(
            !code.contains("Obligation::Mask"),
            "{}: a route masks now: update this census",
            path.display()
        );
    }
}
