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
//! profile validation is a separate task (RC-2, tasks.md), as is the search-parameter oracle. An `AuditEvent`
//! that claims a BALP profile is also checked against that profile (PX-4).

use electronic_patient_record::allergies::{
    AllergyRecord, Category, Coding, Criticality, Status, StoredAllergy,
};
use electronic_patient_record::audit::AuditRow;
use electronic_patient_record::interop::{
    fhir_allergy, fhir_audit, fhir_flag, fhir_observation, fhir_patient,
};
use electronic_patient_record::observations::{AbsentReason, Flag, Range};
use electronic_patient_record::observations_store::{
    ObservationRecord, Status as ObsStatus, StoredObservation,
};
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
            death_date: None,
            temporary_identity: false,
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

fn observation(value: Option<f64>) -> StoredObservation {
    StoredObservation {
        id: "33333333-3333-4333-8333-333333333333".into(),
        patient_id: PATIENT.into(),
        version: 2,
        author: "someone".into(),
        recorded_at: "2026-10-26T09:31:00.123456Z".into(),
        site_zone: "Europe/London".into(),
        record: ObservationRecord {
            code: "8867-4".into(),
            value,
            unit: value.map(|_| "/min".into()),
            absent_reason: value.is_none().then_some(AbsentReason::Declined),
            range: value.map(|_| Range {
                low: Some(60.0),
                high: Some(100.0),
            }),
            critical: None,
            effective_at: "2026-10-26T09:30:00Z".into(),
            status: ObsStatus::Final,
        },
        flag: value.map(|_| Flag::High),
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
/// A need, active or ended.
fn flag(status: electronic_patient_record::adjustments::Status) -> Value {
    let stored = electronic_patient_record::adjustments_store::StoredAdjustment {
        id: "44444444-4444-4444-8444-444444444444".into(),
        patient_id: PATIENT.into(),
        version: 2,
        author: "someone".into(),
        recorded_at: "2026-11-05T09:31:00.123456Z".into(),
        adjustment: electronic_patient_record::adjustments::Adjustment {
            code: "interpreter-spoken".into(),
            note: Some("Polish".into()),
            status,
            review_by: None,
            reason: "they asked".into(),
        },
        category: "interpreting".into(),
        display: "Spoken language interpreter".into(),
        overdue: false,
    };
    fhir_flag::to_fhir(&stored, PATIENT).unwrap()
}

fn resources() -> Vec<(&'static str, Value)> {
    let p = value(&fhir_patient::to_fhir(&patient()).unwrap());
    let active = value(
        &fhir_allergy::to_fhir(&allergy(Status::Active), PATIENT, &[])
            .unwrap()
            .unwrap(),
    );
    let inactive = value(
        &fhir_allergy::to_fhir(&allergy(Status::Inactive), PATIENT, &[])
            .unwrap()
            .unwrap(),
    );
    let mut drill_record = patient();
    drill_record.drill = true;
    let drill = value(&fhir_patient::to_fhir(&drill_record).unwrap());
    let none = value(&fhir_allergy::no_known_allergies(PATIENT, "2026-10-21T09:30:00Z").unwrap());
    let audit = value(&fhir_audit::to_fhir(&audit_row()).unwrap());
    let vital = value(
        &fhir_observation::to_fhir(&observation(Some(130.0)), PATIENT, &[])
            .unwrap()
            .unwrap(),
    );
    let declined = value(
        &fhir_observation::to_fhir(&observation(None), PATIENT, &[])
            .unwrap()
            .unwrap(),
    );
    // A record under an amendment request carries the notice as a note.
    let disputed = [
        electronic_patient_record::amendment::Notice::Pending,
        electronic_patient_record::amendment::Notice::Refused {
            ground: electronic_patient_record::amendment::Ground::Other,
            disagreement: Some("I disagree.".into()),
        },
    ];
    let vital_disputed = value(
        &fhir_observation::to_fhir(&observation(Some(130.0)), PATIENT, &disputed)
            .unwrap()
            .unwrap(),
    );
    let allergy_disputed = value(
        &fhir_allergy::to_fhir(&allergy(Status::Active), PATIENT, &disputed)
            .unwrap()
            .unwrap(),
    );
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
        (
            "flag-active",
            flag(electronic_patient_record::adjustments::Status::Active),
        ),
        (
            "flag-ended",
            flag(electronic_patient_record::adjustments::Status::Ended),
        ),
        ("observation-vital", vital),
        ("observation-declined", declined),
        ("observation-disputed", vital_disputed),
        ("allergy-disputed", allergy_disputed),
        ("patient-searchset", bundle),
    ]
}

fn jar() -> PathBuf {
    let jar = std::env::var("FHIR_VALIDATOR_JAR")
        .expect("set FHIR_VALIDATOR_JAR (sh scripts/fhir-validator.sh prints the path)");
    assert!(Path::new(&jar).is_file(), "{jar} is not a file");
    PathBuf::from(jar)
}

/// One validator at a time: on a cold package cache each run installs `hl7.fhir.r4.core` into
/// `~/.fhir/packages` under a lock file, and two at once fail with "Failed to acquire lock".
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn validate(jar: &Path, files: &[PathBuf]) -> (bool, String) {
    validate_as(jar, "4.0.1", true, files)
}

/// `version` is the release to check against. The amendment-notice extension is loaded for both: it is
/// the same definition, and R5 needs it to know the extension the notice carries.
fn validate_as(jar: &Path, version: &str, with_profile: bool, files: &[PathBuf]) -> (bool, String) {
    let _turn = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut command = Command::new("java");
    command
        .arg("-jar")
        .arg(jar)
        .args(["-version", version, "-tx", "n/a"]);
    if with_profile {
        command.arg("-ig").arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fhir/StructureDefinition-amendment-notice.json"
        ));
    }
    let out = command
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

