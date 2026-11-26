//! Label census (PI-7): every route that gives something out states what it does with a security label.
//! A new output route fails here until it is listed. **Three lists apply labels** (problems, relative history and typing,
//! EH-8), and only their entries can carry one; `http_entry_labels.rs` shows on each of their routes that a labelled entry
//! does not appear. Every other route with patient data is [`Label::NotApplied`], which is a gap: before anything else
//! can carry a label, its routes have to be looked at again. The tests at the end fail when masking reaches another file.

use electronic_patient_record::routes;

/// What a route does with a security label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Label {
    /// It gives out patient data and applies no label (the gap). Nothing it gives out carries one.
    NotApplied,
    /// An entry whose labels the reader may not see is withheld (EH-8).
    Withheld,
    /// It gives out no patient data, so there is nothing to label.
    NoPatientData(&'static str),
}

use Label::{NoPatientData, NotApplied, Withheld};

/// Every route that gives something out, as spelled in `routes::TABLE`.
const OUTPUT: &[(&str, &str, Label)] = &[
    ("POST", "/api/patients/compare", NotApplied),
    ("GET", "/api/patients", NotApplied),
    ("GET", "/api/patients/{id}", NotApplied),
    ("GET", "/api/patients/{id}/photo", NotApplied),
    ("GET", "/api/patients/{id}/links", NotApplied),
    ("GET", "/api/patients/{id}/consents", NotApplied),
    ("GET", "/api/patients/{id}/treatment-refusals", NotApplied),
    ("GET", "/api/patients/{id}/relationships", NotApplied),
    ("GET", "/api/patients/{id}/allergies", NotApplied),
    ("GET", "/api/patients/{id}/allergies/as-at", NotApplied),
    ("GET", "/api/patients/{id}/problems", Withheld),
    ("GET", "/api/patients/{id}/problems/as-at", Withheld),
    (
        "GET",
        "/api/patients/{id}/problems/{problem_id}/versions",
        Withheld,
    ),
    (
        "GET",
        "/api/patients/{id}/problems/{problem_id}/labels",
        Withheld,
    ),
    ("GET", "/api/patients/{id}/relative-history", Withheld),
    ("GET", "/api/patients/{id}/relative-history/as-at", Withheld),
    (
        "GET",
        "/api/patients/{id}/relative-history/{entry_id}/versions",
        Withheld,
    ),
    (
        "GET",
        "/api/patients/{id}/relative-history/{entry_id}/labels",
        Withheld,
    ),
    ("GET", "/api/patients/{id}/compatibility-typing", Withheld),
    (
        "GET",
        "/api/patients/{id}/compatibility-typing/as-at",
        Withheld,
    ),
    (
        "GET",
        "/api/patients/{id}/compatibility-typing/{typing_id}/versions",
        Withheld,
    ),
    (
        "GET",
        "/api/patients/{id}/compatibility-typing/{typing_id}/labels",
        Withheld,
    ),
    ("GET", "/api/patients/{id}/observations", NotApplied),
    ("GET", "/api/patients/{id}/results", NotApplied),
    ("GET", "/api/patients/{id}/assessments", NotApplied),
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
    (
        "GET",
        "/api/patients/{id}/referrals/{referral_id}/suspicion",
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
    ("GET", "/api/tasks", NotApplied),
    (
        "GET",
        "/api/tokens",
        NoPatientData("token metadata belongs to no patient"),
    ),
    ("GET", "/api/watch-list", NotApplied),
    ("GET", "/api/patients/{id}/watch-list", NotApplied),
    ("GET", "/api/death-reports", NotApplied),
    ("GET", "/api/death-reports/count", NoPatientData("a number")),
    ("GET", "/api/overdue-reviews", NotApplied),
    (
        "GET",
        "/api/overdue-reviews/count",
        NoPatientData("a number"),
    ),
    ("GET", "/api/referral-queue", NotApplied),
    ("GET", "/api/referral-queue/{id}", NotApplied),
    ("GET", "/api/feed/refusals", NotApplied),
    ("GET", "/api/feed/refusals/{id}", NotApplied),
    ("GET", "/api/break-glass/reviews", NotApplied),
    ("GET", "/fhir/R4/Patient", NotApplied),
    ("GET", "/fhir/R4/Patient/{id}", NotApplied),
    ("GET", "/fhir/R4/AllergyIntolerance", NotApplied),
    ("GET", "/fhir/R4/Observation", NotApplied),
    ("GET", "/fhir/R4/Flag", NotApplied),
    ("GET", "/fhir/R5/Flag", NotApplied),
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
        "/api/feed/health",
        NoPatientData("states, counts and times about the feed"),
    ),
    (
        "POST",
        "/api/feed/dry-run",
        NoPatientData("outcome words and the acknowledgement of the caller's own message"),
    ),
    (
        "GET",
        "/api/break-glass/reviews/count",
        NoPatientData("a number"),
    ),
];

