//! LC-15: with `autotests = false`, a test file that is neither a module of `suite.rs` nor a `[[test]]` in
//! `Cargo.toml` would never run, and nothing would say so. This census fails first.

use std::collections::BTreeSet;
use std::path::Path;

#[test]
fn every_test_file_is_in_the_suite_or_its_own_program_and_only_once() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files: BTreeSet<String> = std::fs::read_dir(root.join("tests"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .filter(|f| f != "suite.rs")
        .collect();
    let suite = std::fs::read_to_string(root.join("tests/suite.rs")).unwrap();
    let in_suite: Vec<String> = suite
        .lines()
        .filter_map(|l| {
            l.strip_prefix("#[path = \"")?
                .strip_suffix("\"]")
                .map(String::from)
        })
        .collect();
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let own: Vec<String> = manifest
        .lines()
        .filter_map(|l| {
            l.trim()
                .strip_prefix("path = \"tests/")?
                .strip_suffix('"')
                .map(String::from)
        })
        .filter(|f| f != "suite.rs")
        .collect();
    let mut seen = BTreeSet::new();
    for f in in_suite.iter().chain(&own) {
        assert!(seen.insert(f.clone()), "{f} is listed twice");
        assert!(files.contains(f), "{f} is listed but is not in tests/");
    }
    let missing: Vec<&String> = files.difference(&seen).collect();
    assert!(
        missing.is_empty(),
        "never run: {missing:?}: add it to tests/suite.rs or a [[test]] in Cargo.toml"
    );
    assert!(
        manifest.contains("autotests = false"),
        "the list is only needed, and only checked, without autotests"
    );
}
