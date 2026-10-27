//! The terminology service (RC-7): what a code means, whether it is valid, which codes a value set
//! holds and what a code translates to. A trait, so the service does not care whether the answer
//! comes from Snowstorm or from the embedded fixture the tests use.
//!
//! **A code is only ever judged against a pinned version.** A call that names no version, or one the
//! service has not pinned, is refused with [`TerminologyError::Unpinned`] before any lookup: "valid"
//! means nothing without the release it was valid in, and a silent default would let a code change
//! meaning between two reads.
//!
//! Only the embedded [`Fixture`] exists. It holds a handful of codes for tests; its version names
//! (`fixture-1`) are not real releases, and it must never answer for clinical use. The Snowstorm
//! adapter is not built.

use async_trait::async_trait;
use serde::Serialize;

/// SNOMED CT.
pub const SNOMED: &str = "http://snomed.info/sct";
/// LOINC.
pub const LOINC: &str = "http://loinc.org";

/// The fixture's one version of every system it holds.
pub const FIXTURE_VERSION: &str = "fixture-1";
/// The fixture's value set of the supported vital signs.
pub const FIXTURE_VITALS: &str = "urn:epr:fixture:vital-signs";

/// A code system at one version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pinned {
    /// Code system URI.
    pub system: String,
    /// The release the service has pinned.
    pub version: String,
}

/// One concept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Concept {
    /// Code system URI.
    pub system: String,
    /// The version it was found in.
    pub version: String,
    /// The code.
    pub code: String,
    /// Its preferred display.
    pub display: String,
}

/// The answer to `$validate-code`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Validation {
    /// Whether the code is in the system at that version (and the display, if given, matches).
    pub result: bool,
    /// The display held for the code, when there is one.
    pub display: Option<String>,
    /// Why not, when `result` is false.
    pub message: Option<String>,
}

/// One result of `$translate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Match {
    /// How the target relates to the source (`relatedto`, `equivalent`, `wider`, `narrower`).
    pub relationship: String,
    /// The target concept.
    pub concept: Concept,
}

/// Why a terminology call failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminologyError {
    /// No version was named, or the version is not one the service has pinned.
    Unpinned(String),
    /// The system is not held.
    UnknownSystem(String),
    /// The code is not in the system at that version.
    NotFound {
        /// System.
        system: String,
        /// Version.
        version: String,
        /// Code.
        code: String,
    },
    /// The value set is not held.
    UnknownValueSet(String),
    /// The server could not be reached or failed.
    Unavailable(String),
}

impl std::fmt::Display for TerminologyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unpinned(m) => write!(f, "{m}"),
            Self::UnknownSystem(s) => write!(f, "the code system {s} is not held"),
            Self::NotFound {
                system,
                version,
                code,
            } => write!(f, "the code {code} is not in {system} {version}"),
            Self::UnknownValueSet(v) => write!(f, "the value set {v} is not held"),
            Self::Unavailable(m) => write!(f, "the terminology server is unavailable: {m}"),
        }
    }
}

impl std::error::Error for TerminologyError {}

/// The four operations. A caller names the version every time.
#[async_trait]
pub trait TerminologyService: Send + Sync {
    /// The systems and versions this service has pinned.
    fn pinned(&self) -> Vec<Pinned>;

    /// `$lookup`: the concept for a code.
    ///
    /// # Errors
    ///
    /// [`TerminologyError`]: unpinned, unknown system or code, or unavailable.
    async fn lookup(
        &self,
        system: &str,
        version: Option<&str>,
        code: &str,
    ) -> Result<Concept, TerminologyError>;

    /// `$validate-code`. A code that is not there is `result: false`, not an error; an unpinned
    /// version is an error, because the question cannot be answered.
    ///
    /// # Errors
    ///
    /// [`TerminologyError`]: unpinned, unknown system, or unavailable.
    async fn validate_code(
        &self,
        system: &str,
        version: Option<&str>,
        code: &str,
        display: Option<&str>,
    ) -> Result<Validation, TerminologyError>;

    /// `$expand`: the concepts of a value set, at most `count` of them, in code order.
    ///
    /// # Errors
    ///
    /// [`TerminologyError`]: unpinned, unknown value set, or unavailable.
    async fn expand(
        &self,
        value_set: &str,
        version: Option<&str>,
        count: usize,
    ) -> Result<Vec<Concept>, TerminologyError>;

