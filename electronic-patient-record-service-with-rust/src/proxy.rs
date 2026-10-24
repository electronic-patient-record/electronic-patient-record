//! Proxy authority (PA-5): the rule that decides whether a proxy may use one part of a patient's record at a
//! given time. It is pure: the caller passes the authority and the time (Unix seconds), and gets back one
//! [`Decision`].
//!
//! The terms are stored inside a `proxy` care relationship (`relationships.rs`): the relationship's version
//! history supplies the recorder and the time, and ending the relationship is the revocation. The decision
//! applies to every read a proxy makes (`controllers::common::authorize`, the photo and access-log routes).
//!
//! An authority names the **patient**, the **proxy**, the **basis** the administrator relied on, the **scopes**
//! the proxy may use, the **person who recorded it**, and an **expiry**. It can be **revoked**. The decision is
//! the narrowest the facts allow, checked in this order, so the reason shown is the strongest one:
//!
//! 1. revoked at or before `now`: [`Refusal::Revoked`]
//! 2. `now` at or after the expiry: [`Refusal::Expired`]
//! 3. the scope is not one the authority lists: [`Refusal::OutOfScope`]
//!
//! An authority with no scope allows nothing, and there is no "all" scope: a wider grant lists every scope.
//! An expiry is required, never "forever". Recording one is a separate, audited act this module does not
//! perform; [`validate`] says what is wrong with a record before it is stored. **Nothing here proves** the
//! basis is true: that is the administrator's check, outside this service.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A part of the record a proxy may be given. A guess at the first four; the list grows with the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    /// Name, birth date, sex, identifiers and address.
    Identity,
    /// The allergy list and the no-known-allergies statement.
    Allergies,
    /// The patient photo.
    Photo,
    /// Who has accessed the record.
    AccessLog,
}

/// What the administrator relied on to record the authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Basis {
    /// Parental responsibility for a child.
    ParentalResponsibility,
    /// A registered power of attorney for health and welfare.
    PowerOfAttorney,
    /// A court-appointed deputy.
    Deputyship,
    /// The patient asked for it, and the administrator saw it.
    PatientRequest,
}

/// What a proxy relationship carries: the part of an [`Authority`] the relationship record does not already hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Terms {
    /// What the administrator relied on.
    pub basis: Basis,
    /// What the proxy may use.
    pub scopes: BTreeSet<Scope>,
    /// When it ends (Unix seconds).
    pub expires_at: i64,
}

/// One proxy's authority over one patient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authority {
    /// The patient's record id.
    pub patient: String,
    /// The proxy's token `sub`.
    pub proxy: String,
    /// What the administrator relied on.
    pub basis: Basis,
    /// What the proxy may use. Empty allows nothing.
    pub scopes: BTreeSet<Scope>,
    /// The token `sub` of the person who recorded it.
    pub recorded_by: String,
    /// When it was recorded.
    pub recorded_at: i64,
    /// When it ends. Required.
    pub expires_at: i64,
    /// When it was revoked, if it was.
    pub revoked_at: Option<i64>,
}

/// Why a use is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Revoked.
    Revoked,
    /// Past its expiry.
    Expired,
    /// Not a scope the authority lists.
    OutOfScope,
}

/// The answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The proxy may use this part of the record.
    Allowed,
    /// The proxy may not, and why.
    Refused(Refusal),
}

/// The longest term an authority may be recorded for: five years in seconds. A guess; the maintainer and
/// the Clinical Safety Officer set the real limit (it is long enough for a child's proxy to be renewed).
pub const MAX_TERM_SECS: i64 = 5 * 366 * 86_400;

/// May the proxy use `scope` at `now`?
#[must_use]
pub fn decide(a: &Authority, scope: Scope, now: i64) -> Decision {
    let terms = Terms {
        basis: a.basis,
        scopes: a.scopes.clone(),
        expires_at: a.expires_at,
    };
    decide_terms(&terms, a.revoked_at, scope, now)
}

/// The same decision from the terms alone, for a caller that holds the relationship record.
#[must_use]
pub fn decide_terms(t: &Terms, revoked_at: Option<i64>, scope: Scope, now: i64) -> Decision {
    if revoked_at.is_some_and(|r| r <= now) {
        return Decision::Refused(Refusal::Revoked);
    }
    if now >= t.expires_at {
        return Decision::Refused(Refusal::Expired);
    }
    if !t.scopes.contains(&scope) {
        return Decision::Refused(Refusal::OutOfScope);
    }
    Decision::Allowed
}

