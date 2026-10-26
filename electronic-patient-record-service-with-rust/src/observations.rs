//! Vitals and observations (RC-6), first slice: the rules, pure. Nothing calls this yet, so **nothing a
//! clinician sees has changed**: there is no store, route or screen for an observation.
//!
//! An [`Observation`] is one measured value for one LOINC code, with its UCUM unit and, if the sender gave
//! them, a reference range and a critical range. [`validate`] says what is wrong with one before it is
//! stored, as a stable code plus a sentence; [`flag`] says whether the value is low, high or critical.
//!
//! The rules, each written down because each could mislead a clinician if it went wrong:
//!
//! - **Only the standard vital signs are accepted** ([`SUPPORTED`]). Any other code is refused as
//!   unsupported, never stored on a guess.
//! - **One unit per code, no conversion.** A unit other than the code's own is refused (`unit_mismatch`):
//!   converting °F to °C or lb to kg here would be a second place for an error to hide.
//! - **Absent evidence is `null` with a reason, never zero.** No value needs an [`AbsentReason`]; a value
//!   and a reason together are refused; zero is a value like any other and must pass the plausibility bounds.
//! - **Implausible values are refused**, not flagged (a systolic pressure of 3 is a slip of the keys). The
//!   bounds in [`SUPPORTED`] are wide guesses at what is physically possible, **not clinical limits**.
//! - **The range is the sender's.** No table of normal values is held, so with no range the flag is `None`
//!   ("not assessed"), never "normal". A value on the edge of a range is normal; only strictly outside is
//!   low or high, and strictly outside the critical range is critical.

use serde::{Deserialize, Serialize};

/// A supported vital sign.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Supported {
    /// LOINC code.
    pub loinc: &'static str,
    /// The one UCUM unit accepted.
    pub unit: &'static str,
    /// Smallest value accepted (a guess at what is physically possible).
    pub min: f64,
    /// Largest value accepted (as above).
    pub max: f64,
}

/// The vital signs this accepts. Bounds are guesses, not clinical limits, and nobody has reviewed them.
pub const SUPPORTED: &[Supported] = &[
    Supported {
        loinc: "8480-6",
        unit: "mm[Hg]",
        min: 30.0,
        max: 350.0,
    }, // systolic blood pressure
    Supported {
        loinc: "8462-4",
        unit: "mm[Hg]",
        min: 10.0,
        max: 250.0,
    }, // diastolic blood pressure
    Supported {
        loinc: "8867-4",
        unit: "/min",
        min: 10.0,
        max: 350.0,
    }, // heart rate
    Supported {
        loinc: "9279-1",
        unit: "/min",
        min: 1.0,
        max: 120.0,
    }, // respiratory rate
    Supported {
        loinc: "8310-5",
        unit: "Cel",
        min: 20.0,
        max: 50.0,
    }, // body temperature
    Supported {
        loinc: "59408-5",
        unit: "%",
        min: 1.0,
        max: 100.0,
    }, // oxygen saturation, pulse oximetry
    Supported {
        loinc: "29463-7",
        unit: "kg",
        min: 0.2,
        max: 700.0,
    }, // body weight
    Supported {
        loinc: "8302-2",
        unit: "cm",
        min: 20.0,
        max: 280.0,
    }, // body height
];

/// The entry for a LOINC code, if supported.
#[must_use]
pub fn supported(loinc: &str) -> Option<&'static Supported> {
    SUPPORTED.iter().find(|s| s.loinc == loinc)
}

/// Why there is no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AbsentReason {
    /// Not measured.
    NotDone,
    /// The patient declined.
    Declined,
    /// The attempt failed (a sensor off the finger, a cuff that would not hold).
    Failed,
    /// The result is not known.
    Unknown,
}

/// A range of values. Either end may be absent; an absent end is unbounded.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range {
    /// Lower end, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low: Option<f64>,
    /// Upper end, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high: Option<f64>,
}

/// One observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    /// LOINC code.
    pub code: String,
    /// The measured value, or `None` with an `absent_reason`. Never zero for "none".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// UCUM unit; required with a value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Why there is no value; required without one, refused with one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absent_reason: Option<AbsentReason>,
    /// The reference range the sender gave.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    /// The critical range the sender gave: outside it is critical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critical: Option<Range>,
}

/// Where a value sits against its ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Flag {
    /// Below the critical range.
    CriticalLow,
    /// Below the reference range.
    Low,
    /// Within the reference range, ends included.
    Normal,
    /// Above the reference range.
    High,
    /// Above the critical range.
    CriticalHigh,
}

