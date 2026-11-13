//! Permission at the point of care (WL-4): the basis on which a member of staff opens a patient's
//! clinical record, said on the request and kept on every audit row of it.
//!
//! The Welsh GP Record asks the patient's permission at each consultation, "unless the clinician is
//! unable to ask you and there is a clinical reason" (`spec/wales/index.md`). Here the caller says
//! which in the `X-Permission-Basis` header:
//!
//! - `asked-agreed`: the patient was asked and agreed;
//! - `could-not-ask:<reason>`: they could not be asked, for a coded clinical reason (`unconscious`,
//!   `lacks-capacity`, `too-unwell`, `emergency`). The reason is a code, never free text, so nothing a
//!   person typed reaches the audit trail.
//!
//! An unknown basis is a 400. When `EPR_REQUIRE_PERMISSION_BASIS` is on, a clinical read of one
//! patient by anyone but the patient or their proxy needs a basis, or it is refused and the refusal
//! audited. It is off by default: a basis is then kept when given and never asked for. Absence is
//! never agreement: a row with no basis says nothing about permission.

use std::sync::OnceLock;

/// The request header.
pub const HEADER: &str = "x-permission-basis";

/// The setting that makes a basis required.
pub const SETTING: &str = "EPR_REQUIRE_PERMISSION_BASIS";

/// Why a patient could not be asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Not conscious.
    Unconscious,
    /// Lacks capacity to decide this.
    LacksCapacity,
    /// Too unwell to be asked now.
    TooUnwell,
    /// An emergency leaves no time to ask.
    Emergency,
}

impl Reason {
    /// Every reason.
    pub const ALL: [Reason; 4] = [
        Reason::Unconscious,
        Reason::LacksCapacity,
        Reason::TooUnwell,
        Reason::Emergency,
    ];

    /// The code.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Reason::Unconscious => "unconscious",
            Reason::LacksCapacity => "lacks-capacity",
            Reason::TooUnwell => "too-unwell",
            Reason::Emergency => "emergency",
        }
    }
}

/// The basis for an access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// The patient was asked and agreed.
    AskedAgreed,
    /// The patient could not be asked, for this reason.
    CouldNotAsk(Reason),
}

impl Basis {
    /// Read a header value. `None` for anything that is not exactly a basis.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Basis> {
        let raw = raw.trim();
        if raw == "asked-agreed" {
            return Some(Basis::AskedAgreed);
        }
        let reason = raw.strip_prefix("could-not-ask:")?;
        Reason::ALL
            .into_iter()
            .find(|r| r.code() == reason)
            .map(Basis::CouldNotAsk)
    }

    /// The word kept in the audit row.
    #[must_use]
    pub fn word(self) -> String {
        match self {
            Basis::AskedAgreed => "asked-agreed".into(),
            Basis::CouldNotAsk(r) => format!("could-not-ask:{}", r.code()),
        }
    }
}

/// Read the setting's value: unset, `false` or `0` is off; `true` or `1` is on.
///
/// # Errors
///
/// Anything else, so a typo stops the service starting rather than silently turning it off.
pub fn parse_required(raw: Option<&str>) -> Result<bool, String> {
    match raw.map(str::trim) {
        None | Some("" | "false" | "0") => Ok(false),
        Some("true" | "1") => Ok(true),
        Some(other) => Err(format!("{SETTING} must be true or false, not {other:?}")),
    }
}

static REQUIRED: OnceLock<bool> = OnceLock::new();

/// Read [`SETTING`] once, at start-up.
///
/// # Errors
///
/// As [`parse_required`].
pub fn init() -> Result<bool, String> {
    let on = parse_required(std::env::var(SETTING).ok().as_deref())?;
    let _ = REQUIRED.set(on);
    Ok(on)
}

/// Whether a basis is required. Before [`init`] (tests that build no app), the environment is read.
#[must_use]
pub fn required() -> bool {
    *REQUIRED.get_or_init(|| parse_required(std::env::var(SETTING).ok().as_deref()).unwrap_or(true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_basis_is_exactly_one_of_the_words() {
        assert_eq!(Basis::parse("asked-agreed"), Some(Basis::AskedAgreed));
        assert_eq!(
            Basis::parse(" could-not-ask:unconscious "),
            Some(Basis::CouldNotAsk(Reason::Unconscious))
        );
        for r in Reason::ALL {
            let b = Basis::CouldNotAsk(r);
            assert_eq!(Basis::parse(&b.word()), Some(b));
        }
        for bad in [
            "",
            "yes",
            "could-not-ask:",
            "could-not-ask:he was asleep",
            "Asked-Agreed",
        ] {
            assert_eq!(Basis::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn the_setting_is_on_off_or_refused() {
        assert_eq!(parse_required(None), Ok(false));
        assert_eq!(parse_required(Some("false")), Ok(false));
        assert_eq!(parse_required(Some("1")), Ok(true));
        assert!(parse_required(Some("yes")).is_err());
    }
}
