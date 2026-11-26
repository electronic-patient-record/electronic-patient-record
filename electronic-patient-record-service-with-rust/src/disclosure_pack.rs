//! The tenant's disclosure pack (DR-3): `EPR_DISCLOSURE_RULES_FILE`, read and checked at start. A pack that does not
//! load stops the service and says why; no file is no pack, and every setting is then the spec's default, said as one
//! ([`crate::disclosure_rules::Setting::Default`]). **Nothing reads the pack to decide a read yet.**

use crate::disclosure_rules::{self, Pack};

/// The setting's name.
pub const SETTING: &str = "EPR_DISCLOSURE_RULES_FILE";

/// The pack at `path`, or none when no path is set.
///
/// # Errors
///
/// A blank path, a file that cannot be read, or a pack that [`disclosure_rules::parse`] refuses, each naming the setting.
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
    disclosure_rules::parse(&text, &format!("{SETTING} {path}")).map(Some)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn written(name: &str, text: &str) -> String {
        let path = std::env::temp_dir().join(format!("epr-dr3-{name}-{}.toml", std::process::id()));
        std::fs::write(&path, text).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn no_file_is_no_pack_and_a_good_one_loads() {
        assert_eq!(load_from(None).unwrap(), None);
        let good = written(
            "good",
            "[pack]\nid = \"x\"\nplace = \"P\"\nlaw_as_of = \"2026-11-26\"\n",
        );
        let pack = load_from(Some(&good)).unwrap().expect("a pack");
        assert!(pack.version().starts_with("x@2026-11-26#"));
        let _ = std::fs::remove_file(good);
    }

    #[test]
    fn a_blank_path_a_missing_file_and_a_contradiction_stop_the_start_and_say_why() {
        assert!(load_from(Some(" ")).unwrap_err().contains("blank"));
        let missing = load_from(Some("/nonexistent/epr-dr3.toml")).unwrap_err();
        assert!(missing.starts_with(SETTING), "{missing}");
        let bad = written(
            "bad",
            "[pack]\nid = \"x\"\nplace = \"P\"\nlaw_as_of = \"2026-11-26\"\n[[rule]]\nkey = \"emergency-beats-refusal\"\nvalue = \"yes\"\nsource = \"s\"\n",
        );
        let e = load_from(Some(&bad)).unwrap_err();
        assert!(e.contains("patient-may-close-emergency"), "{e}");
        let _ = std::fs::remove_file(bad);
    }
}