fn finite(x: Option<f64>) -> bool {
    x.is_none_or(f64::is_finite)
}

fn range_problem(name: &str, r: &Range) -> Option<String> {
    if !finite(r.low) || !finite(r.high) {
        return Some(format!("the {name} range has a number that is not finite"));
    }
    match (r.low, r.high) {
        (Some(l), Some(h)) if l > h => Some(format!(
            "the {name} range has its low end above its high end"
        )),
        _ => None,
    }
}

/// What is wrong with an observation, each as a stable code and a sentence, in a fixed order; empty if
/// nothing. Codes: `code_unsupported`, `value_missing`, `value_and_reason`, `value_not_finite`,
/// `unit_missing`, `unit_mismatch`, `value_implausible`, `range_invalid`, `critical_range_invalid`,
/// `critical_inside_range`.
#[must_use]
pub fn validate(o: &Observation) -> Vec<(&'static str, String)> {
    let mut problems = Vec::new();
    let spec = supported(&o.code);
    if spec.is_none() {
        problems.push((
            "code_unsupported",
            format!(
                "LOINC code {:?} is not one of the supported vital signs",
                o.code
            ),
        ));
    }
    match (o.value, o.absent_reason) {
        (None, None) => problems.push((
            "value_missing",
            "there is no value and no reason for none".to_string(),
        )),
        (Some(_), Some(_)) => problems.push((
            "value_and_reason",
            "a value and a reason for having none cannot both be given".to_string(),
        )),
        _ => {}
    }
    if let Some(v) = o.value {
        if !v.is_finite() {
            problems.push((
                "value_not_finite",
                "the value is not a finite number".to_string(),
            ));
        }
        match (&o.unit, spec) {
            (None, _) => problems.push(("unit_missing", "a value needs a unit".to_string())),
            (Some(u), Some(s)) if u != s.unit => problems.push((
                "unit_mismatch",
                format!(
                    "the unit {u:?} does not fit this code, which takes {:?}",
                    s.unit
                ),
            )),
            _ => {}
        }
        // The bounds are in the code's own unit, so they say nothing about a value in another one.
        if let Some(s) = spec
            && v.is_finite()
            && o.unit.as_deref() == Some(s.unit)
            && (v < s.min || v > s.max)
        {
            problems.push((
                "value_implausible",
                format!(
                    "{v} is outside what is possible for this code ({} to {})",
                    s.min, s.max
                ),
            ));
        }
    }
    if let Some(m) = o.range.as_ref().and_then(|r| range_problem("reference", r)) {
        problems.push(("range_invalid", m));
    }
    if let Some(m) = o
        .critical
        .as_ref()
        .and_then(|r| range_problem("critical", r))
    {
        problems.push(("critical_range_invalid", m));
    }
    if let (Some(r), Some(c)) = (&o.range, &o.critical)
        && range_problem("reference", r).is_none()
        && range_problem("critical", c).is_none()
    {
        let low_inside = matches!((r.low, c.low), (Some(rl), Some(cl)) if cl > rl);
        let high_inside = matches!((r.high, c.high), (Some(rh), Some(ch)) if ch < rh);
        if low_inside || high_inside {
            problems.push((
                "critical_inside_range",
                "the critical range must lie outside the reference range".to_string(),
            ));
        }
    }
    problems
}

