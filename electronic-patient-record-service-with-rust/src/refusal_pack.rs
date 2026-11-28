//! The tenant's pack of refusal-of-treatment rules (RF-3): `EPR_REFUSAL_RULES_FILE`, read and checked at start. A pack that
//! does not load stops the service and says why; **no file is no rule**, and nothing is then said of what the place's law
//! asks. The author's pack for England and Wales ships in the domain crate (`refusal_rules::ENGLAND_WALES`) unreviewed; it
//! is read only when a deployment names a file. Nothing here says whether a person may be treated, and no withdrawal is
//! ended by a pack yet (`treatment_refusals_store` still keeps every withdrawal heard as doubted).

use crate::refusal_rules::{self, Pack};
use serde::Serialize;

/// The setting's name.
pub const SETTING: &str = "EPR_REFUSAL_RULES_FILE";

/// The pack at `path`, or none when no path is set.
///
/// # Errors
///
/// A blank path, a file that cannot be read, or a pack that [`refusal_rules::parse`] refuses (a contradiction, an
/// unknown key or value, a rule with no source), each naming the setting.
pub fn load_from(path: Option<&str>) -> Result<Option<Pack>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    if path.trim().is_empty() {
        return Err(format!(
            "{SETTING} is set but blank: unset it, or name the pack"
        ));
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{SETTING}: {path}: {e}"))?;
    refusal_rules::parse(&text, &format!("{SETTING} {path}")).map(Some)
}

/// The pack in force for the current runtime.
///
/// # Errors
///
/// As [`load_from`].
pub fn load() -> Result<Option<Pack>, String> {
    load_from(crate::runtime::setting(SETTING).as_deref())
}

/// Check the pack at start: one that does not load stops the service (RCA-IN-4).
///
/// # Errors
///
/// As [`load_from`].
pub fn check_settings() -> Result<(), String> {
    load().map(|_| ())
}

/// The pack in force as a reader is shown it beside what it is used for: its place, the day the law was read, who
/// reviewed it and whether that review stands on `today`. Never the rules' verdict on a person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InForce {
    /// The pack's head: id, place, the day the law was read, the lawyer, the clinician and the days.
    #[serde(flatten)]
    pub head: refusal_rules::Head,
    /// `unreviewed`, `lapsed` or `current`.
    pub review: refusal_rules::Review,
}

impl InForce {
    /// What `pack` is shown as on `today`.
    #[must_use]
    pub fn of(pack: &Pack, today: jiff::civil::Date) -> Self {
        InForce {
            head: pack.head.clone(),
            review: pack.review(today),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(name: &str, text: &str) -> String {
        let path = std::env::temp_dir().join(format!("epr-rf3-{name}-{}.toml", std::process::id()));
        std::fs::write(&path, text).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn no_file_is_no_rule_and_the_shipped_pack_loads_unreviewed() {
        assert_eq!(load_from(None).unwrap(), None);
        let path = written("ew", refusal_rules::ENGLAND_WALES);
        let pack = load_from(Some(&path)).unwrap().expect("a pack");
        let today = crate::clock::parse_day("2026-11-28").unwrap();
        let shown = InForce::of(&pack, today);
        assert_eq!(shown.review, refusal_rules::Review::Unreviewed);
        let json = serde_json::to_value(&shown).unwrap();
        assert_eq!(json["place"], pack.head.place.as_str());
        assert_eq!(json["review"], "unreviewed");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_blank_path_a_missing_file_and_a_bad_pack_stop_the_start_and_say_why() {
        assert!(load_from(Some(" ")).unwrap_err().contains("blank"));
        let missing = load_from(Some("/nonexistent/epr-rf3.toml")).unwrap_err();
        assert!(missing.starts_with(SETTING), "{missing}");
        let unknown = written(
            "unknown",
            &format!(
                "{}\n[[rule]]\nkey = \"no-such-rule\"\nvalue = \"x\"\nsource = \"s\"\n",
                refusal_rules::ENGLAND_WALES
            ),
        );
        let e = load_from(Some(&unknown)).unwrap_err();
        assert!(e.starts_with(SETTING), "{e}");
        let _ = std::fs::remove_file(unknown);
    }
}
