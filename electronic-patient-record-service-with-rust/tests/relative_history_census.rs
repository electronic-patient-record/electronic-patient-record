//! EH-3: an entry about a relative reaches no other person, and is not a family view under another name (SG-2,
//! `tests/grief_census.rs`, whose refusal of the word in a route stands with no exception). Database-free.

use electronic_patient_record::policy::Resource;
use electronic_patient_record::relative_history::RelativeHistoryRecord;
use electronic_patient_record::{openapi, routes};
use std::path::Path;

/// The word SG-2 refuses, in a name a client or an operator meets: a path, a policy resource, a schema.
#[test]
fn no_path_resource_or_schema_is_named_for_a_family() {
    let mut names: Vec<String> = routes::TABLE.iter().map(|r| r.1.to_string()).collect();
    names.extend(Resource::ALL.iter().map(|r| r.name().to_string()));
    let document = openapi::document();
    let schemas = document["components"]["schemas"].as_object().unwrap();
    names.extend(schemas.keys().cloned());
    assert!(
        names.len() > 200,
        "the census read too little: {}",
        names.len()
    );
    for name in names {
        assert!(!name.to_lowercase().contains("family"), "{name}");
    }
}

#[test]
fn an_entry_has_nowhere_to_put_another_person() {
    let full = serde_json::json!({
        "genetic": "genetic-parent", "line": "maternal", "gestational": "yes",
        "environment": "childhood", "sex": "female", "member": 1, "condition": "c",
        "coding": { "system": "urn:x", "code": "1" }, "presence": "present", "onset_age": 1,
        "source": "patient", "status": "recorded"
    });
    let record: RelativeHistoryRecord = serde_json::from_value(full.clone()).unwrap();
    // `full` is every field there is: a new one makes this fail until someone has looked at it here.
    assert_eq!(serde_json::to_value(&record).unwrap(), full);
    for field in [
        "patient_id",
        "relative_id",
        "person",
        "name",
        "identifier",
        "birth_date",
        "note",
    ] {
        let mut linked = full.clone();
        linked[field] = serde_json::json!("x");
        assert!(
            serde_json::from_value::<RelativeHistoryRecord>(linked).is_err(),
            "{field}"
        );
    }
}

#[test]
fn nothing_lists_entries_across_patients() {
    let mut seen = 0;
    for (_, path, resource, _) in routes::TABLE {
        if *resource == Resource::RelativeHistory {
            seen += 1;
            assert!(
                path.starts_with("/api/patients/{id}/relative-history"),
                "{path}: only under one patient"
            );
        }
    }
    assert_eq!(seen, 5);
    // The table's only patient is the one whose record it is.
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("migrations/20261124000043_relative_history.sql");
    let migration = std::fs::read_to_string(path).unwrap();
    let table = migration
        .split("CREATE TABLE relative_history_versions")
        .nth(1)
        .and_then(|t| t.split(");").next())
        .expect("the table");
    let uuids: Vec<&str> = table
        .lines()
        .filter(|l| l.contains(" UUID "))
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    assert_eq!(uuids, ["entry_id", "patient_id"]);
}
