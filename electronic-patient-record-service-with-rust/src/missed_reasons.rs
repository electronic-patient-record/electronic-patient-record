//! The deployment's reasons for a missed appointment (XP-15): the file `EPR_MISSED_REASONS_FILE` names, which replaces the
//! built-in list ([`crate::missed`]), read once at start. One deployment serves one tenant today (QA-12), so this is the
//! tenant's list.

use crate::missed::{self, Reason};
use std::sync::Arc;

/// The setting naming the file.
pub const FILE: &str = "EPR_MISSED_REASONS_FILE";

/// The most bytes the file may hold: far more than a long list needs.
pub const MAX_FILE: u64 = 1_000_000;

/// The list in force.
pub enum List {
    /// The deployment's own.
    Own(Arc<Vec<Reason>>),
    /// The built-in one.
    BuiltIn(&'static [Reason]),
}

impl std::ops::Deref for List {
    type Target = [Reason];

    fn deref(&self) -> &[Reason] {
        match self {
            List::Own(all) => all,
            List::BuiltIn(all) => all,
        }
    }
}

/// The list in force: the deployment's, read by [`init`], or the built-in one.
///
/// # Errors
///
/// A message when the built-in list does not load (a test keeps it valid).
pub fn list() -> Result<List, String> {
    match crate::runtime::Runtime::current().missed_reasons.get() {
        Some(own) => Ok(List::Own(Arc::clone(own))),
        None => missed::built_in().map(List::BuiltIn),
    }
}

/// The reasons in the file at `path`.
///
/// # Errors
///
/// A message naming the setting and the file when it cannot be read, is larger than [`MAX_FILE`] or is not a list.
pub fn load(path: &std::path::Path) -> Result<Vec<Reason>, String> {
    let shown = path.display();
    let unread = |e: std::io::Error| format!("{FILE} `{shown}` cannot be read: {e}");
    let size = std::fs::metadata(path).map_err(unread)?.len();
    if size > MAX_FILE {
        return Err(format!(
            "{FILE} `{shown}` is {size} bytes; at most {MAX_FILE}"
        ));
    }
    let text = std::fs::read_to_string(path).map_err(unread)?;
    missed::parse(&text, &shown.to_string()).map_err(|e| format!("{FILE}: {e}"))
}

/// Read the list once, at start. Returns it, so the start can log what runs.
///
/// # Errors
///
/// [`load`]'s for a file; [`list`]'s for the built-in one.
pub fn init() -> Result<List, String> {
    let setting = crate::runtime::setting(FILE);
    if let Some(path) = setting.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        let own = load(std::path::Path::new(path))?;
        let _ = crate::runtime::Runtime::current()
            .missed_reasons
            .set(Arc::new(own));
    }
    list()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_that_cannot_be_used_names_the_setting_and_the_file() {
        let e = load(std::path::Path::new("/no/such/file.toml")).unwrap_err();
        assert!(e.contains(FILE) && e.contains("/no/such/file.toml"), "{e}");
        let dir = std::env::temp_dir().join(format!("epr-missed-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("reasons.toml");
        std::fs::write(&path, "[[reason]]\ncode = \"other\"\nwords = \"Other\"\n").unwrap();
        let e = load(&path).unwrap_err();
        assert!(e.contains(FILE) && e.contains("unknown"), "{e}");
        let mine = "[[reason]]\ncode = \"unknown\"\nwords = \"Not known\"\n";
        std::fs::write(&path, mine).unwrap();
        assert_eq!(load(&path).unwrap()[0].words, "Not known");
        std::fs::write(&path, format!("{mine}#{}", "x".repeat(1_000_001))).unwrap();
        assert!(load(&path).unwrap_err().contains("at most"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
