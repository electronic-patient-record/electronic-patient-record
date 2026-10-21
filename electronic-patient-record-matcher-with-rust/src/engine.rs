//! The matching engine: a pure, total, deterministic function from two
//! patients to an explainable result. No clocks, no randomness, no IO.

use crate::nhs::{NHS_NUMBER_SYSTEM, canonical_identifier, is_valid_nhs_number};
use crate::normalize::{PartialDate, fold_name, fold_postcode, fold_telecom, parse_date};
use crate::patient::{AdministrativeSex, Patient};
use serde::{Deserialize, Serialize};

/// Field weights. They need not sum to one; the score is renormalised over
/// the fields that both records actually carry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Weights {
    pub family: f64,
    pub given: f64,
    pub birth_date: f64,
    pub sex: f64,
    pub postal_code: f64,
    pub telecom: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            family: 0.25,
            given: 0.20,
            birth_date: 0.30,
            sex: 0.05,
            postal_code: 0.10,
            telecom: 0.10,
        }
    }
}

/// Thresholds and guards.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MatchConfig {
    pub weights: Weights,
    /// At or above this (and every guard passing) the pair may be linked automatically.
    pub auto_link_threshold: f64,
    /// At or above this the pair goes to a human review queue.
    pub review_threshold: f64,
    /// Auto-link needs given-name similarity at least this high (twins guard).
    pub min_given_for_auto: f64,
    /// Auto-link needs family-name similarity at least this high.
    pub min_family_for_auto: f64,
    /// Auto-link needs birth-date similarity at least this high.
    pub min_birth_date_for_auto: f64,
    /// Fraction of total weight that must be present to go beyond Review.
    pub min_evidence_weight: f64,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            weights: Weights::default(),
            auto_link_threshold: 0.92,
            review_threshold: 0.75,
            min_given_for_auto: 0.85,
            min_family_for_auto: 0.85,
            min_birth_date_for_auto: 0.99,
            min_evidence_weight: 0.5,
        }
    }
}

/// What to do with a candidate pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    AutoLink,
    Review,
    NonMatch,
}

/// One field's contribution. `similarity` is `None` (never zero) when either
/// record lacks the field; `note` says why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldScore {
    pub field: String,
    pub similarity: Option<f64>,
    pub weight: f64,
    pub note: String,
}

/// The explainable result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchResult {
    pub score: f64,
    pub decision: Decision,
    pub fields: Vec<FieldScore>,
    /// Set when a verified national identifier decided the score.
    pub deterministic: Option<String>,
    /// Reasons a link must not be automatic. Never hidden.
    pub conflicts: Vec<String>,
}

/// The engine. Cheap to construct; holds only configuration.
#[derive(Debug, Clone, Default)]
pub struct MatchingEngine {
    config: MatchConfig,
}

impl MatchingEngine {
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &MatchConfig {
        &self.config
    }