    /// `$translate`: the targets of a code in another system. No target is an empty list, which is
    /// not a statement that no equivalent exists.
    ///
    /// # Errors
    ///
    /// [`TerminologyError`]: unpinned, unknown system, or unavailable.
    async fn translate(
        &self,
        system: &str,
        version: Option<&str>,
        code: &str,
        target_system: &str,
    ) -> Result<Vec<Match>, TerminologyError>;
}

/// The embedded subset for tests. No network, no files.
#[derive(Debug, Default, Clone, Copy)]
pub struct Fixture;

const CODES: &[(&str, &str, &str)] = &[
    (LOINC, "8480-6", "Systolic blood pressure"),
    (LOINC, "8462-4", "Diastolic blood pressure"),
    (LOINC, "8867-4", "Heart rate"),
    (LOINC, "9279-1", "Respiratory rate"),
    (LOINC, "8310-5", "Body temperature"),
    (
        LOINC,
        "59408-5",
        "Oxygen saturation in Arterial blood by Pulse oximetry",
    ),
    (LOINC, "29463-7", "Body weight"),
    (LOINC, "8302-2", "Body height"),
    (SNOMED, "22298006", "Myocardial infarction (disorder)"),
    (SNOMED, "364075005", "Heart rate (observable entity)"),
];

/// (source system, source code, target system, target code, relationship)
const MAPS: &[(&str, &str, &str, &str, &str)] =
    &[(LOINC, "8867-4", SNOMED, "364075005", "relatedto")];

impl Fixture {
    fn check(system: &str, version: Option<&str>) -> Result<String, TerminologyError> {
        let held = [SNOMED, LOINC];
        let Some(v) = version.filter(|v| !v.trim().is_empty()) else {
            return Err(TerminologyError::Unpinned(format!(
                "no version of {system} was named; a code is judged only against a pinned version"
            )));
        };
        if !held.contains(&system) {
            return Err(TerminologyError::UnknownSystem(system.to_string()));
        }
        if v != FIXTURE_VERSION {
            return Err(TerminologyError::Unpinned(format!(
                "{system} {v} is not pinned; the pinned version is {FIXTURE_VERSION}"
            )));
        }
        Ok(v.to_string())
    }

    fn find(system: &str, version: &str, code: &str) -> Option<Concept> {
        CODES
            .iter()
            .find(|(s, c, _)| *s == system && *c == code)
            .map(|(s, c, d)| Concept {
                system: (*s).into(),
                version: version.into(),
                code: (*c).into(),
                display: (*d).into(),
            })
    }
}

#[async_trait]
impl TerminologyService for Fixture {
    fn pinned(&self) -> Vec<Pinned> {
        [SNOMED, LOINC]
            .into_iter()
            .map(|s| Pinned {
                system: s.into(),
                version: FIXTURE_VERSION.into(),
            })
            .collect()
    }

    async fn lookup(
        &self,
        system: &str,
        version: Option<&str>,
        code: &str,
    ) -> Result<Concept, TerminologyError> {
        let v = Self::check(system, version)?;
        Self::find(system, &v, code).ok_or(TerminologyError::NotFound {
            system: system.into(),
            version: v,
            code: code.into(),
        })
    }

    async fn validate_code(
        &self,
        system: &str,
        version: Option<&str>,
        code: &str,
        display: Option<&str>,
    ) -> Result<Validation, TerminologyError> {
        let v = Self::check(system, version)?;
        Ok(match Self::find(system, &v, code) {
            None => Validation {
                result: false,
                display: None,
                message: Some(format!("the code {code} is not in {system} {v}")),
            },
            Some(c) if display.is_some_and(|d| d != c.display) => Validation {
                result: false,
                message: Some(format!("the display does not match {:?}", c.display)),
                display: Some(c.display),
            },
            Some(c) => Validation {
                result: true,
                display: Some(c.display),
                message: None,
            },
        })
    }

    async fn expand(
        &self,
        value_set: &str,
        version: Option<&str>,
        count: usize,
    ) -> Result<Vec<Concept>, TerminologyError> {
        let v = Self::check(LOINC, version)?;
        if value_set != FIXTURE_VITALS {
            return Err(TerminologyError::UnknownValueSet(value_set.into()));
        }
        let mut out: Vec<Concept> = CODES
            .iter()
            .filter(|(s, _, _)| *s == LOINC)
            .filter_map(|(s, c, _)| Self::find(s, &v, c))
            .collect();
        out.sort_by(|a, b| a.code.cmp(&b.code));
        out.truncate(count);
        Ok(out)
    }

