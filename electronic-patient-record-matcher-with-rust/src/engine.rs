//! The matching engine: a pure, total, deterministic function from two
//! patients to an explainable result. No clocks, no randomness, no IO.

use crate::ihi::{IHI_SYSTEM, is_valid_ihi};
use crate::nhs::{NHS_NUMBER_SYSTEM, canonical_identifier, is_valid_nhs_number};
use crate::normalize::{
    PartialDate, are_nicknames, fold_name, fold_postcode, fold_telecom, is_usable_name,
    normalize_phone, parse_date, soundex, transliterate,
};
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
    /// Country calling code assumed for a phone number written with a leading `0` (44 for the UK).
    #[serde(default = "default_calling_code")]
    pub default_calling_code: u16,
    /// Treat a shared valid Australian IHI as the NHS number is treated (off by default).
    #[serde(default)]
    pub use_ihi: bool,
}

fn default_calling_code() -> u16 {
    44
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
            default_calling_code: default_calling_code(),
            use_ihi: false,
        }
    }
}

/// Why a `MatchConfig` is refused by `MatchConfig::validate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("a weight is negative or not a finite number")]
    InvalidWeight,
    #[error("the weights sum to zero, so no field would count")]
    NoWeight,
    #[error("a threshold is outside 0 to 1 or not a finite number")]
    ThresholdOutOfRange,
    #[error("review_threshold is above auto_link_threshold")]
    ThresholdsInverted,
    #[error("default_calling_code is outside 1 to 999")]
    InvalidCallingCode,
}

impl MatchConfig {
    /// Refuse a configuration that would make scores meaningless or let a pair auto-link below
    /// the review line. `Default` is valid. `MatchingEngine::new` does not call this; use `try_new`.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let w = &self.weights;
        let weights = [
            w.family,
            w.given,
            w.birth_date,
            w.sex,
            w.postal_code,
            w.telecom,
        ];
        if weights.iter().any(|x| !x.is_finite() || *x < 0.0) {
            return Err(ConfigError::InvalidWeight);
        }
        if weights.iter().sum::<f64>() <= 0.0 {
            return Err(ConfigError::NoWeight);
        }
        let thresholds = [
            self.auto_link_threshold,
            self.review_threshold,
            self.min_given_for_auto,
            self.min_family_for_auto,
            self.min_birth_date_for_auto,
            self.min_evidence_weight,
        ];
        if thresholds.iter().any(|x| !(0.0..=1.0).contains(x)) {
            return Err(ConfigError::ThresholdOutOfRange);
        }
        if self.review_threshold > self.auto_link_threshold {
            return Err(ConfigError::ThresholdsInverted);
        }
        if !(1..=999).contains(&self.default_calling_code) {
            return Err(ConfigError::InvalidCallingCode);
        }
        Ok(())
    }
}

/// The name or date a guard checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardField {
    GivenName,
    FamilyName,
    BirthDate,
}

impl GuardField {
    fn label(self) -> &'static str {
        match self {
            Self::GivenName => "given name",
            Self::FamilyName => "family name",
            Self::BirthDate => "birth date",
        }
    }
}

/// Why a link must not be automatic: a stable code a client can translate, with its numbers. The English
/// text in `MatchResult.conflicts` is `message()` of the same code, in the same order.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum ConflictCode {
    DifferentNhsNumbers,
    SameNhsDifferentBirthDates,
    SameNhsDifferentNames,
    DifferentIhis,
    SameIhiDifferentBirthDates,
    SameIhiDifferentNames,
    NamesSoundAlike,
    NamesSwapped,
    ExtraGivenName,
    /// A name or date needed to confirm the link is absent on one or both records.
    FieldUnavailable {
        field: GuardField,
    },
    /// A name or date is less similar than the auto-link minimum.
    FieldBelowMinimum {
        field: GuardField,
        similarity: f64,
        minimum: f64,
    },
    /// Less of the total field weight is present than `min_evidence_weight`.
    EvidenceThin {
        percent_present: f64,
    },
}

