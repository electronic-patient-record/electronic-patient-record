//! Role views (WL-7): a role that sees a named subset of the record, as data.
//!
//! `src/role_views.toml` is read into the build. Each view names a role whose permissions come only
//! from a view ([`Role::viewed`]), and the kinds of record it may read and search. [`crate::policy`] asks
//! [`reads`] for such a role instead of its own table, so changing what a vaccinator sees is a change to
//! the data file, reviewed like any other, not to the match in `policy.rs`. A view grants reads only;
//! clinical content still needs a care relationship, a fitting purpose and the patient's consent.

use crate::policy::{Resource, Role};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

/// The data file, as built in.
pub const SOURCE: &str = include_str!("role_views.toml");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    view: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    role: String,
    reads: Vec<String>,
    source: String,
}

/// Parse a views file: by role, the kinds of record it may read.
///
/// # Errors
///
/// A message naming the problem: a file that is not TOML of this shape, an unknown role or resource, a
/// role whose permissions do not come from a view, a role named twice, an empty list, or no source.
pub fn parse(text: &str) -> Result<BTreeMap<Role, BTreeSet<Resource>>, String> {
    let file: File = toml::from_str(text).map_err(|e| format!("role views: {e}"))?;
    let mut out = BTreeMap::new();
    for entry in file.view {
        let role = Role::parse(&entry.role)
            .ok_or_else(|| format!("role views: unknown role {:?}", entry.role))?;
        if !role.viewed() {
            return Err(format!(
                "role views: {:?} has its own permissions; a view is only for a role that has none",
                entry.role
            ));
        }
        if entry.source.trim().is_empty() {
            return Err(format!("role views: {:?} needs a source", entry.role));
        }
        if entry.reads.is_empty() {
            return Err(format!("role views: {:?} reads nothing", entry.role));
        }
        let reads = entry
            .reads
            .iter()
            .map(|r| {
                Resource::parse(r).ok_or_else(|| format!("role views: unknown resource {r:?}"))
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        if out.insert(role, reads).is_some() {
            return Err(format!("role views: {:?} is named twice", entry.role));
        }
    }
    Ok(out)
}

static VIEWS: LazyLock<BTreeMap<Role, BTreeSet<Resource>>> = LazyLock::new(|| {
    // The built-in file is proved to parse by the tests below, so this cannot fail in a tested build.
    parse(SOURCE).unwrap_or_default()
});

/// Whether `role` may read and search `resource` under its view. A role with no view reads nothing.
#[must_use]
pub fn reads(role: Role, resource: Resource) -> bool {
    VIEWS.get(&role).is_some_and(|r| r.contains(&resource))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_file_parses_and_the_vaccinator_sees_medicines_and_allergies() {
        let views = parse(SOURCE).expect("src/role_views.toml must parse");
        let expected: BTreeSet<Resource> = [
            Resource::PatientIdentity,
            Resource::AllergyIntolerance,
            Resource::MedicationRequest,
        ]
        .into_iter()
        .collect();
        assert_eq!(views[&Role::Vaccinator], expected);
        assert!(reads(Role::Vaccinator, Resource::AllergyIntolerance));
        assert!(!reads(Role::Vaccinator, Resource::Observation));
        assert!(!reads(Role::Vaccinator, Resource::DiagnosticReport));
        assert!(
            !reads(Role::Clinician, Resource::AllergyIntolerance),
            "a view never widens another role"
        );
    }

    #[test]
    fn a_bad_file_is_refused_with_the_reason() {
        let one = |role: &str, reads: &str, source: &str| {
            format!("[[view]]\nrole = \"{role}\"\nreads = [{reads}]\nsource = \"{source}\"\n")
        };
        for (text, words) in [
            (one("nobody", "\"Observation\"", "s"), "unknown role"),
            (one("vaccinator", "\"Everything\"", "s"), "unknown resource"),
            (one("clinician", "\"Observation\"", "s"), "own permissions"),
            (one("vaccinator", "", "s"), "reads nothing"),
            (one("vaccinator", "\"Observation\"", " "), "needs a source"),
            (
                one("vaccinator", "\"Observation\"", "s")
                    + &one("vaccinator", "\"Observation\"", "s"),
                "named twice",
            ),
            (
                "[[view]]\nrole = \"vaccinator\"\nwrites = []\n".to_string(),
                "role views",
            ),
        ] {
            let err = parse(&text).expect_err(&text);
            assert!(err.contains(words), "{text}: {err}");
        }
    }
}
