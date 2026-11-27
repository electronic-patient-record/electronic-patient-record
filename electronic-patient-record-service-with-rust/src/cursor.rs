//! Signed cursor pages for lists that a record policy filters row by row (`axa::cursor_pagination` 0.1.4).
//!
//! A list whose rows the caller may or may not see, one by one, must not fetch one page and drop what the caller may not
//! see: the page comes back short, which says that rows were hidden, and how many. [`axa::cursor_pagination::CursorPage::fill`] reads on in
//! batches until the page is full, up to a read budget, and the next page starts right after the last row it returned.
//! Patient search is such a list: a patient's refusal of disclosure hides them from some callers.
//!
//! The key is `EPR_CURSOR_KEY`, at least 32 bytes as hex. Without it a random key is made for the process, so a cursor
//! does not outlive a restart and is refused (400) by another replica, which is safe, and the start says nothing. A
//! cursor holds only for the caller and purpose of use it was made for ([`context`]).

use axa::cursor_pagination::{CursorKey, CursorLimits, MIN_KEY_LEN};
use std::sync::LazyLock;

/// The setting's name.
pub const SETTING: &str = "EPR_CURSOR_KEY";

/// Patient search: 50 a page, as before, and no more.
pub const SEARCH_LIMITS: CursorLimits = CursorLimits {
    default_limit: 50,
    max_limit: 50,
};

/// The most rows one search request reads to fill its page: ten pages. A caller who may see almost nothing cannot make
/// every request read the whole table; a page that spent the budget may come back short with a next link.
pub const SEARCH_MAX_SCAN: u64 = 500;

fn from_hex(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| {
            text.get(i..i + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
        })
        .collect()
}

/// The key in `text` (hex), or why not.
///
/// # Errors
///
/// Not hex, or shorter than [`MIN_KEY_LEN`] bytes.
pub fn key_from(text: &str) -> Result<CursorKey, String> {
    let bytes = from_hex(text).ok_or_else(|| format!("{SETTING} must be hex"))?;
    CursorKey::new(&bytes)
        .map_err(|e| format!("{SETTING}: {e}; give at least {MIN_KEY_LEN} bytes as hex"))
}

/// A key for this process alone, from two random UUIDs (32 bytes, 244 random bits).
static PROCESS_KEY: LazyLock<CursorKey> = LazyLock::new(|| {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    #[allow(clippy::expect_used)]
    CursorKey::new(&bytes).expect("32 bytes is a cursor key")
});

/// The key in force: `EPR_CURSOR_KEY`, or the process's own.
///
/// # Errors
///
/// As [`key_from`], for a setting that is set and wrong (the start refuses it first).
pub fn key() -> Result<CursorKey, String> {
    match crate::runtime::setting(SETTING) {
        Some(text) => key_from(&text),
        None => Ok(PROCESS_KEY.clone()),
    }
}

/// Check the key at start: one that is set and wrong stops the service.
///
/// # Errors
///
/// As [`key_from`].
pub fn check_settings() -> Result<(), String> {
    key().map(|_| ())
}

/// What a cursor is bound to: the caller and the purpose of use, so that a cursor carried to another caller, who may see
/// other rows, is refused.
#[must_use]
pub fn context(subject: &str, purpose: crate::policy::Purpose) -> String {
    format!("{subject}|{purpose:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_hex_of_at_least_32_bytes() {
        assert!(key_from(&"ab".repeat(32)).is_ok());
        assert!(key_from(&"AB".repeat(40)).is_ok());
        assert!(
            key_from(&"ab".repeat(31))
                .unwrap_err()
                .contains("at least 32")
        );
        assert!(
            key_from("not hex at all, not at all, no")
                .unwrap_err()
                .contains("hex")
        );
        assert!(key_from("abc").unwrap_err().contains("hex"));
    }

    #[test]
    fn a_cursor_holds_only_for_its_caller_and_purpose() {
        use crate::policy::Purpose;
        assert_ne!(
            context("a", Purpose::Treatment),
            context("b", Purpose::Treatment)
        );
        assert_ne!(
            context("a", Purpose::Treatment),
            context("a", Purpose::Operations)
        );
    }
}
