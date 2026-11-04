//! How long a referral may wait before its sender is told it has gone quiet (task PC-3, hazard H-044).
//!
//! **Assumed, awaiting the maintainer (OQ-PC3):** the periods are configuration, and there is **no default**
//! for any priority, so a deployment must choose; a priority with no period never times out and the worklist
//! says which ones those are. `EPR_REFERRAL_TIMEOUTS` is JSON:
//!
//! ```json
//! {"no_response_days": {"routine": 14, "urgent": 2, "asap": 1, "stat": 1},
//!  "no_outcome_days":  {"routine": 90, "urgent": 14}}
//! ```
//!
//! - `no_response_days`: a referral still `requested` (nobody has accepted, declined or asked anything) this
//!   many days after its current version was written.
//! - `no_outcome_days`: a referral `accepted`, `appointment-booked` or `interim-report` with no outcome this
//!   many days after its current version was written.
//!
//! Unset or blank means no timeouts at all. A value that does not parse, names an unknown key or priority, or
//! gives a period outside 1 to 365 days stops the service starting ([`init`]): a typo that quietly switched a
//! timeout off would leave an urgent referral unwatched. Not here: an appointment date that has passed (`Book`
//! carries no date), and reminders (a timeout only puts the referral on the sender's list).

use crate::referrals_store::Priority;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The longest period accepted, in days.
pub const MAX_DAYS: u32 = 365;

/// The configured periods, by priority.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Timeouts {
    /// Days a `requested` referral may wait for any answer.
    pub no_response: BTreeMap<String, u32>,
    /// Days an accepted referral may wait for its outcome.
    pub no_outcome: BTreeMap<String, u32>,
}

/// The priority words, in the order they are listed.
pub const PRIORITIES: [Priority; 4] = [
    Priority::Routine,
    Priority::Urgent,
    Priority::Asap,
    Priority::Stat,
];

/// The kebab-case word of a priority, as stored in the referral body.
#[must_use]
pub fn word(p: Priority) -> &'static str {
    match p {
        Priority::Routine => "routine",
        Priority::Urgent => "urgent",
        Priority::Asap => "asap",
        Priority::Stat => "stat",
    }
}

static CURRENT: OnceLock<Timeouts> = OnceLock::new();

impl Timeouts {
    /// Parse the setting. Blank or absent is no timeouts.
    ///
    /// # Errors
    ///
    /// A message naming the setting when the JSON is malformed, a key or priority is unknown, or a period is
    /// outside 1 to [`MAX_DAYS`].
    pub fn parse(value: Option<&str>) -> Result<Self, String> {
        let Some(text) = value.map(str::trim).filter(|v| !v.is_empty()) else {
            return Ok(Self::default());
        };
        let bad = |m: String| format!("EPR_REFERRAL_TIMEOUTS: {m}");
        let root: serde_json::Value =
            serde_json::from_str(text).map_err(|e| bad(format!("not JSON ({e})")))?;
        let object = root
            .as_object()
            .ok_or_else(|| bad("must be a JSON object".into()))?;
        let mut out = Self::default();
        for (key, value) in object {
            let target = match key.as_str() {
                "no_response_days" => &mut out.no_response,
                "no_outcome_days" => &mut out.no_outcome,
                other => return Err(bad(format!("unknown key `{other}`"))),
            };
            let periods = value
                .as_object()
                .ok_or_else(|| bad(format!("`{key}` must be an object of priority to days")))?;
            for (priority, days) in periods {
                if !PRIORITIES.iter().any(|p| word(*p) == priority) {
                    return Err(bad(format!(
                        "`{priority}` is not a priority (routine, urgent, asap, stat)"
                    )));
                }
                let days = days
                    .as_u64()
                    .and_then(|d| u32::try_from(d).ok())
                    .filter(|d| (1..=MAX_DAYS).contains(d))
                    .ok_or_else(|| {
                        bad(format!(
                            "`{key}.{priority}` must be a whole number of days from 1 to {MAX_DAYS}"
                        ))
                    })?;
                target.insert(priority.clone(), days);
            }
        }
        Ok(out)
    }

    /// The priorities that never time out for the first reason (no answer), in listing order.
    #[must_use]
    pub fn untimed_no_response(&self) -> Vec<&'static str> {
        PRIORITIES
            .iter()
            .map(|p| word(*p))
            .filter(|w| !self.no_response.contains_key(*w))
            .collect()
    }

    /// The priorities that never time out for the second reason (no outcome), in listing order.
    #[must_use]
    pub fn untimed_no_outcome(&self) -> Vec<&'static str> {
        PRIORITIES
            .iter()
            .map(|p| word(*p))
            .filter(|w| !self.no_outcome.contains_key(*w))
            .collect()
    }
}

/// Read and check `EPR_REFERRAL_TIMEOUTS` once, at start-up.
///
/// # Errors
///
/// As [`Timeouts::parse`].
pub fn init() -> Result<(), String> {
    let parsed = Timeouts::parse(std::env::var("EPR_REFERRAL_TIMEOUTS").ok().as_deref())?;
    let _ = CURRENT.set(parsed);
    Ok(())
}

/// The periods [`init`] checked, or none when it has not run (a test that does not set them).
#[must_use]
pub fn current() -> Timeouts {
    CURRENT.get().cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_or_blank_is_no_timeouts_and_every_priority_is_untimed() {
        for v in [None, Some(""), Some("  ")] {
            let t = Timeouts::parse(v).unwrap();
            assert_eq!(t, Timeouts::default());
            assert_eq!(
                t.untimed_no_response(),
                ["routine", "urgent", "asap", "stat"]
            );
        }
    }

    #[test]
    fn periods_are_read_per_priority_and_the_rest_are_listed_as_untimed() {
        let t = Timeouts::parse(Some(
            r#"{"no_response_days":{"routine":14,"urgent":2},"no_outcome_days":{"routine":90}}"#,
        ))
        .unwrap();
        assert_eq!(t.no_response["urgent"], 2);
        assert_eq!(t.untimed_no_response(), ["asap", "stat"]);
        assert_eq!(t.untimed_no_outcome(), ["urgent", "asap", "stat"]);
    }

    #[test]
    fn a_setting_that_would_quietly_switch_a_timeout_off_stops_the_service() {
        for bad in [
            "nonsense",
            "[1]",
            r#"{"no_reponse_days":{"routine":1}}"#,
            r#"{"no_response_days":{"routin":1}}"#,
            r#"{"no_response_days":{"routine":0}}"#,
            r#"{"no_response_days":{"routine":366}}"#,
            r#"{"no_response_days":{"routine":"7"}}"#,
            r#"{"no_response_days":{"routine":1.5}}"#,
            r#"{"no_response_days":[1]}"#,
        ] {
            let e = Timeouts::parse(Some(bad)).unwrap_err();
            assert!(e.starts_with("EPR_REFERRAL_TIMEOUTS"), "{bad}: {e}");
        }
    }
}