impl ConflictCode {
    /// The English text, kept stable for clients that print it.
    pub fn message(&self) -> String {
        match self {
            Self::DifferentNhsNumbers => "different verified NHS numbers".into(),
            Self::SameNhsDifferentBirthDates => "same NHS number but different birth dates".into(),
            Self::SameNhsDifferentNames => "same NHS number but different names".into(),
            Self::DifferentIhis => "different verified IHIs".into(),
            Self::SameIhiDifferentBirthDates => "same IHI but different birth dates".into(),
            Self::SameIhiDifferentNames => "same IHI but different names".into(),
            Self::NamesSoundAlike => "names sound alike but are spelled differently".into(),
            Self::NamesSwapped => "given and family names appear swapped".into(),
            Self::ExtraGivenName => "one record has a given name the other lacks".into(),
            Self::FieldUnavailable { field } => {
                format!("{} not available to confirm the link", field.label())
            }
            Self::FieldBelowMinimum {
                field,
                similarity,
                minimum,
            } => format!(
                "{} similarity {similarity:.2} below {minimum:.2}",
                field.label()
            ),
            Self::EvidenceThin { percent_present } => {
                format!("only {percent_present:.0}% of the evidence is present")
            }
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
    /// The same reasons as stable codes, in the same order, for a client that translates them.
    #[serde(default)]
    pub conflict_codes: Vec<ConflictCode>,
    /// The family and the first given name of one name on each record sound alike (Soundex) though they
    /// are spelled differently. Never part of the score; see `compare`.
    #[serde(default)]
    pub phonetic_match: bool,
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

    /// Like `new`, but refuses a configuration that `MatchConfig::validate` rejects.
    pub fn try_new(config: MatchConfig) -> Result<Self, ConfigError> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn config(&self) -> &MatchConfig {
        &self.config
    }

    /// Compare two patients. Symmetric: `compare(a, b)` equals `compare(b, a)`.
    pub fn compare(&self, a: &Patient, b: &Patient) -> MatchResult {
        let w = &self.config.weights;
        let mut family = family_similarity(a, b);
        let (mut given, given_extra) = match given_similarity(a, b) {
            Some((s, e)) => (Some(s), e),
            None => (None, false),
        };
        // Given and family name keyed into each other's boxes: when neither name matches as written
        // but each matches the other's, score the swapped pair at a penalty and block AutoLink.
        let swapped = match (family, given, swapped_similarity(a, b)) {
            (Some(f), Some(g), Some(s)) if f < SWAP_FLOOR && g < SWAP_FLOOR => Some(s),
            _ => None,
        };
        if let Some(s) = swapped {
            family = Some(s * SWAP_PENALTY);
            given = Some(s * SWAP_PENALTY);
        }
        let dob = birth_date_similarity(a, b);
        let sex = sex_similarity(a, b);
        let postal = postal_similarity(a, b);
        let (telecom, telecom_individual) =
            match telecom_similarity(a, b, self.config.default_calling_code) {
                Some((s, individual)) => (Some(s), individual),
                None => (None, false),
            };
        // Household evidence: a postcode, and a phone number shared without an email, say where a person
        // lives or can be reached, which relatives share. They count at half weight.
        let postal_weight = w.postal_code * HOUSEHOLD_WEIGHT_FACTOR;
        let telecom_weight = if telecom_individual {
            w.telecom
        } else {
            w.telecom * HOUSEHOLD_WEIGHT_FACTOR
        };

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
                postal_weight,
                "no postcode on one or both records",
            ),
            field(
                "telecom",
                telecom,
                telecom_weight,
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

        let phonetic_match = phonetic_match(a, b);
        let mut conflicts: Vec<ConflictCode> = Vec::new();
        let mut deterministic = None;
        let mut decision;

        // National identifiers: a shared valid number decides; two different valid ones forbid a link.
        // The IHI counts only when `use_ihi` is on; a clash on either number forbids a link.
        let (nhs_a, nhs_b) = (
            valid_numbers(a, NHS_NUMBER_SYSTEM, is_valid_nhs_number),
            valid_numbers(b, NHS_NUMBER_SYSTEM, is_valid_nhs_number),
        );
        let (ihi_a, ihi_b) = if self.config.use_ihi {
            (
                valid_numbers(a, IHI_SYSTEM, is_valid_ihi),
                valid_numbers(b, IHI_SYSTEM, is_valid_ihi),
            )
        } else {
            (vec![], vec![])
        };
        let nhs_shared = nhs_a.iter().any(|n| nhs_b.contains(n));
        let ihi_shared = ihi_a.iter().any(|n| ihi_b.contains(n));
        let nhs_clash = !nhs_a.is_empty() && !nhs_b.is_empty() && !nhs_shared;
        let ihi_clash = !ihi_a.is_empty() && !ihi_b.is_empty() && !ihi_shared;
        let clash = nhs_clash || ihi_clash;
        let shared = nhs_shared || ihi_shared;
        let dob_conflict = matches!((full_date(a), full_date(b)), (Some(x), Some(y)) if x != y);

        if clash {
            conflicts.push(if nhs_clash {
                ConflictCode::DifferentNhsNumbers
            } else {
                ConflictCode::DifferentIhis
            });
            decision = Decision::NonMatch;
        } else if shared {
            deterministic = Some(
                if nhs_shared {
                    "shared verified NHS number"
                } else {
                    "shared verified IHI"
                }
                .to_string(),
            );
            score = 1.0;
            if dob_conflict {
                conflicts.push(if nhs_shared {
                    ConflictCode::SameNhsDifferentBirthDates
                } else {
                    ConflictCode::SameIhiDifferentBirthDates
                });
            }
            // A family name alone may change (marriage); both names disagreeing suggests a
            // mistyped or reused number.
            let c = &self.config;
            if given.is_some_and(|s| s < c.min_given_for_auto)
                && family.is_some_and(|s| s < c.min_family_for_auto)
            {
                conflicts.push(if nhs_shared {
                    ConflictCode::SameNhsDifferentNames
                } else {
                    ConflictCode::SameIhiDifferentNames
                });
            }
            // The rule for every other pair holds here too: missing evidence is not agreement, so a
            // shared number with no given name, family name or birth date to check it against is for a
            // person to confirm (CQ-37). A name that differs is not blocked here (a family name can
            // change by marriage); only a name or date that is absent is.
            for (field, sim) in [
                (GuardField::GivenName, given),
                (GuardField::FamilyName, family),
                (GuardField::BirthDate, dob),
            ] {
                if sim.is_none() {
                    conflicts.push(ConflictCode::FieldUnavailable { field });
                }
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
            // A pair the spelling scores as unrelated, with names that sound alike and the same birth
            // date, is worth a person's look. The score is unchanged, and this never reaches AutoLink.
            if decision == Decision::NonMatch
                && phonetic_match
                && dob.is_some_and(|d| d >= self.config.min_birth_date_for_auto)
            {
                conflicts.push(ConflictCode::NamesSoundAlike);
                decision = Decision::Review;
            }
            if decision == Decision::AutoLink {
                let c = &self.config;
                // Missing evidence blocks automatic linking; it is not treated as agreement.
                let guards = [
                    (GuardField::GivenName, given, c.min_given_for_auto),
                    (GuardField::FamilyName, family, c.min_family_for_auto),
                    (GuardField::BirthDate, dob, c.min_birth_date_for_auto),
                ];
                for (field, sim, min) in guards {
                    match sim {
                        None => conflicts.push(ConflictCode::FieldUnavailable { field }),
                        Some(s) if s < min => conflicts.push(ConflictCode::FieldBelowMinimum {
                            field,
                            similarity: s,
                            minimum: min,
                        }),
                        _ => {}
                    }
                }
                if swapped.is_some() {
                    conflicts.push(ConflictCode::NamesSwapped);
                }
                if given_extra {
                    // `John` and `John Paul` can be two people (twins, a father and son).
                    conflicts.push(ConflictCode::ExtraGivenName);
                }
                if present / total < c.min_evidence_weight {
                    conflicts.push(ConflictCode::EvidenceThin {
                        percent_present: 100.0 * present / total,
                    });
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
            conflicts: conflicts.iter().map(ConflictCode::message).collect(),
            conflict_codes: conflicts,
            phonetic_match,
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
    let (fx, fy) = (fold_name(x), fold_name(y));
    // `Müller` against `Mueller`: the better of the spelling as written and the transliterated one.
    folded_name_sim(&fx, &fy).max(folded_name_sim(&transliterate(&fx), &transliterate(&fy)))
}

fn folded_name_sim(fx: &str, fy: &str) -> f64 {
    if fx.contains(' ') || fy.contains(' ') {
        compound_sim(fx, fy)
    } else {
        strsim::jaro_winkler(fx, fy)
    }
}

/// A family name of several parts (`Smith-Jones`) against another name: each part of the shorter
/// takes its best match among the parts of the longer, and the mean is kept. A part counts for a
/// similar part only in proportion to their lengths, so `Smith-Jones` does not match `Smithson`
/// because `Smith` is a prefix of it, while `Smith` still matches `Smith-Jones` (a name that lost
/// or gained a part on marriage).
fn compound_sim(x: &str, y: &str) -> f64 {
    let (xs, ys): (Vec<&str>, Vec<&str>) = (x.split(' ').collect(), y.split(' ').collect());
    let part = |a: &str, b: &str| {
        let (la, lb) = (a.chars().count() as f64, b.chars().count() as f64);
        strsim::jaro_winkler(a, b) * la.min(lb) / la.max(lb)
    };
    let mean = |short: &[&str], long: &[&str]| {
        let total: f64 = short
            .iter()
            .map(|s| long.iter().map(|l| part(s, l)).fold(0.0, f64::max))
            .sum();
        total / short.len() as f64
    };
    // With equal numbers of parts, both ways and the lower kept, so the order of the records
    // never raises the score.
    match xs.len().cmp(&ys.len()) {
        std::cmp::Ordering::Less => mean(&xs, &ys),
        std::cmp::Ordering::Greater => mean(&ys, &xs),
        std::cmp::Ordering::Equal => mean(&xs, &ys).min(mean(&ys, &xs)),
    }
}

/// Whether one name on each record has the same Soundex for the family name and for the first given
/// name while being spelled differently in at least one of the two (identical spellings are not news).
fn phonetic_match(a: &Patient, b: &Patient) -> bool {
    let parts = |p: &Patient| -> Vec<(String, String)> {
        p.names
            .iter()
            .filter_map(|n| {
                let family = fold_name(n.family.as_deref().filter(|s| is_usable_name(s))?);
                let given = fold_name(n.given.iter().find(|s| is_usable_name(s))?);
                Some((family, given))
            })
            .collect()
    };
    let (xa, xb) = (parts(a), parts(b));
    xa.iter().any(|(fa, ga)| {
        xb.iter().any(|(fb, gb)| {
            (fa != fb || ga != gb)
                && soundex(fa).is_some_and(|s| Some(s) == soundex(fb))
                && soundex(ga).is_some_and(|s| Some(s) == soundex(gb))
        })
    })
}

/// A name pair is read as swapped only when each name matches the other's at least this well, and
/// neither matches as written (the same floor as the auto-link name guards).
const SWAP_FLOOR: f64 = 0.85;
/// A swapped pair is scored at this fraction of its similarity: it is weaker evidence.
const SWAP_PENALTY: f64 = 0.9;

/// The best, over name pairs, of the lower of: family against the other's first given name, and
/// first given name against the other's family. `None` below `SWAP_FLOOR` or when a name is absent.
/// Symmetric, because `name_sim` is.
fn swapped_similarity(a: &Patient, b: &Patient) -> Option<f64> {
    let parts = |p: &Patient| -> Vec<(String, String)> {
        p.names
            .iter()
            .filter_map(|n| {
                let family = n.family.as_deref().filter(|s| is_usable_name(s))?;
                let given = n.given.iter().find(|s| is_usable_name(s))?;
                Some((family.to_string(), given.clone()))
            })
            .collect()
    };
    let mut best: Option<f64> = None;
    for (fa, ga) in parts(a) {
        for (fb, gb) in parts(b) {
            let s = name_sim(&fa, &gb).min(name_sim(&ga, &fb));
            if s >= SWAP_FLOOR {
                best = Some(best.map_or(s, |x: f64| x.max(s)));
            }
        }
    }
    best
}

fn family_similarity(a: &Patient, b: &Patient) -> Option<f64> {
    best_over(
        a.names
            .iter()
            .filter_map(|n| n.family.as_deref())
            .filter(|s| is_usable_name(s)),
        b.names
            .iter()
            .filter_map(|n| n.family.as_deref())
            .filter(|s| is_usable_name(s)),
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

/// A nickname and its formal name (`Bill`, `William`) score this: a match, but under `min_given_for_auto`,
/// so it can reach Review and never links automatically on the name alone.
const NICKNAME_SIMILARITY: f64 = 0.8;

/// Initials match tokens that start with the same letter (scored 0.8).
fn given_token_sim(x: &str, y: &str) -> f64 {
    let (fx, fy) = (fold_name(x), fold_name(y));
    let initial = |s: &str| s.chars().count() == 1;
    if (initial(&fx) || initial(&fy)) && fx.chars().next() == fy.chars().next() {
        return 0.8;
    }
    let spelled = strsim::jaro_winkler(&fx, &fy).max(strsim::jaro_winkler(
        &transliterate(&fx),
        &transliterate(&fy),
    ));
    if are_nicknames(&fx, &fy) {
        spelled.max(NICKNAME_SIMILARITY)
    } else {
        spelled
    }
}

/// Given-name similarity, and whether the best-matching pair of lists leaves a name unmatched on
/// the longer side (`John` against `John Paul`: the average over the shorter list cannot see it).
fn given_similarity(a: &Patient, b: &Patient) -> Option<(f64, bool)> {
    let mut best: Option<(f64, bool)> = None;
    // A token that folds to nothing (`-`, spaces) or is a placeholder (`unknown`) is absent, not a name that disagrees.
    let tokens = |given: &[String]| -> Vec<String> {
        given
            .iter()
            .filter(|t| is_usable_name(t))
            .cloned()
            .collect()
    };
    for na in a.names.iter().map(|n| tokens(&n.given)) {
        for nb in b.names.iter().map(|n| tokens(&n.given)) {
            if na.is_empty() || nb.is_empty() {
                continue;
            }
            // Average over the shorter list; with equal lengths, average both ways and keep the
            // lower, so the order of the records never raises the score.
            let (s, extra) = match na.len().cmp(&nb.len()) {
                std::cmp::Ordering::Less => (given_average(&na, &nb), unmatched(&na, &nb)),
                std::cmp::Ordering::Greater => (given_average(&nb, &na), unmatched(&nb, &na)),
                std::cmp::Ordering::Equal => {
                    (given_average(&na, &nb).min(given_average(&nb, &na)), false)
                }
            };
            // The better pair wins; on a tie the one with nothing unmatched.
            best = match best {
                Some((b, e)) if b > s || (b == s && !e) => Some((b, e)),
                _ => Some((s, extra)),
            };
        }
    }
    best
}

/// Whether `long` has a token that no token of `short` matches well enough to be the same name.
fn unmatched(short: &[String], long: &[String]) -> bool {
    long.iter().any(|l| {
        short
            .iter()
            .all(|s| given_token_sim(s, l) < UNMATCHED_BELOW)
    })
}

/// A given-name token is unmatched below this similarity to every token on the other side.
const UNMATCHED_BELOW: f64 = 0.85;

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

/// Half weight for a shared postcode or phone (see `compare`); a shared email address is individual.
const HOUSEHOLD_WEIGHT_FACTOR: f64 = 0.5;

/// The similarity, and whether an email address in common decided it (individual evidence).
fn telecom_similarity(a: &Patient, b: &Patient, calling_code: u16) -> Option<(f64, bool)> {
    // (is an email, folded value)
    let values = |p: &Patient| -> Vec<(bool, String)> {
        p.telecom
            .iter()
            .map(|t| {
                let email = t.system.eq_ignore_ascii_case("email");
                let v = if email {
                    fold_telecom(&t.system, &t.value)
                } else {
                    normalize_phone(&t.value, calling_code)
                };
                (email, v)
            })
            .filter(|(_, s)| !s.is_empty())
            .collect()
    };
    let (x, y) = (values(a), values(b));
    if x.is_empty() || y.is_empty() {
        return None;
    }
    let email_match = x.iter().any(|c| c.0 && y.contains(c));
    let any_match = x.iter().any(|c| y.contains(c));
    Some((if any_match { 1.0 } else { 0.0 }, email_match))
}

fn valid_numbers(p: &Patient, system: &str, valid: fn(&str) -> bool) -> Vec<String> {
    p.identifiers
        .iter()
        .filter(|i| i.system == system && valid(&i.value))
        .map(|i| canonical_identifier(&i.value))
        .collect()
}