/// Every problem with a record that is about to be stored.
#[must_use]
pub fn validate(a: &Authority) -> Vec<String> {
    let mut problems = Vec::new();
    for (what, v) in [
        ("patient", &a.patient),
        ("proxy", &a.proxy),
        ("recorded_by", &a.recorded_by),
    ] {
        if v.trim().is_empty() || v.len() > 200 {
            problems.push(format!("{what} must be 1 to 200 characters"));
        }
    }
    if a.proxy == a.recorded_by {
        problems.push("a proxy cannot record their own authority".to_string());
    }
    if a.scopes.is_empty() {
        problems.push("at least one scope is required".to_string());
    }
    if a.expires_at <= a.recorded_at {
        problems.push("the expiry must be after the time it is recorded".to_string());
    } else if a.expires_at - a.recorded_at > MAX_TERM_SECS {
        problems.push("the expiry is further away than the longest term allowed".to_string());
    }
    if a.revoked_at.is_some_and(|r| r < a.recorded_at) {
        problems.push("it cannot be revoked before it was recorded".to_string());
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_800_000_000;
    const DAY: i64 = 86_400;

    fn authority() -> Authority {
        Authority {
            patient: "p1".into(),
            proxy: "mum".into(),
            basis: Basis::ParentalResponsibility,
            scopes: BTreeSet::from([Scope::Identity, Scope::Allergies]),
            recorded_by: "admin".into(),
            recorded_at: T0,
            expires_at: T0 + 30 * DAY,
            revoked_at: None,
        }
    }

    #[test]
    fn a_listed_scope_inside_the_term_is_allowed() {
        assert_eq!(
            decide(&authority(), Scope::Allergies, T0 + DAY),
            Decision::Allowed
        );
    }

    #[test]
    fn a_proxy_cannot_use_a_scope_the_authority_does_not_list() {
        for s in [Scope::Photo, Scope::AccessLog] {
            assert_eq!(
                decide(&authority(), s, T0 + DAY),
                Decision::Refused(Refusal::OutOfScope)
            );
        }
    }

    #[test]
    fn the_expiry_instant_is_already_expired() {
        let a = authority();
        assert_eq!(
            decide(&a, Scope::Identity, a.expires_at - 1),
            Decision::Allowed
        );
        assert_eq!(
            decide(&a, Scope::Identity, a.expires_at),
            Decision::Refused(Refusal::Expired)
        );
        assert_eq!(
            decide(&a, Scope::Identity, a.expires_at + DAY),
            Decision::Refused(Refusal::Expired)
        );
    }

    #[test]
    fn revocation_takes_effect_at_its_instant_and_is_the_reason_shown() {
        let a = Authority {
            revoked_at: Some(T0 + DAY),
            ..authority()
        };
        assert_eq!(decide(&a, Scope::Identity, T0 + DAY - 1), Decision::Allowed);
        for now in [T0 + DAY, T0 + 60 * DAY] {
            assert_eq!(
                decide(&a, Scope::Identity, now),
                Decision::Refused(Refusal::Revoked),
                "revoked is named even when it has also expired"
            );
        }
        assert_eq!(
            decide(&a, Scope::Photo, T0 + DAY),
            Decision::Refused(Refusal::Revoked)
        );
    }

    #[test]
    fn no_scope_allows_nothing() {
        let a = Authority {
            scopes: BTreeSet::new(),
            ..authority()
        };
        for s in [
            Scope::Identity,
            Scope::Allergies,
            Scope::Photo,
            Scope::AccessLog,
        ] {
            assert_eq!(
                decide(&a, s, T0 + DAY),
                Decision::Refused(Refusal::OutOfScope)
            );
        }
    }

    #[test]
    fn a_good_record_has_no_problems() {
        assert!(validate(&authority()).is_empty());
    }

    #[test]
    fn a_record_that_would_grant_too_much_or_too_little_is_refused() {
        let bad = |f: fn(&mut Authority), needle: &str| {
            let mut a = authority();
            f(&mut a);
            let p = validate(&a);
            assert!(p.iter().any(|m| m.contains(needle)), "{needle}: {p:?}");
        };
        bad(|a| a.proxy = " ".into(), "proxy must be");
        bad(|a| a.patient = String::new(), "patient must be");
        bad(|a| a.recorded_by = "x".repeat(201), "recorded_by must be");
        bad(|a| a.recorded_by = a.proxy.clone(), "their own authority");
        bad(|a| a.scopes.clear(), "at least one scope");
        bad(|a| a.expires_at = a.recorded_at, "after the time");
        bad(|a| a.expires_at = a.recorded_at - 1, "after the time");
        bad(
            |a| a.expires_at = a.recorded_at + MAX_TERM_SECS + 1,
            "longest term",
        );
        bad(
            |a| a.revoked_at = Some(a.recorded_at - 1),
            "before it was recorded",
        );
    }

    #[test]
    fn the_longest_term_itself_is_allowed() {
        let mut a = authority();
        a.expires_at = a.recorded_at + MAX_TERM_SECS;
        assert!(validate(&a).is_empty());
        a.revoked_at = Some(a.recorded_at);
        assert!(validate(&a).is_empty());
    }
}