    async fn translate(
        &self,
        system: &str,
        version: Option<&str>,
        code: &str,
        target_system: &str,
    ) -> Result<Vec<Match>, TerminologyError> {
        let v = Self::check(system, version)?;
        Ok(MAPS
            .iter()
            .filter(|(s, c, t, _, _)| *s == system && *c == code && *t == target_system)
            .filter_map(|(_, _, t, tc, rel)| {
                Self::find(t, &v, tc).map(|concept| Match {
                    relationship: (*rel).into(),
                    concept,
                })
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V: Option<&str> = Some(FIXTURE_VERSION);

    fn unpinned<T: std::fmt::Debug>(r: Result<T, TerminologyError>) -> bool {
        matches!(r, Err(TerminologyError::Unpinned(_)))
    }

    #[tokio::test]
    async fn a_call_without_a_pinned_version_is_refused_on_every_operation() {
        let f = Fixture;
        for version in [None, Some(""), Some("  "), Some("2099-01-01")] {
            assert!(unpinned(f.lookup(LOINC, version, "8867-4").await));
            assert!(unpinned(
                f.validate_code(LOINC, version, "8867-4", None).await
            ));
            assert!(unpinned(f.expand(FIXTURE_VITALS, version, 10).await));
            assert!(unpinned(
                f.translate(LOINC, version, "8867-4", SNOMED).await
            ));
        }
    }

    #[tokio::test]
    async fn lookup_finds_a_pinned_code_and_names_the_version() {
        let c = Fixture.lookup(LOINC, V, "8867-4").await.unwrap();
        assert_eq!(
            (c.display.as_str(), c.version.as_str()),
            ("Heart rate", FIXTURE_VERSION)
        );
        assert!(matches!(
            Fixture.lookup(LOINC, V, "0000-0").await,
            Err(TerminologyError::NotFound { .. })
        ));
        assert!(matches!(
            Fixture.lookup("urn:other", V, "x").await,
            Err(TerminologyError::UnknownSystem(_))
        ));
    }

    #[tokio::test]
    async fn validate_code_says_false_for_a_missing_code_or_a_wrong_display() {
        let f = Fixture;
        assert!(
            f.validate_code(LOINC, V, "8867-4", None)
                .await
                .unwrap()
                .result
        );
        assert!(
            f.validate_code(LOINC, V, "8867-4", Some("Heart rate"))
                .await
                .unwrap()
                .result
        );
        let wrong = f
            .validate_code(LOINC, V, "8867-4", Some("Pulse"))
            .await
            .unwrap();
        assert!(!wrong.result && wrong.message.is_some());
        let missing = f.validate_code(LOINC, V, "0000-0", None).await.unwrap();
        assert!(!missing.result && missing.display.is_none());
    }

    #[tokio::test]
    async fn the_vital_sign_value_set_holds_the_supported_codes() {
        let all = Fixture.expand(FIXTURE_VITALS, V, 100).await.unwrap();
        let mut held: Vec<_> = all.iter().map(|c| c.code.as_str()).collect();
        held.sort_unstable();
        let mut supported: Vec<_> = crate::observations::SUPPORTED
            .iter()
            .map(|s| s.loinc)
            .collect();
        supported.sort_unstable();
        assert_eq!(held, supported);
        assert_eq!(Fixture.expand(FIXTURE_VITALS, V, 3).await.unwrap().len(), 3);
        assert!(matches!(
            Fixture.expand("urn:nope", V, 3).await,
            Err(TerminologyError::UnknownValueSet(_))
        ));
    }

    #[tokio::test]
    async fn translate_returns_a_related_target_or_nothing_never_an_invented_one() {
        let m = Fixture.translate(LOINC, V, "8867-4", SNOMED).await.unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].relationship, "relatedto");
        assert_eq!(m[0].concept.code, "364075005");
        assert!(
            Fixture
                .translate(LOINC, V, "8310-5", SNOMED)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn the_pinned_versions_are_listed() {
        assert_eq!(Fixture.pinned().len(), 2);
    }
}
