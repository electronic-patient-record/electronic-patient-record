//! Single-use link tokens (PA-11), pure. A link handed to a person (an invitation, a portal link)
//! carries an opaque random token; the service stores only its SHA-256 and looks a presented token
//! up by that hash, so a leaked database does not leak working links. The token holds nothing about
//! the person or the record. Idea and tests from the patient-administration-system project.
//!
//! **GET reads, POST acts.** [`Link::view`] takes `&self` and cannot change anything; only
//! [`Link::consume`] spends a link, once, and it refuses an expired or already-spent one. Time is
//! passed in, so a test needs no clock. When this is wired to a table, the spend must be one atomic
//! `UPDATE … WHERE consumed_at IS NULL AND expires_at > now()` so two callers cannot both win; this
//! module cannot prove that, and nothing in the service uses it yet.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// A fresh token: two random UUIDs as 64 lowercase hex characters (244 random bits).
#[must_use]
pub fn new_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

/// The stored form of a token: SHA-256, lowercase hex.
#[must_use]
pub fn hash(token: &str) -> String {
    use std::fmt::Write;
    Sha256::digest(token.as_bytes())
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// How an action taken through a link is named in the audit log: the first 12 characters of the
/// hash, enough to find the row and useless for rebuilding the link.
#[must_use]
pub fn audit_tag(token: &str) -> String {
    let mut h = hash(token);
    h.truncate(12);
    h
}

/// Where a link stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// It can be spent.
    Usable,
    /// Its time has passed.
    Expired,
    /// It was spent.
    Used,
}

/// Why a link could not be spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// Its time has passed.
    Expired,
    /// It was spent already.
    Used,
}

/// The stored side of a link: the hash, its expiry, and whether it was spent. No token, no person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// SHA-256 of the token, lowercase hex.
    pub token_hash: String,
    /// The last instant it can be used is just before this one.
    pub expires_at: DateTime<Utc>,
    /// Set when it is spent.
    pub consumed_at: Option<DateTime<Utc>>,
}

impl Link {
    /// A new link for `token`, which the caller shows once and does not keep.
    #[must_use]
    pub fn new(token: &str, expires_at: DateTime<Utc>) -> Self {
        Self {
            token_hash: hash(token),
            expires_at,
            consumed_at: None,
        }
    }

    /// Whether `token` is the one this link was made for.
    #[must_use]
    pub fn is_for(&self, token: &str) -> bool {
        self.token_hash == hash(token)
    }

    /// Where the link stands at `now`. Read-only: looking never spends it. A spent link is `Used`
    /// even after its time has passed.
    #[must_use]
    pub fn view(&self, now: DateTime<Utc>) -> State {
        if self.consumed_at.is_some() {
            State::Used
        } else if self.expires_at <= now {
            State::Expired
        } else {
            State::Usable
        }
    }

    /// Spend the link at `now`.
    ///
    /// # Errors
    /// [`Refused::Used`] if it was spent, [`Refused::Expired`] if its time has passed.
    pub fn consume(&mut self, now: DateTime<Utc>) -> Result<(), Refused> {
        match self.view(now) {
            State::Usable => {
                self.consumed_at = Some(now);
                Ok(())
            }
            State::Expired => Err(Refused::Expired),
            State::Used => Err(Refused::Used),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 28, 9, 0, 0).unwrap()
    }

    #[test]
    fn a_token_is_64_lowercase_hex_characters_and_never_repeats() {
        let (a, b) = (new_token(), new_token());
        assert_eq!(a.len(), 64);
        assert!(
            a.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        );
        assert_ne!(a, b);
    }

    #[test]
    fn the_hash_is_sha256_hex_and_the_audit_tag_is_its_start() {
        assert_eq!(
            hash("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let t = new_token();
        assert_eq!(audit_tag(&t), hash(&t)[..12]);
    }

    #[test]
    fn the_link_stores_the_hash_and_never_the_token() {
        let token = new_token();
        let link = Link::new(&token, t0() + Duration::hours(1));
        assert!(link.is_for(&token));
        assert!(!link.is_for(&new_token()));
        assert!(!format!("{link:?}").contains(&token));
    }

    #[test]
    fn a_link_works_once() {
        let mut link = Link::new(&new_token(), t0() + Duration::hours(1));
        assert_eq!(link.consume(t0()), Ok(()));
        assert_eq!(link.consume(t0()), Err(Refused::Used));
        assert_eq!(link.view(t0()), State::Used);
    }

    #[test]
    fn an_expired_link_is_refused_at_and_after_its_time_and_not_spent() {
        let mut link = Link::new(&new_token(), t0() + Duration::hours(1));
        assert_eq!(link.view(t0() + Duration::minutes(59)), State::Usable);
        assert_eq!(
            link.consume(t0() + Duration::hours(1)),
            Err(Refused::Expired)
        );
        assert_eq!(
            link.consume(t0() + Duration::days(9)),
            Err(Refused::Expired)
        );
        assert_eq!(link.consumed_at, None);
    }

    #[test]
    fn looking_never_changes_the_link() {
        let link = Link::new(&new_token(), t0() + Duration::hours(1));
        let before = link.clone();
        for minutes in [0, 30, 59, 61, 600] {
            let _ = link.view(t0() + Duration::minutes(minutes));
        }
        assert_eq!(link, before);
    }

    #[test]
    fn a_spent_link_stays_used_after_its_time_passes() {
        let mut link = Link::new(&new_token(), t0() + Duration::hours(1));
        link.consume(t0()).unwrap();
        assert_eq!(link.view(t0() + Duration::days(1)), State::Used);
    }
}