    /// Compare two patients. Symmetric: `compare(a, b)` equals `compare(b, a)`.
    pub fn compare(&self, a: &Patient, b: &Patient) -> MatchResult {
        let w = &self.config.weights;
        let family = family_similarity(a, b);
        let given = given_similarity(a, b);
        let dob = birth_date_similarity(a, b);
        let sex = sex_similarity(a, b);
        let postal = postal_similarity(a, b);
        let telecom = telecom_similarity(a, b);

        let fields = vec![
            field(
                "family_name",
                family,
                w.family,
                "no family name on one or both records",
            ),
            field(
                "given_name",
                given,
                w.given,
                "no given name on one or both records",
            ),
            field(
                "birth_date",
                dob,
                w.birth_date,
                "no usable birth date on one or both records",
            ),
            field("sex", sex, w.sex, "sex unknown on one or both records"),
            field(
                "postal_code",
                postal,
                w.postal_code,
                "no postcode on one or both records",
            ),
            field(
                "telecom",
                telecom,
                w.telecom,
                "no phone or email on one or both records",
            ),
        ];

        let total: f64 = fields.iter().map(|f| f.weight).sum();
        let present: f64 = fields
            .iter()
            .filter(|f| f.similarity.is_some())
            .map(|f| f.weight)
            .sum();
        let weighted: f64 = fields
            .iter()
            .filter_map(|f| f.similarity.map(|s| s * f.weight))
            .sum();
        let mut score = if present > 0.0 {
            weighted / present
        } else {
            0.0
        };

        let mut conflicts = Vec::new();
        let mut deterministic = None;
        let mut decision;

        // National identifier: a shared valid NHS number decides; two different valid ones forbid a link.
        let (nhs_a, nhs_b) = (valid_nhs_numbers(a), valid_nhs_numbers(b));
        let shared = nhs_a.iter().any(|n| nhs_b.contains(n));
        let clash = !nhs_a.is_empty() && !nhs_b.is_empty() && !shared;
        let dob_conflict = matches!((full_date(a), full_date(b)), (Some(x), Some(y)) if x != y);

        if clash {
            conflicts.push("different verified NHS numbers".to_string());
            decision = Decision::NonMatch;
        } else if shared {
            deterministic = Some("shared verified NHS number".to_string());
            score = 1.0;
            if dob_conflict {
                conflicts.push("same NHS number but different birth dates".to_string());
            }
            // A family name alone may change (marriage); both names disagreeing suggests a
            // mistyped or reused number.
            let c = &self.config;
            if given.is_some_and(|s| s < c.min_given_for_auto)
                && family.is_some_and(|s| s < c.min_family_for_auto)
            {
                conflicts.push("same NHS number but different names".to_string());
            }
            decision = if conflicts.is_empty() {
                Decision::AutoLink
            } else {
                Decision::Review
            };
        } else {
            decision = if score >= self.config.auto_link_threshold {
                Decision::AutoLink
            } else if score >= self.config.review_threshold {
                Decision::Review
            } else {
                Decision::NonMatch
            };
            if decision == Decision::AutoLink {
                let c = &self.config;
                // Missing evidence blocks automatic linking; it is not treated as agreement.
                let guards = [
                    ("given name", given, c.min_given_for_auto),
                    ("family name", family, c.min_family_for_auto),
                    ("birth date", dob, c.min_birth_date_for_auto),
                ];
                for (label, sim, min) in guards {
                    match sim {
                        None => {
                            conflicts.push(format!("{label} not available to confirm the link"))
                        }
                        Some(s) if s < min => {
                            conflicts.push(format!("{label} similarity {s:.2} below {min:.2}"))
                        }
                        _ => {}
                    }
                }
                if present / total < c.min_evidence_weight {
                    conflicts.push(format!(
                        "only {:.0}% of the evidence is present",
                        100.0 * present / total
                    ));
                }
                if !conflicts.is_empty() {
                    decision = Decision::Review;
                }
            }
        }

        MatchResult {
            score,
            decision,
            fields,
            deterministic,
            conflicts,
        }
    }
}

fn field(name: &str, similarity: Option<f64>, weight: f64, absent: &str) -> FieldScore {
    let note = match similarity {
        Some(s) => format!("similarity {s:.2}"),
        None => absent.to_string(),
    };
    FieldScore {
        field: name.to_string(),
        similarity,
        weight,
        note,
    }
}

fn name_sim(x: &str, y: &str) -> f64 {
    strsim::jaro_winkler(&fold_name(x), &fold_name(y))
}

fn family_similarity(a: &Patient, b: &Patient) -> Option<f64> {
    best_over(
        a.names
            .iter()
            .filter_map(|n| n.family.as_deref())
            .filter(|s| !fold_name(s).is_empty()),
        b.names
            .iter()
            .filter_map(|n| n.family.as_deref())
            .filter(|s| !fold_name(s).is_empty()),
        name_sim,
    )
}

fn best_over<'a>(
    xs: impl Iterator<Item = &'a str>,
    ys: impl Iterator<Item = &'a str> + Clone,
    f: impl Fn(&str, &str) -> f64,
) -> Option<f64> {
    let mut best: Option<f64> = None;
    let mut any_y = false;
    for x in xs {
        for y in ys.clone() {
            any_y = true;
            let s = f(x, y);
            best = Some(best.map_or(s, |b: f64| b.max(s)));
        }
    }
    if any_y { best } else { None }
}

/// Initials match tokens that start with the same letter (scored 0.8).
fn given_token_sim(x: &str, y: &str) -> f64 {
    let (fx, fy) = (fold_name(x), fold_name(y));
    let initial = |s: &str| s.chars().count() == 1;
    if (initial(&fx) || initial(&fy)) && fx.chars().next() == fy.chars().next() {
        return 0.8;
    }
    strsim::jaro_winkler(&fx, &fy)
}

