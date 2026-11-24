//! A coded term beside the words: the code system and the code. Only the shape is checked here; a SNOMED CT identifier
//! is checked by the service (`interop::terminology`). No code is looked up.

use serde::{Deserialize, Serialize};

/// The longest system URI kept.
pub const SYSTEM_LIMIT: usize = 200;
/// The longest code kept.
pub const CODE_LIMIT: usize = 64;

/// A coded term.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coding {
    /// The code system URI.
    pub system: String,
    /// The code.
    pub code: String,
}

/// What is wrong with the shape of a coding, in words that never quote it; empty if nothing.
#[must_use]
pub fn problems(c: &Coding) -> Vec<String> {
    let mut out = Vec::new();
    let uri = ["http://", "https://", "urn:"]
        .iter()
        .any(|p| c.system.starts_with(p));
    if !uri || c.system.len() > SYSTEM_LIMIT || c.system.chars().any(char::is_control) {
        out.push("the code system must be a URI (http, https or urn)".into());
    }
    if c.code.trim().is_empty() || c.code.len() > CODE_LIMIT || c.code.chars().any(char::is_control)
    {
        out.push("the code must be 1 to 64 characters".into());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coding(system: &str, code: &str) -> Coding {
        Coding {
            system: system.into(),
            code: code.into(),
        }
    }

    #[test]
    fn a_uri_and_a_short_code_pass() {
        for system in ["http://snomed.info/sct", "https://x.test", "urn:oid:1.2"] {
            assert!(problems(&coding(system, "22298006")).is_empty(), "{system}");
        }
    }

    #[test]
    fn each_fault_is_named_and_nothing_is_quoted() {
        let bad = problems(&coding("javascript:SECRET", " "));
        assert_eq!(bad.len(), 2, "{bad:?}");
        assert!(bad.iter().all(|p| !p.contains("SECRET")));
        assert_eq!(
            problems(&coding("urn:x", &"1".repeat(CODE_LIMIT + 1))).len(),
            1
        );
        assert_eq!(
            problems(&coding(&format!("urn:{}", "x".repeat(SYSTEM_LIMIT)), "1")).len(),
            1
        );
        assert_eq!(problems(&coding("urn:x", "1\n2")).len(), 1);
    }
}
