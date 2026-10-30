//! Documentation census (OS-9): every source file or workflow that a config file, `SECURITY.md` or
//! `deny.toml` names must exist. A comment that points at a file that is not there is a promise the
//! code does not keep; this test fails when one appears. It reads paths, not meaning, so a file
//! that exists can still be described wrongly.

use std::{fs, path::Path};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The documents scanned, relative to the service crate.
fn scanned() -> Vec<String> {
    let mut files: Vec<String> = fs::read_dir(Path::new(ROOT).join("config"))
        .expect("config directory")
        .filter_map(|e| e.ok())
        .map(|e| format!("config/{}", e.file_name().to_string_lossy()))
        .filter(|n| n.ends_with(".yaml"))
        .collect();
    files.push("deny.toml".into());
    files.push("../electronic-patient-record-matcher-with-rust/deny.toml".into());
    files.push("../SECURITY.md".into());
    files
}

/// Paths of the form `src/....rs` or `.github/workflows/....yml` found in `text`.
fn named_paths(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for word in text.split(|c: char| !(c.is_alphanumeric() || "/._-".contains(c))) {
        let rs = word.starts_with("src/") && word.ends_with(".rs");
        let wf = word.starts_with(".github/workflows/") && word.ends_with(".yml");
        if rs || wf {
            found.push(word.to_string());
        }
    }
    found
}

#[test]
fn every_path_named_in_config_and_security_docs_exists() {
    let mut missing = Vec::new();
    for doc in scanned() {
        let text = fs::read_to_string(Path::new(ROOT).join(&doc)).expect("scanned file readable");
        for path in named_paths(&text) {
            // A workflow lives at the repository root; a source file in the service crate.
            let base = if path.starts_with(".github") {
                "../"
            } else {
                ""
            };
            if !Path::new(ROOT).join(base).join(&path).exists() {
                missing.push(format!("{doc} names {path}, which does not exist"));
            }
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}

#[test]
fn the_census_sees_a_planted_missing_path() {
    let found = named_paths("see src/definitely_not_a_file.rs and .github/workflows/nope.yml");
    assert_eq!(found.len(), 2);
    for p in found {
        let base = if p.starts_with(".github") { "../" } else { "" };
        assert!(!Path::new(ROOT).join(base).join(&p).exists());
    }
}
