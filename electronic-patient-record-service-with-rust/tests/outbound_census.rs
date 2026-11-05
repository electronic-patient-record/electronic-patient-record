//! Outbound census (BV-3): every way this service reaches out is listed here, and each has a test that a
//! standing report of death stops it. There is one: the outbox runner, tested by
//! `tests/outbox_store.rs::a_report_of_death_holds_an_event_until_it_is_withdrawn`. A new way out fails this
//! file until it is given a hold and a test and added here. It reads the source, so it finds a call, not
//! an intent: a route that sends through a crate not on `CLIENTS` would pass.
//!
//! An answer to a caller is not outbound: the acknowledgement of an HL7 message and a FHIR read answer
//! someone who asked, and the FHIR read says what the caller may know about a death.

use std::path::{Path, PathBuf};

/// The files that may call a delivery, with the test that a hold stops it.
const WAYS_OUT: &[&str] = &["src/outbox.rs"];

/// Crates that open a connection to send something. None is a dependency today.
const CLIENTS: &[&str] = &[
    "reqwest",
    "lettre",
    "ureq",
    "isahc",
    "surf",
    "awc",
    "attohttpc",
    "twilio",
    "aws-sdk-ses",
    "aws-sdk-sns",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn every_delivery_goes_through_a_listed_way_out() {
    let mut files = Vec::new();
    rust_files(&root().join("src"), &mut files);
    let mut found: Vec<String> = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        if text.contains(".deliver(") {
            let name = path.strip_prefix(root()).unwrap().display().to_string();
            found.push(name);
        }
    }
    found.sort();
    assert_eq!(
        found, WAYS_OUT,
        "a new way out: give it a death hold and a test, then list it"
    );
}

#[test]
fn the_delivery_runner_checks_for_a_death_before_it_delivers() {
    let text = std::fs::read_to_string(root().join("src/outbox.rs")).unwrap();
    let check = text.find("holds_contact()").expect("the hold is gone");
    let deliver = text.find(".deliver(").expect("no delivery");
    assert!(check < deliver, "the hold must come before the delivery");
}

#[test]
fn no_crate_that_sends_is_a_dependency() {
    let manifest = std::fs::read_to_string(root().join("Cargo.toml")).unwrap();
    for client in CLIENTS {
        let declared = manifest.lines().any(|l| {
            l.trim_start().starts_with(&format!("{client} "))
                || l.trim_start().starts_with(&format!("{client}="))
        });
        assert!(!declared, "{client}: a new way out needs a death hold");
    }
}