fn given_similarity(a: &Patient, b: &Patient) -> Option<f64> {
    let mut best: Option<f64> = None;
    for na in &a.names {
        for nb in &b.names {
            if na.given.is_empty() || nb.given.is_empty() {
                continue;
            }
            // Average over the shorter list; with equal lengths, average both ways and keep the
            // lower, so the order of the records never raises the score.
            let s = match na.given.len().cmp(&nb.given.len()) {
                std::cmp::Ordering::Less => given_average(&na.given, &nb.given),
                std::cmp::Ordering::Greater => given_average(&nb.given, &na.given),
                std::cmp::Ordering::Equal => {
                    given_average(&na.given, &nb.given).min(given_average(&nb.given, &na.given))
                }
            };
            best = Some(best.map_or(s, |b: f64| b.max(s)));
        }
    }
    best
}

/// Each token of `short` takes its best match in `long`; the mean of those.
fn given_average(short: &[String], long: &[String]) -> f64 {
    let total: f64 = short
        .iter()
        .map(|s| {
            long.iter()
                .map(|l| given_token_sim(s, l))
                .fold(0.0, f64::max)
        })
        .sum();
    total / short.len() as f64
}

fn full_date(p: &Patient) -> Option<PartialDate> {
    p.birth_date
        .as_deref()
        .and_then(parse_date)
        .filter(|d| d.month.is_some() && d.day.is_some())
}

fn birth_date_similarity(a: &Patient, b: &Patient) -> Option<f64> {
    let x = a.birth_date.as_deref().and_then(parse_date)?;
    let y = b.birth_date.as_deref().and_then(parse_date)?;
    if x == y {
        return Some(if x.day.is_some() {
            1.0
        } else if x.month.is_some() {
            0.8
        } else {
            0.6
        });
    }
    // Compare at the precision both carry.
    let (month_known, day_known) = (x.month.zip(y.month), x.day.zip(y.day));
    if x.year == y.year {
        match (month_known, day_known) {
            (Some((mx, my)), Some((dx, dy))) => {
                if mx == my {
                    return Some(typo(dx.into(), dy.into(), 2)); // day typo
                }
                if mx == dy && my == dx {
                    return Some(0.8); // day and month swapped
                }
                if dx == dy {
                    return Some(typo(mx.into(), my.into(), 2)); // month typo
                }
                return Some(0.0);
            }
            (Some((mx, my)), None) => return Some(if mx == my { 0.8 } else { 0.0 }),
            _ => return Some(0.6),
        }
    }
    if let (Some((mx, my)), Some((dx, dy))) = (month_known, day_known)
        && mx == my
        && dx == dy
    {
        return Some(typo(x.year, y.year, 4)); // year typo
    }
    Some(0.0)
}

/// 0.6 when two different values, written with `width` digits, differ in one digit or by
/// swapping two adjacent digits (a keying slip); otherwise 0.0.
fn typo(x: u16, y: u16, width: usize) -> f64 {
    let (a, b) = (format!("{x:0width$}"), format!("{y:0width$}"));
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let diff: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
    let slip = match diff.as_slice() {
        [_] => true,
        [i, j] => *j == i + 1 && a[*i] == b[*j] && a[*j] == b[*i],
        _ => false,
    };
    if slip { 0.6 } else { 0.0 }
}

fn sex_similarity(a: &Patient, b: &Patient) -> Option<f64> {
    let known = |p: &Patient| p.sex.filter(|s| *s != AdministrativeSex::Unknown);
    let (x, y) = (known(a)?, known(b)?);
    Some(if x == y { 1.0 } else { 0.0 })
}

fn postal_similarity(a: &Patient, b: &Patient) -> Option<f64> {
    let codes = |p: &Patient| -> Vec<String> {
        p.addresses
            .iter()
            .filter_map(|x| x.postal_code.as_deref())
            .map(fold_postcode)
            .filter(|s| !s.is_empty())
            .collect()
    };
    let (x, y) = (codes(a), codes(b));
    if x.is_empty() || y.is_empty() {
        return None;
    }
    Some(if x.iter().any(|c| y.contains(c)) {
        1.0
    } else {
        0.0
    })
}

fn telecom_similarity(a: &Patient, b: &Patient) -> Option<f64> {
    let values = |p: &Patient| -> Vec<String> {
        p.telecom
            .iter()
            .map(|t| fold_telecom(&t.system, &t.value))
            .filter(|s| !s.is_empty())
            .collect()
    };
    let (x, y) = (values(a), values(b));
    if x.is_empty() || y.is_empty() {
        return None;
    }
    Some(if x.iter().any(|c| y.contains(c)) {
        1.0
    } else {
        0.0
    })
}

fn valid_nhs_numbers(p: &Patient) -> Vec<String> {
    p.identifiers
        .iter()
        .filter(|i| i.system == NHS_NUMBER_SYSTEM && is_valid_nhs_number(&i.value))
        .map(|i| canonical_identifier(&i.value))
        .collect()
}
