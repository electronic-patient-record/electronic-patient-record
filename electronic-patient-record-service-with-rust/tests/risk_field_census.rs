//! SW-1: no risk score in the core. NICE NG225 advises against risk tools and low, medium or high
//! stratification for predicting suicide or for deciding treatment or discharge, so a field shaped
//! like one must not appear in the service, the migrations, the matcher or the UI. See
//! `spec/suicide-watch/index.md`. A pack that needs one, such as a US deployment's validated
//! screen, must declare it in `DECLARED` with a reason.
//!
//! This matches field shapes, not the word "risk": an allergy's FHIR criticality ("high risk") is a
//! different thing and stays. Database-free: it reads the source.

use std::path::Path;

/// Declared exceptions, as `path` substrings. Empty: the core declares none.
const DECLARED: &[&str] = &[];

/// Field shapes, compared with underscores and hyphens removed and in lower case, so `risk_level`,
/// `riskLevel` and `risk-level` are one shape.
const SHAPES: &[&str] = &[
    "risklevel",
    "riskscore",
    "riskband",
    "riskrating",
    "riskcategory",
    "risktier",
    "riskstratif",
    "suiciderisk",
    "suicidescore",
    "selfharmrisk",
];

fn risk_shape(line: &str) -> Option<&'static str> {
    let folded: String = line
        .chars()
        .filter(|c| *c != '_' && *c != '-')
        .collect::<String>()
        .to_lowercase();
    SHAPES.iter().copied().find(|s| folded.contains(s))
}

fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| matches!(e.to_str(), Some("rs" | "ts" | "svelte" | "sql")))
        {
            out.push((
                path.display().to_string(),
                std::fs::read_to_string(&path).unwrap(),
            ));
        }
    }
}

#[test]
fn no_field_in_the_core_is_shaped_like_a_risk_score() {
    let service = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = service.parent().unwrap();
    let mut files = Vec::new();
    for dir in [
        service.join("src"),
        service.join("migration/src"),
        root.join("electronic-patient-record-matcher-with-rust/src"),
        root.join("electronic-patient-record-ui-with-svelte/src"),
    ] {
        assert!(dir.is_dir(), "the census must reach {}", dir.display());
        sources(&dir, &mut files);
    }
    assert!(files.len() > 100, "the census read too few files");
    let mut found = Vec::new();
    for (path, text) in &files {
        if DECLARED.iter().any(|d| path.contains(d)) {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            if let Some(shape) = risk_shape(line) {
                found.push(format!("{path}:{}: {shape}", n + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "a risk score does not belong in the core (SW-1, spec/suicide-watch/index.md); a pack must declare one:\n{}",
        found.join("\n")
    );
}

/// The census would be worthless if it could not fire, or if it fired on allergy wording.
#[test]
fn the_census_finds_a_planted_field_and_leaves_allergy_criticality_alone() {
    for planted in [
        "risk_level TEXT NOT NULL",
        "pub suicide_risk: Option<String>,",
        "let riskScore = 3;",
        "\"self-harm-risk\": \"high\"",
        "riskStratification",
    ] {
        assert!(risk_shape(planted).is_some(), "missed {planted}");
    }
    for fine in [
        "allergyRiskHigh: \"high risk\",",
        "criticality: Option<String>,",
        "const RISK: Record<string, string> = {",
        "the risk of a wrong link",
    ] {
        assert_eq!(risk_shape(fine), None, "wrongly flagged {fine}");
    }
}
