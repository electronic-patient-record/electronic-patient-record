//! The official HL7 FHIR validator (`validator_cli.jar`, hapifhir/org.hl7.fhir.core) as a test oracle: every
//! resource kind the service writes is checked against the base R4 (4.0.1) rules. Ignored by default because
//! the jar is about 200 MB and needs Java:
//!
//! ```text
//! FHIR_VALIDATOR_JAR=$(sh ../scripts/fhir-validator.sh) cargo test --test fhir_validator -- --ignored
//! ```
//!
//! Run with `-tx n/a`: no terminology server is contacted, so structure, cardinality, types, required
//! bindings the jar carries, and invariants are checked, but a code is not looked up. UK Core and US Core
//! profile validation is a separate task (RC-2, tasks.md), as is the search-parameter oracle.

use electronic_patient_record::allergies::{
    AllergyRecord, Category, Coding, Criticality, Status, StoredAllergy,
};
use electronic_patient_record::audit::AuditRow;
use electronic_patient_record::interop::{fhir_allergy, fhir_audit, fhir_patient};
use electronic_patient_record::patients_store::Stored;
use electronic_patient_record_matcher::{
    Address, AdministrativeSex, HumanName, Identifier, NHS_NUMBER_SYSTEM, Patient, Telecom,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

const PATIENT: &str = "11111111-1111-4111-8111-111111111111";

fn patient() -> Stored {
    Stored {
        drill: false,
        id: PATIENT.into(),
        version: 3,
        author: "someone".into(),
        patient: Patient {
            id: Some(PATIENT.into()),
            names: vec![HumanName {
                family: Some("Smith".into()),
                given: vec!["Ann".into(), "Marie".into()],
            }],
            birth_date: Some("1980-03".into()),
            sex: Some(AdministrativeSex::Female),
            identifiers: vec![Identifier {
                system: NHS_NUMBER_SYSTEM.into(),
                value: "9434765919".into(),
            }],
            addresses: vec![Address {
                postal_code: Some("AB1 2CD".into()),
            }],
            telecom: vec![Telecom {
                system: "phone".into(),
                value: "01632 960001".into(),
            }],
        },
    }
}

fn allergy(status: Status) -> StoredAllergy {
    StoredAllergy {
        id: "22222222-2222-4222-8222-222222222222".into(),
        patient_id: PATIENT.into(),
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

fn audit_row() -> AuditRow {
    AuditRow {
        id: 42,
        at: "2026-10-21T09:00:12.123456Z".into(),
        action: "R".into(),
        outcome: 0,
        agent: "607b683d-fd04-4a73-ba47-f4d05df3d046".into(),
        patient: Some(format!("Patient/{PATIENT}")),
        resource: Some("Patient".into()),
        purpose: Some("treatment".into()),
        break_glass: false,
        network_address: None,
        detail: None,
        digest: None,
        prev_digest: None,
    }
}

fn value<T: serde::Serialize>(r: &T) -> Value {
    serde_json::to_value(r).unwrap()
}

/// What the service writes, by name.
fn resources() -> Vec<(&'static str, Value)> {
    let p = value(&fhir_patient::to_fhir(&patient()).unwrap());
    let active = value(
        &fhir_allergy::to_fhir(&allergy(Status::Active), PATIENT)
            .unwrap()
            .unwrap(),
    );
    let inactive = value(
        &fhir_allergy::to_fhir(&allergy(Status::Inactive), PATIENT)
            .unwrap()
            .unwrap(),
    );
    let mut drill_record = patient();
    drill_record.drill = true;
    let drill = value(&fhir_patient::to_fhir(&drill_record).unwrap());
    let none = value(&fhir_allergy::no_known_allergies(PATIENT, "2026-10-21T09:30:00Z").unwrap());
    let audit = value(&fhir_audit::to_fhir(&audit_row()).unwrap());
    let bundle = json!({
        "resourceType": "Bundle",
        "type": "searchset",
        "total": 1,
        "entry": [{ "fullUrl": format!("urn:uuid:{PATIENT}"), "resource": p.clone(), "search": { "mode": "match" } }],
    });
    vec![
        ("patient", p),
        ("patient-drill", drill),
        ("allergy-active", active),
        ("allergy-inactive", inactive),
        ("no-known-allergies", none),
        ("audit-event", audit),
        ("patient-searchset", bundle),
    ]
}

fn jar() -> PathBuf {
    let jar = std::env::var("FHIR_VALIDATOR_JAR")
        .expect("set FHIR_VALIDATOR_JAR (sh scripts/fhir-validator.sh prints the path)");
    assert!(Path::new(&jar).is_file(), "{jar} is not a file");
    PathBuf::from(jar)
}

fn validate(jar: &Path, files: &[PathBuf]) -> (bool, String) {
    let out = Command::new("java")
        .arg("-jar")
        .arg(jar)
        .args(["-version", "4.0.1", "-tx", "n/a"])
        .args(files)
        .output()
        .expect("java must be on the PATH");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

#[test]
#[ignore = "needs Java and the validator jar: set FHIR_VALIDATOR_JAR"]
fn fhir_validator_accepts_every_resource_the_service_writes() {
    let jar = jar();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fhir-validator");
    std::fs::create_dir_all(&dir).unwrap();
    let mut files = Vec::new();
    for (name, v) in resources() {
        let path = dir.join(format!("{name}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(&v).unwrap()).unwrap();
        files.push(path);
    }
    let (ok, text) = validate(&jar, &files);
    assert!(ok, "the validator reported errors:\n{text}");
    assert!(
        !text.contains("Error @"),
        "the validator reported errors:\n{text}"
    );
}

/// A control: the oracle must be able to fail. A Patient with an invalid `gender` code and a stray
/// element is refused, so a green run above means something.
#[test]
#[ignore = "needs Java and the validator jar: set FHIR_VALIDATOR_JAR"]
fn fhir_validator_refuses_a_broken_patient() {
    let jar = jar();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fhir-validator");
    std::fs::create_dir_all(&dir).unwrap();
    let mut v = value(&fhir_patient::to_fhir(&patient()).unwrap());
    v["gender"] = json!("not-a-code");
    v["noSuchElement"] = json!(true);
    let path = dir.join("broken-patient.json");
    std::fs::write(&path, v.to_string()).unwrap();
    let (ok, text) = validate(&jar, &[path]);
    assert!(!ok, "the broken patient passed:\n{text}");
}
