//! SG-2: what is not built for people who are bereaved, and must stay that way. No family or household
//! view, no list of people bereaved by suicide, no automatic referral, no anniversary mailing and no
//! grief score (`spec/survivor-grief/index.md`, "Do not build"), and no outbound message
//! derived from a date of death, a due date or a dead child's birth date.
//!
//! Database-free: it reads the source, so it finds a name or a call, not an intent. A comment that uses
//! a listed word fails too, and a thing built under another name passes. It is a prompt for someone to
//! look, not a proof. Nothing enqueues today, so the date rule guards the first caller.

use std::path::{Path, PathBuf};

/// Shapes compared with underscores and hyphens removed and in lower case, as the risk census does, so
/// `family_view`, `familyView` and `family-view` are one shape. The matcher's household weight is
/// evidence about an address and is not one of these.
const SHAPES: &[&str] = &[
    "familyview",
    "familyrecord",
    "householdview",
    "householdrecord",
    "anniversary",
    "bereavedlist",
    "bereavementlist",
    "bereavedbysuicide",
    "suicidebereaved",
    "griefscore",
    "griefrisk",
    "griefrating",
    "griefflag",
    "griefmarker",
    "bereavementscore",
    "bereavementrisk",
    "deathreminder",
    "deathdatereminder",
];

/// A helpline or a service's name in core code, spaces removed and in lower case. A directory is a pack
/// with an owner and a review date (SG-4): a number in code goes out of date with nobody looking.
const DIRECTORY_SHAPES: &[&str] = &["116123", "0808", "0800", "samaritans", "cruse"];

/// Files that may call `outbox::enqueue`. A new one is a new reason to contact someone: list it here
/// only after looking at what starts it.
const ENQUEUERS: &[&str] = &[];

/// Words that say a file works from one of the dates a message must not come from. A real need, such
/// as a person asking for a contact on a date, is their plan on their pathway, and has to be argued
/// here rather than slipped past.
const DATES: &[&str] = &[
    "death",
    "deceased",
    "occurred",
    "duedate",
    "estimateddelivery",
    "birthdate",
];

/// The code outside the `#[cfg(test)]` module, because a test may name what the code may not do.
fn code(text: &str) -> &str {
    text.split("#[cfg(test)]").next().unwrap_or(text)
}

fn fold(text: &str) -> String {
    text.chars()
        .filter(|c| *c != '_' && *c != '-')
        .collect::<String>()
        .to_lowercase()
}

fn refused_shape(line: &str) -> Option<&'static str> {
    let folded = fold(line);
    SHAPES.iter().copied().find(|s| folded.contains(s))
}

fn directory_shape(line: &str) -> Option<&'static str> {
    let folded: String = line
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    DIRECTORY_SHAPES
        .iter()
        .copied()
        .find(|s| folded.contains(s))
}

fn date_word(text: &str) -> Option<&'static str> {
    let folded = fold(text);
    DATES.iter().copied().find(|d| folded.contains(d))
}

fn calls_enqueue(text: &str) -> bool {
    text.contains("enqueue(")
}

fn sources(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| matches!(e.to_str(), Some("rs" | "ts" | "svelte" | "sql")))
        {
            let text = std::fs::read_to_string(&path).unwrap();
            out.push((path, text));
        }
    }
}

fn service() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn service_source() -> Vec<(String, String)> {
    let mut files = Vec::new();
    sources(&service().join("src"), &mut files);
    files
        .into_iter()
        .map(|(path, text)| {
            let name = path.strip_prefix(service()).unwrap().display().to_string();
            (name, text)
        })
        .collect()
}

fn read(name: &str) -> String {
    std::fs::read_to_string(service().join(name)).unwrap()
}