/// Where the value sits, or `None` when it cannot be said: no value, a value that is not finite, no
/// reference range, or a range that [`validate`] would refuse. `None` means "not assessed", never normal.
#[must_use]
pub fn flag(o: &Observation) -> Option<Flag> {
    let v = o.value.filter(|v| v.is_finite())?;
    let r = o.range.as_ref()?;
    if range_problem("reference", r).is_some() {
        return None;
    }
    if let Some(c) = &o.critical {
        if range_problem("critical", c).is_some() {
            return None;
        }
        if c.low.is_some_and(|l| v < l) {
            return Some(Flag::CriticalLow);
        }
        if c.high.is_some_and(|h| v > h) {
            return Some(Flag::CriticalHigh);
        }
    }
    if r.low.is_some_and(|l| v < l) {
        return Some(Flag::Low);
    }
    if r.high.is_some_and(|h| v > h) {
        return Some(Flag::High);
    }
    Some(Flag::Normal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(code: &str, value: f64, unit: &str) -> Observation {
        Observation {
            code: code.into(),
            value: Some(value),
            unit: Some(unit.into()),
            absent_reason: None,
            range: None,
            critical: None,
        }
    }
    fn heart(value: f64) -> Observation {
        obs("8867-4", value, "/min")
    }
    fn with_ranges(mut o: Observation) -> Observation {
        o.range = Some(Range {
            low: Some(60.0),
            high: Some(100.0),
        });
        o.critical = Some(Range {
            low: Some(40.0),
            high: Some(140.0),
        });
        o
    }
    fn codes(o: &Observation) -> Vec<&'static str> {
        validate(o).into_iter().map(|(c, _)| c).collect()
    }

    #[test]
    fn every_supported_code_accepts_a_value_at_each_bound_and_refuses_just_outside() {
        for s in SUPPORTED {
            assert!(
                validate(&obs(s.loinc, s.min, s.unit)).is_empty(),
                "{} min",
                s.loinc
            );
            assert!(
                validate(&obs(s.loinc, s.max, s.unit)).is_empty(),
                "{} max",
                s.loinc
            );
            assert_eq!(
                codes(&obs(s.loinc, s.min - 0.001, s.unit)),
                ["value_implausible"],
                "{}",
                s.loinc
            );
            assert_eq!(
                codes(&obs(s.loinc, s.max + 0.001, s.unit)),
                ["value_implausible"],
                "{}",
                s.loinc
            );
        }
    }

    #[test]
    fn zero_is_a_value_not_an_absence_and_is_refused_for_every_vital() {
        for s in SUPPORTED {
            assert_eq!(
                codes(&obs(s.loinc, 0.0, s.unit)),
                ["value_implausible"],
                "{}",
                s.loinc
            );
        }
    }

    #[test]
    fn the_codes_are_the_standard_vitals_and_each_has_a_unique_loinc() {
        let mut seen: Vec<&str> = SUPPORTED.iter().map(|s| s.loinc).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), SUPPORTED.len());
        assert_eq!(SUPPORTED.len(), 8);
        assert_eq!(supported("8310-5").map(|s| s.unit), Some("Cel"));
        assert!(supported("2160-0").is_none());
    }

    #[test]
    fn an_unsupported_code_is_refused() {
        assert_eq!(codes(&obs("2160-0", 1.0, "mg/dL")), ["code_unsupported"]);
        assert_eq!(codes(&obs("", 1.0, "/min")), ["code_unsupported"]);
    }

    #[test]
    fn a_unit_that_does_not_fit_the_code_is_refused_and_never_converted() {
        // 98.6 is a fine Fahrenheit temperature: it is refused for its unit, not called implausible.
        assert_eq!(codes(&obs("8310-5", 98.6, "[degF]")), ["unit_mismatch"]);
        assert_eq!(codes(&obs("29463-7", 70.0, "[lb_av]")), ["unit_mismatch"]);
        assert_eq!(codes(&obs("8867-4", 70.0, "mm[Hg]")), ["unit_mismatch"]);
        assert_eq!(codes(&obs("8867-4", 70.0, "")), ["unit_mismatch"]);
        assert_eq!(codes(&obs("8867-4", 70.0, "/MIN")), ["unit_mismatch"]);
    }

    #[test]
    fn a_value_needs_a_unit() {
        let mut o = heart(70.0);
        o.unit = None;
        assert_eq!(codes(&o), ["unit_missing"]);
    }

    #[test]
    fn no_value_needs_a_reason_and_a_value_refuses_one() {
        let mut none = heart(70.0);
        none.value = None;
        none.unit = None;
        assert_eq!(codes(&none), ["value_missing"]);
        for reason in [
            AbsentReason::NotDone,
            AbsentReason::Declined,
            AbsentReason::Failed,
            AbsentReason::Unknown,
        ] {
            none.absent_reason = Some(reason);
            assert!(validate(&none).is_empty(), "{reason:?}");
        }
        let mut both = heart(70.0);
        both.absent_reason = Some(AbsentReason::Failed);
        assert_eq!(codes(&both), ["value_and_reason"]);
    }

    #[test]
    fn a_value_that_is_not_a_finite_number_is_refused() {
        for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(codes(&heart(v)), ["value_not_finite"], "{v}");
        }
    }

    #[test]
    fn several_problems_come_together_in_a_fixed_order() {
        let o = Observation {
            code: "nope".into(),
            value: Some(1.0),
            unit: None,
            absent_reason: Some(AbsentReason::Failed),
            range: Some(Range {
                low: Some(2.0),
                high: Some(1.0),
            }),
            critical: None,
        };
        assert_eq!(
            codes(&o),
            [
                "code_unsupported",
                "value_and_reason",
                "unit_missing",
                "range_invalid"
            ]
        );
    }

    #[test]
    fn a_range_must_be_finite_and_ordered_and_the_critical_range_must_lie_outside_it() {
        let mut o = heart(70.0);
        o.range = Some(Range {
            low: Some(100.0),
            high: Some(60.0),
        });
        assert_eq!(codes(&o), ["range_invalid"]);
        o.range = Some(Range {
            low: Some(f64::NAN),
            high: None,
        });
        assert_eq!(codes(&o), ["range_invalid"]);
        o.range = Some(Range {
            low: Some(60.0),
            high: Some(60.0),
        });
        assert!(validate(&o).is_empty(), "a one-point range is allowed");
        o.range = Some(Range {
            low: Some(60.0),
            high: Some(100.0),
        });
        o.critical = Some(Range {
            low: Some(70.0),
            high: Some(140.0),
        });
        assert_eq!(codes(&o), ["critical_inside_range"]);
        o.critical = Some(Range {
            low: Some(40.0),
            high: Some(90.0),
        });
        assert_eq!(codes(&o), ["critical_inside_range"]);
        o.critical = Some(Range {
            low: Some(60.0),
            high: Some(100.0),
        });
        assert!(
            validate(&o).is_empty(),
            "a critical range equal to the reference range is allowed"
        );
        o.critical = Some(Range {
            low: Some(140.0),
            high: Some(40.0),
        });
        assert_eq!(codes(&o), ["critical_range_invalid"]);
    }

    #[test]
    fn the_flag_moves_exactly_at_each_boundary() {
        let f = |v: f64| flag(&with_ranges(heart(v)));
        assert_eq!(f(39.9), Some(Flag::CriticalLow));
        assert_eq!(
            f(40.0),
            Some(Flag::Low),
            "the critical end itself is not critical"
        );
        assert_eq!(f(59.9), Some(Flag::Low));
        assert_eq!(f(60.0), Some(Flag::Normal), "the low end is normal");
        assert_eq!(f(80.0), Some(Flag::Normal));
        assert_eq!(f(100.0), Some(Flag::Normal), "the high end is normal");
        assert_eq!(f(100.1), Some(Flag::High));
        assert_eq!(
            f(140.0),
            Some(Flag::High),
            "the critical end itself is not critical"
        );
        assert_eq!(f(140.1), Some(Flag::CriticalHigh));
    }

    #[test]
    fn a_half_open_range_flags_only_the_side_it_has() {
        let mut o = heart(200.0);
        o.range = Some(Range {
            low: None,
            high: Some(100.0),
        });
        assert_eq!(flag(&o), Some(Flag::High));
        o.value = Some(20.0);
        assert_eq!(flag(&o), Some(Flag::Normal));
        o.range = Some(Range {
            low: Some(60.0),
            high: None,
        });
        assert_eq!(flag(&o), Some(Flag::Low));
        o.value = Some(500.0);
        assert_eq!(flag(&o), Some(Flag::Normal));
    }

    #[test]
    fn with_no_range_no_value_or_a_broken_range_the_flag_is_none_never_normal() {
        assert_eq!(flag(&heart(70.0)), None);
        let mut none = with_ranges(heart(70.0));
        none.value = None;
        none.absent_reason = Some(AbsentReason::Failed);
        assert_eq!(flag(&none), None);
        assert_eq!(flag(&with_ranges(heart(f64::NAN))), None);
        let mut bad = heart(70.0);
        bad.range = Some(Range {
            low: Some(100.0),
            high: Some(60.0),
        });
        assert_eq!(flag(&bad), None);
        let mut bad_critical = with_ranges(heart(70.0));
        bad_critical.critical = Some(Range {
            low: Some(f64::NAN),
            high: None,
        });
        assert_eq!(flag(&bad_critical), None);
    }

    #[test]
    fn a_critical_range_without_a_reference_range_flags_nothing() {
        let mut o = heart(10.0);
        o.critical = Some(Range {
            low: Some(40.0),
            high: Some(140.0),
        });
        assert_eq!(flag(&o), None);
    }

    #[test]
    fn it_round_trips_through_json_with_absent_fields_left_out() {
        let o = heart(70.0);
        let text = serde_json::to_string(&o).unwrap();
        assert_eq!(text, r#"{"code":"8867-4","value":70.0,"unit":"/min"}"#);
        assert_eq!(serde_json::from_str::<Observation>(&text).unwrap(), o);
        let none: Observation =
            serde_json::from_str(r#"{"code":"8867-4","absent_reason":"declined"}"#).unwrap();
        assert_eq!(none.value, None);
        assert_eq!(none.absent_reason, Some(AbsentReason::Declined));
    }
}
