//! Every licence in the SPDX expression has `LICENSES/<lowercase-spdx-id>/index.md`,
//! `LICENSE.md` links each one, and no stray `LICENSE/` directory or `.txt` copy exists.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

/// SPDX identifiers in an expression such as `MIT OR Apache-2.0 OR GPL-3.0-only`.
fn ids(expression: &str) -> Vec<String> {
    expression
        .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
        .filter(|t| !t.is_empty() && !matches!(*t, "OR" | "AND" | "WITH"))
        .map(String::from)
        .collect()
}

fn declared() -> Vec<String> {
    let root = repo_root();
    let mut found = Vec::new();
    for manifest in [
        "electronic-patient-record-service-with-rust/Cargo.toml",
        "electronic-patient-record-matcher-with-rust/Cargo.toml",
    ] {
        let text = fs::read_to_string(root.join(manifest)).expect(manifest);
        let line = text
            .lines()
            .find(|l| l.trim_start().starts_with("license = "))
            .unwrap_or_else(|| panic!("{manifest} declares no license"));
        found.extend(ids(line.split('"').nth(1).expect("quoted license")));
    }
    found.sort();
    found.dedup();
    assert!(!found.is_empty());
    found
}

#[test]
fn every_declared_licence_has_a_text() {
    let dir = repo_root().join("LICENSES");
    for id in declared() {
        let path = dir.join(id.to_lowercase()).join("index.md");
        let text =
            fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing {}", path.display()));
        assert!(
            text.contains("```text"),
            "{} has no text fence",
            path.display()
        );
        assert!(text.len() > 500, "{} looks empty", path.display());
    }
}

#[test]
fn licence_index_links_every_text() {
    let index = fs::read_to_string(repo_root().join("LICENSE.md")).unwrap();
    for id in declared() {
        let link = format!("LICENSES/{}/index.md", id.to_lowercase());
        assert!(index.contains(&link), "LICENSE.md does not link {link}");
    }
}

#[test]
fn there_is_no_stray_licence_directory_or_text_copy() {
    let root = repo_root();
    for dir in [
        root.join("LICENSE"),
        root.join("electronic-patient-record-matcher-with-rust/LICENSE"),
    ] {
        assert!(
            !dir.exists(),
            "{} must not exist; use LICENSES/",
            dir.display()
        );
    }
}