#[test]
fn nothing_in_the_core_is_shaped_like_something_we_refuse_to_build() {
    let root = service().parent().unwrap().to_path_buf();
    let mut files = Vec::new();
    for dir in [
        service().join("src"),
        service().join("migration/src"),
        root.join("electronic-patient-record-matcher-with-rust/src"),
        root.join("electronic-patient-record-ui-with-svelte/src"),
    ] {
        assert!(dir.is_dir(), "the census must reach {}", dir.display());
        sources(&dir, &mut files);
    }
    assert!(files.len() > 100, "the census read too few files");
    let mut found = Vec::new();
    for (path, text) in &files {
        for (n, line) in text.lines().enumerate() {
            if let Some(shape) = refused_shape(line) {
                found.push(format!("{}:{}: {shape}", path.display(), n + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "refused in spec/survivor-grief/index.md (SG-2):\n{}",
        found.join("\n")
    );
}

#[test]
fn no_support_service_or_helpline_is_written_into_core_code() {
    let root = service().parent().unwrap().to_path_buf();
    let mut files = Vec::new();
    sources(&service().join("src"), &mut files);
    sources(
        &root.join("electronic-patient-record-ui-with-svelte/src"),
        &mut files,
    );
    let mut found = Vec::new();
    for (path, text) in &files {
        for (n, line) in text.lines().enumerate() {
            if let Some(shape) = directory_shape(line) {
                found.push(format!("{}:{}: {shape}", path.display(), n + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "a directory is a pack with an owner and a review date (SG-4):\n{}",
        found.join("\n")
    );
}

#[test]
fn no_route_is_about_a_family_a_household_or_grief() {
    let routes = read("src/routes.rs").to_lowercase();
    for word in ["family", "household", "bereav", "grief"] {
        assert!(
            !routes.contains(word),
            "{word}: access is per person (SG-2)"
        );
    }
}

#[test]
fn no_outbound_message_starts_from_a_date_of_death_a_due_date_or_a_birth_date() {
    let mut callers: Vec<String> = Vec::new();
    for (name, text) in service_source() {
        if name != "src/outbox.rs" && calls_enqueue(code(&text)) {
            if let Some(word) = date_word(code(&text)) {
                panic!("{name} enqueues and reads {word}: no contact on a date (SG-2)");
            }
            callers.push(name);
        }
    }
    callers.sort();
    assert_eq!(
        callers, ENQUEUERS,
        "a new reason to contact someone: look at what starts it, then list it"
    );
}

/// The two places that act on a death use only that one stands. The date must not reach them.
#[test]
fn the_hold_and_the_proxy_stop_do_not_read_the_date() {
    for name in ["src/outbox.rs", "src/relationships.rs"] {
        let text = read(name);
        for field in ["occurred", "dates_differ", "death_date"] {
            assert!(
                !code(&text).contains(field),
                "{name} reads {field}: a stop needs no date"
            );
        }
    }
}

/// A death starts no referral, and a referral does not look for one. A referral comes in with its
/// consent recorded.
#[test]
fn a_death_does_not_start_a_referral() {
    for name in [
        "src/death.rs",
        "src/death_store.rs",
        "src/controllers/deaths.rs",
        "src/adt_feed.rs",
    ] {
        assert!(
            !read(name).to_lowercase().contains("referral"),
            "{name}: no automatic referral (SG-2)"
        );
    }
    for (name, text) in service_source() {
        if name.contains("referral") {
            assert!(
                !text.to_lowercase().contains("death"),
                "{name}: no automatic referral (SG-2)"
            );
        }
    }
}

/// The census would be worthless if it could not fire, or if it fired on the matcher's household weight.
#[test]
fn the_census_finds_what_is_planted_and_leaves_ordinary_words_alone() {
    for planted in [
        "family_view TEXT NOT NULL",
        "let householdView = load();",
        "pub fn anniversary_mailing()",
        "\"grief-score\": 3",
        "bereaved_by_suicide BOOLEAN",
    ] {
        assert!(refused_shape(planted).is_some(), "missed {planted}");
    }
    for fine in [
        "const HOUSEHOLD_WEIGHT_FACTOR: f64 = 0.5;",
        "family_name: String,",
        "deathTitle: \"Death\",",
    ] {
        assert_eq!(refused_shape(fine), None, "wrongly flagged {fine}");
    }
    assert!(directory_shape("call 116 123 at any time").is_some());
    assert!(directory_shape("\"Cruse\": \"self-referral\"").is_some());
    assert_eq!(directory_shape("const LIMIT: u32 = 800;"), None);
    assert_eq!(date_word("let at = patient.birth_date;"), Some("birthdate"));
    assert_eq!(date_word("let at = referral.due_date;"), Some("duedate"));
    assert_eq!(date_word("outbox::enqueue(db, new)"), None);
    assert!(calls_enqueue("outbox::enqueue(db, new).await?"));
    assert_eq!(code("real\n#[cfg(test)]\nfake"), "real\n");
}
