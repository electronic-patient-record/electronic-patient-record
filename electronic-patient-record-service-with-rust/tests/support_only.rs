//! PA-3: a tripwire for NFR-12 (no score may reduce what a patient is offered). A migration that names a
//! column like `score`, `rank`, `deprioritise`, `suppress`, `withhold` or `overbook` fails unless the name
//! is on the allow-list below with a written reason. It reads names, not behaviour: a score can be hidden
//! under another name, so this is a tripwire and not proof. Database-free: it reads the migration source.

use std::path::Path;

const WATCHED: [&str; 6] = [
    "score",
    "rank",
    "deprioritise",
    "suppress",
    "withhold",
    "overbook",
];

/// (name, reason). Each reason says why the name cannot reduce what a patient is offered.
const ALLOWED: [(&str, &str); 1] = [(
    "match_score",
    "the matcher's score for a proposed patient link; it asks a person to review two records and \
     changes nothing a patient is offered",
)];

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn watched(word: &str) -> bool {
    word.split('_').any(|part| WATCHED.contains(&part))
}

fn migration_words() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migration/src");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            out.extend(words(&std::fs::read_to_string(&path).unwrap()));
        }
    }
    out
}

#[test]
fn no_migration_names_a_watched_column_without_a_reason() {
    let mut found: Vec<String> = migration_words()
        .into_iter()
        .filter(|w| watched(w) && !ALLOWED.iter().any(|(name, _)| name == w))
        .collect();
    found.sort();
    found.dedup();
    assert!(
        found.is_empty(),
        "add a reason to ALLOWED in tests/support_only.rs, or rename:\n{}",
        found.join("\n")
    );
}

#[test]
fn every_allowed_name_has_a_reason_and_is_still_used() {
    let used = migration_words();
    for (name, reason) in ALLOWED {
        assert!(reason.len() >= 20, "{name} needs a real reason");
        assert!(
            used.iter().any(|w| w == name),
            "{name} is no longer in a migration: remove it"
        );
    }
}

#[test]
fn the_tripwire_catches_a_planted_column() {
    assert!(watched("patient_priority_rank"));
    assert!(watched("risk_score"));
    assert!(watched("overbook"));
    assert!(!watched("note_free_text"));
}