/// POST routes that answer with something read from the record. Every other POST or PUT answers with what
/// was written, which is listed by its read.
// The dry run (IH-11) reads the record: "updated" or "unchanged" says a patient with those identifiers exists.
const POSTS_THAT_READ: &[&str] = &["/api/patients/compare", "/api/feed/dry-run"];

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

/// The decision for a whole list is asked with no labels; the one for each entry of the three lists with its own. A second
/// place that asks with labels, or a list read with nothing withheld, has to be looked at here.
#[test]
fn only_the_three_lists_ask_the_decision_with_labels() {
    let common = source("src/controllers/common.rs");
    assert_eq!(
        common
            .matches("labels: std::collections::BTreeSet::new(),")
            .count(),
        2,
        "labels reach the decision for a whole list now: revisit every route in this census"
    );
    assert_eq!(common.matches("fn decide(").count(), 1);
    let handlers = source("src/controllers/entries.rs");
    assert!(
        !handlers.contains("Viewer::"),
        "a handler makes its own viewer"
    );
    assert_eq!(
        handlers.matches("caller.decider.viewer()").count(),
        handlers.matches("Caller::gate::<R>(").count(),
        "each handler reads for its caller"
    );
    assert_eq!(
        handlers.matches("caller.decider.note(&viewer)").count(),
        handlers.matches("Caller::gate::<R>(").count(),
        "each handler flags what break-the-glass showed"
    );
}

/// Where labels and masking are, for the three lists. Another file here means other routes to look at.
const MASKING: [&str; 7] = [
    // Re-exports the modules.
    "electronic-patient-record-service-with-rust/src/lib.rs",
    "electronic-patient-record-domain/src/masking.rs",
    "electronic-patient-record-domain/src/labelling.rs",
    "electronic-patient-record-service-with-rust/src/entry_labels.rs",
    "electronic-patient-record-service-with-rust/src/entries_store.rs",
    "src/controllers/entries.rs",
    "src/controllers/common.rs",
];

/// When another route acts on a mask, this fails: that route is no longer `NotApplied`.
#[test]
fn nothing_else_masks_or_labels() {
    let mut files = Vec::new();
    let service = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dirs = vec![
        service.join("src"),
        service.join("electronic-patient-record-domain/src"),
    ];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs")
                && !path.ends_with("electronic-patient-record-domain/src/policy.rs")
                && !MASKING.iter().any(|allowed| path.ends_with(allowed))
            {
                files.push(path);
            }
        }
    }
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        // A unit test may look at a decision; only the code that serves a route counts.
        let code = text.split("#[cfg(test)]").next().unwrap();
        // Nor adds or lifts a label (OQ-PI7): one that hides nothing would be trusted.
        assert!(
            !code.contains("labelling::{")
                && !code.contains("labelling::add")
                && !code.contains("labelling::lift"),
            "{}: labels are added here too: update this census, and mask with them",
            path.display()
        );
        assert!(
            !code.contains("Obligation::Mask"),
            "{}: a route masks now: update this census",
            path.display()
        );
        assert!(
            !code.contains("masking::") && !code.contains("domain::masking"),
            "{}: masking is called here too: update this census",
            path.display()
        );
    }
}