/// The IHE BALP package the claims are checked against. Bump it with `spec/audit/index.md`.
const BALP_PACKAGE: &str = "ihe.iti.balp#1.1.4";

/// The validator on `files`, with the BALP package loaded so that `meta.profile` is enforced.
fn validate_balp(jar: &Path, files: &[PathBuf]) -> (bool, String) {
    let _turn = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let out = Command::new("java")
        .arg("-jar")
        .arg(jar)
        .args(["-version", "4.0.1", "-tx", "n/a", "-ig", BALP_PACKAGE])
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

/// An audit row for `action` and `interaction` that meets its BALP profile.
fn claiming_row(action: &str, interaction: &str) -> AuditRow {
    let mut row = audit_row();
    row.action = action.into();
    row.network_address = Some("192.0.2.7".into());
    row.detail =
        Some(json!({ "interaction": interaction, "server": "https://epr.example.org/fhir" }));
    row
}

fn write_resource(name: &str, resource: &Value) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fhir-validator-balp");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(resource).unwrap()).unwrap();
    path
}

#[test]
#[ignore = "needs Java and the validator jar: set FHIR_VALIDATOR_JAR"]
fn fhir_validator_accepts_every_event_that_claims_a_balp_profile() {
    let jar = jar();
    let files: Vec<PathBuf> = [
        ("R", "read"),
        ("C", "create"),
        ("U", "update"),
        ("U", "patch"),
    ]
    .iter()
    .map(|(action, interaction)| {
        let row = claiming_row(action, interaction);
        assert!(fhir_audit::violations(&row).is_empty());
        let event = value(&fhir_audit::to_fhir(&row).unwrap());
        assert!(event["meta"]["profile"][0].is_string(), "{interaction}");
        write_resource(&format!("claims-{interaction}"), &event)
    })
    .collect();
    let (ok, text) = validate_balp(&jar, &files);
    assert!(ok, "the validator refused a claim:\n{text}");
    assert!(
        !text.contains("Error @"),
        "the validator reported errors:\n{text}"
    );
    assert!(
        text.contains("IHE.BasicAudit.Patient"),
        "the validator did not check the profile:\n{text}"
    );
}

/// The check above is worth nothing if the validator would pass anything: an event that claims the
/// profile and has lost its server agent must fail.
#[test]
#[ignore = "needs Java and the validator jar: set FHIR_VALIDATOR_JAR"]
fn fhir_validator_refuses_a_claim_the_event_does_not_meet() {
    let jar = jar();
    let mut event = value(&fhir_audit::to_fhir(&claiming_row("R", "read")).unwrap());
    event["agent"].as_array_mut().unwrap().remove(1);
    let file = write_resource("claims-without-server", &event);
    let (ok, text) = validate_balp(&jar, &[file]);
    assert!(!ok, "a broken claim passed:\n{text}");
    assert!(
        text.contains("agent:server"),
        "refused for another reason:\n{text}"
    );
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

/// `/fhir/R5` serves the R4 body converted (IO-16). Every resource it can serve is converted exactly
/// as the route does and checked against base R5 (5.0.0). `AuditEvent` is not served in R5.
#[test]
#[ignore = "needs Java and the validator jar: set FHIR_VALIDATOR_JAR"]
fn fhir_validator_accepts_every_resource_the_r5_facade_serves() {
    let jar = jar();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fhir-validator-r5");
    std::fs::create_dir_all(&dir).unwrap();
    let mut files = Vec::new();
    for (name, v) in resources() {
        if name == "audit-event" {
            continue;
        }
        let converted = electronic_patient_record::controllers::fhir_r5::to_r5(
            &v,
            "/fhir/R5/Patient?family=Smith",
        )
        .unwrap_or_else(|e| panic!("{name} does not convert to R5 without loss: {e:?}"));
        let path = dir.join(format!("{name}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(&converted).unwrap()).unwrap();
        files.push(path);
    }
    let (ok, text) = validate_as(&jar, "5.0.0", true, &files);
    assert!(ok, "the validator reported errors:\n{text}");
    assert!(
        !text.contains("Error @"),
        "the validator reported errors:\n{text}"
    );
}

/// A control for the R5 run: the oracle must be able to fail on R5.
#[test]
#[ignore = "needs Java and the validator jar: set FHIR_VALIDATOR_JAR"]
fn fhir_validator_refuses_a_broken_r5_patient() {
    let jar = jar();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fhir-validator-r5");
    std::fs::create_dir_all(&dir).unwrap();
    let mut v = value(&fhir_patient::to_fhir(&patient()).unwrap());
    v["gender"] = json!("not-a-code");
    let mut converted =
        electronic_patient_record::controllers::fhir_r5::to_r5(&v, "/fhir/R5/Patient?family=Smith")
            .unwrap();
    converted["noSuchElement"] = json!(true);
    let path = dir.join("broken-patient.json");
    std::fs::write(&path, converted.to_string()).unwrap();
    let (ok, text) = validate_as(&jar, "5.0.0", true, &[path]);
    assert!(!ok, "the broken R5 patient passed:\n{text}");
}
