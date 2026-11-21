//! The demo binary: two patient files in, an explainable result out; a wrong call or a bad file is refused.

use std::process::Command;

const ADA: &str = r#"{"names":[{"family":"Lovelace","given":["Ada"]}],"birth_date":"1815-12-10"}"#;

fn demo() -> Command {
    Command::new(env!("CARGO_BIN_EXE_electronic-patient-record-matcher"))
}

fn file(dir: &std::path::Path, name: &str, body: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    path.to_str().unwrap().to_owned()
}

#[test]
fn two_patient_files_print_a_decision_as_json() {
    let dir = tempfile_dir("two_files");
    let (a, b) = (file(&dir, "a.json", ADA), file(&dir, "b.json", ADA));
    let out = demo().args([&a, &b]).output().unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json.get("decision").is_some());
}

#[test]
fn a_wrong_number_of_arguments_prints_usage_and_exits_with_2() {
    let out = demo().output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage:"));
}

#[test]
fn a_missing_or_malformed_file_is_an_error_not_a_decision() {
    let dir = tempfile_dir("bad_file");
    let good = file(&dir, "a.json", ADA);
    let bad = file(&dir, "bad.json", "not json");
    let missing = dir.join("none.json").to_str().unwrap().to_owned();
    for other in [bad, missing] {
        let out = demo().args([&good, &other]).output().unwrap();
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
    }
}

fn tempfile_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("epr-matcher-demo-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
