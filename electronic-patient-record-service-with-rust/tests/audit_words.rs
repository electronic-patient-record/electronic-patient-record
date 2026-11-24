//! EH-16: an audit row says what kind of refusal there was, never the words of it. The words are for the caller and can
//! quote what was sent: until 2026-11-24 a refused allergy put the SNOMED CT code it was sent into the trail. Database-free:
//! it reads the source, so it finds a call and not an intent.

use electronic_patient_record::patients_store::StoreError;
use std::path::Path;

/// The shapes that put an error's own words into the JSON of an audit row.
fn words(line: &str) -> bool {
    let l: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    let refusal = ["\"refused\":", "refused(&ctx"]
        .iter()
        .any(|s| l.contains(s));
    refusal && (l.contains(".to_string()") || l.contains("format!("))
}

#[test]
fn no_controller_audits_the_words_of_a_refusal() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/controllers");
    let (mut found, mut kinds) = (Vec::new(), 0);
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        kinds += text.matches("refusal_kind()").count();
        // Statements, so that a call broken over lines is one.
        for statement in text.split(';') {
            if words(statement) {
                found.push(format!("{}: {}", path.display(), statement.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "audit the kind:\n{}",
        found.join("\n---\n")
    );
    assert!(kinds >= 15, "the census read too little: {kinds}");
}

#[test]
fn the_census_finds_what_it_is_for() {
    assert!(words("serde_json::json!({ \"refused\": e.to_string() })"));
    assert!(words(
        "refused(&ctx, &agent, purpose, Action::Create, &e.to_string())"
    ));
    assert!(words("json!({ \"refused\": format!(\"{e}\") })"));
    assert!(!words("json!({ \"refused\": e.refusal_kind() })"));
}

#[test]
fn a_kind_never_holds_the_words() {
    let secret = "SECRET".to_string();
    for e in [
        StoreError::NotFound,
        StoreError::Conflict { current: 3 },
        StoreError::BadQuery(secret.clone()),
        StoreError::NotSamePerson {
            message: secret.clone(),
            conflict_codes: serde_json::json!(["SECRET"]),
        },
        StoreError::TemporaryIdentityCleared,
        StoreError::Db(secret.clone()),
        StoreError::Unavailable(secret),
    ] {
        let kind = e.refusal_kind();
        assert!(!kind.is_empty() && !kind.contains("SECRET"), "{kind}");
        assert!(
            kind.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
            "{kind}"
        );
    }
}
