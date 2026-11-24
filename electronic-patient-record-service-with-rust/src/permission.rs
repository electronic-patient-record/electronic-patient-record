//! Permission at the point of care (WL-4), as one running service holds it: whether a basis is required is read from
//! that service's settings (`EPR_REQUIRE_PERMISSION_BASIS`), once at start, and kept in its [`Runtime`] (QA-16). The
//! words, headers and parsing are the domain crate's, re-exported here unchanged.

pub use electronic_patient_record_domain::permission::*;

use crate::runtime::Runtime;

/// Read [`SETTING`] from this service's settings, once, at start-up.
///
/// # Errors
///
/// As [`parse_required`].
pub fn init() -> Result<bool, String> {
    let on = parse_required(crate::runtime::setting(SETTING).as_deref())?;
    let _ = Runtime::current().permission_required.set(on);
    Ok(on)
}

/// Whether a basis is required. Before [`init`] (a test that builds no app), the settings are read, and an unusable
/// value means required: the safe answer.
#[must_use]
pub fn required() -> bool {
    *Runtime::current()
        .permission_required
        .get_or_init(|| parse_required(crate::runtime::setting(SETTING).as_deref()).unwrap_or(true))
}
