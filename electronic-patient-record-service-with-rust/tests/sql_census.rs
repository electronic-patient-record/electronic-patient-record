//! CQ-9: no query compares an id column as text. `patient_id::text = lower($1)` (and `= ANY($1::text[])`)
//! hides the column from its index, so every lookup scans the whole index; bind `$1::uuid` instead,
//! after `ids::uuid` has checked the value. Database-free: it reads the source.

use std::path::Path;

fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push((
                path.display().to_string(),
                std::fs::read_to_string(&path).unwrap(),
            ));
        }
    }
}

#[test]
fn no_id_column_is_compared_as_text() {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let mut found = Vec::new();
    for (path, text) in &files {
        for (n, line) in text.lines().enumerate() {
            let l = line.replace(' ', "");
            if l.contains("_id::text=") || l.contains("_id::textIN") {
                found.push(format!("{path}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "compare ids as uuid, not text:\n{}",
        found.join("\n")
    );
}

/// CQ-12: one `pg` and one `db_err` (in `src/store.rs`, the latter through `store_errors!`), and a
/// version conflict is read from the error kind, never by matching a constraint name in the text.
#[test]
fn store_helpers_are_shared_and_conflicts_are_typed() {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let mut found = Vec::new();
    for (path, text) in &files {
        if path.ends_with("src/store.rs") {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("fn pg(") || t.starts_with("fn db_err(") || line.contains("_pkey\")") {
                found.push(format!("{path}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "use crate::store:\n{}", found.join("\n"));
}
