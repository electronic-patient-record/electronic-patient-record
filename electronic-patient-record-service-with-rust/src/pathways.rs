//! Suspicion pathways as configurable data (RS-6, `spec/referral-suspicion/index.md`): per suspected
//! condition, when its clock starts, its target dates, its priority, and what else it needs. Pure: the
//! catalogue and the arithmetic, no database.
//!
//! Cancer is one pathway among several. Each pathway version is data in `src/suspicion_pathways.toml`,
//! read into the build, with the sources it came from and an approver and day (both empty: a draft). A
//! pathway either sets one priority and its targets, or has **bands** the writer must choose between
//! (heart failure: the NT-proBNP value sets the interval), each with its own priority and targets. It may
//! need the person's **agreement** (dementia), carry an **instruction** for the receiving team
//! (Parkinson's: untreated), and list **factors** that raise the priority (sleep apnoea and driving). A
//! pathway with no interval says why, so a missing target date is never read as "no hurry".
//!
//! [`plan`] turns a pathway, a band, the factors and the clock start into the priority and the due day of
//! each target. These are the record's view, shown beside the suspicion; the PAS owns the clock that is
//! reported (PA-15), and its pauses and stops are not modelled here.

use crate::referrals_store::Priority;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::LazyLock;

/// The catalogue, as built in.
pub const SOURCE: &str = include_str!("suspicion_pathways.toml");

/// The longest target, in days.
pub const MAX_DAYS: u32 = 365;

/// When a pathway's clock starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClockFrom {
    /// The recorded point of suspicion.
    PointOfSuspicion,
    /// When the referral was sent (its first version).
    ReferralSent,
}

/// A target date, as days from the clock start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// Stable within the pathway.
    pub id: String,
    /// What should have happened by then.
    pub text: String,
    /// Days from the clock start; 0 is the same day.
    pub days: u32,
}

/// A band the writer chooses, with its own priority and targets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    /// Stable within the pathway.
    pub id: String,
    /// What puts the patient in this band.
    pub text: String,
    /// Its priority.
    pub priority: Priority,
    /// Whether the suspicion's grounds must be a result (the band follows a measured value).
    pub needs_result: bool,
    /// Its targets.
    #[serde(default, rename = "target")]
    pub targets: Vec<Target>,
}

/// A circumstance that raises the priority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Factor {
    /// Stable within the pathway.
    pub id: String,
    /// The circumstance.
    pub text: String,
    /// The priority it raises the suspicion to, at least.
    pub priority: Priority,
}

/// One code in a subtype list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubtypeCode {
    /// The code, as the list's owner writes it.
    pub code: String,
    /// Its name.
    pub name: String,
}

/// The list a suspicion's subtype comes from (RS-5): a primary cancer site, a type of dementia. The
/// scheme is named with the code, so a code from one list is never read as one from another; the PAS
/// checks the same scheme against its own profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subtypes {
    /// The list's name, stored with each code (for example `wales-primary-cancer-site`).
    pub scheme: String,
    /// What a subtype is here (for example "Primary cancer site").
    pub title: String,
    /// Where the list comes from.
    pub source: String,
    /// Whether every suspicion under this pathway version must name one.
    pub required: bool,
    /// The codes.
    #[serde(rename = "code")]
    pub codes: Vec<SubtypeCode>,
}

/// One version of one pathway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pathway {
    /// The pathway.
    pub id: String,
    /// Its version, from 1.
    pub version: i32,
    /// Its title.
    pub title: String,
    /// Where the figures came from (keys in the spec's source list).
    pub sources: Vec<String>,
    /// The clinical approver; empty when not approved.
    pub approved_by: String,
    /// The day of approval, `YYYY-MM-DD`; empty when not approved.
    pub approved_on: String,
    /// When the clock starts.
    pub clock_from: ClockFrom,
    /// The priority, when the pathway has no bands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    /// Whether the person's agreement to the referral must be recorded.
    #[serde(default)]
    pub needs_agreement: bool,
    /// What the receiving team must see, if anything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    /// Why there is no target date, when there is none anywhere; empty otherwise.
    pub no_target_reason: String,
    /// The targets, when the pathway has no bands.
    #[serde(default, rename = "target")]
    pub targets: Vec<Target>,
    /// The bands, one of which the writer must choose.
    #[serde(default, rename = "band")]
    pub bands: Vec<Band>,
    /// The factors that raise the priority.
    #[serde(default, rename = "factor")]
    pub factors: Vec<Factor>,
    /// The subtype list, if a suspicion may or must name one; none means no subtype is allowed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtypes: Option<Subtypes>,
}

impl Pathway {
    /// Whether a named clinician approved this version on a stated day.
    #[must_use]
    pub fn approved(&self) -> bool {
        !self.approved_by.trim().is_empty() && !self.approved_on.trim().is_empty()
    }

    /// One of its bands.
    #[must_use]
    pub fn band(&self, id: &str) -> Option<&Band> {
        self.bands.iter().find(|b| b.id == id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    pathway: Vec<Pathway>,
}

/// How urgent a priority is, lowest first.
#[must_use]
pub fn rank(p: Priority) -> u8 {
    match p {
        Priority::Routine => 0,
        Priority::Urgent => 1,
        Priority::Asap => 2,
        Priority::Stat => 3,
    }
}

fn unique<'a>(name: &str, what: &str, ids: impl Iterator<Item = &'a str>) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() || !seen.insert(id) {
            return Err(format!(
                "suspicion pathways: {name} has a blank or repeated {what} id"
            ));
        }
    }
    Ok(())
}

fn targets_ok(name: &str, targets: &[Target]) -> Result<(), String> {
    unique(name, "target", targets.iter().map(|t| t.id.as_str()))?;
    if let Some(t) = targets
        .iter()
        .find(|t| t.days > MAX_DAYS || t.text.trim().is_empty())
    {
        return Err(format!(
            "suspicion pathways: {name} target {} needs words and 0 to {MAX_DAYS} days",
            t.id
        ));
    }
    Ok(())
}

fn check(p: &Pathway) -> Result<(), String> {
    let name = format!("{} v{}", p.id, p.version);
    if p.id.trim().is_empty() || p.title.trim().is_empty() || p.sources.is_empty() {
        return Err(format!(
            "suspicion pathways: {name} needs an id, a title and its sources"
        ));
    }
    match (
        p.bands.is_empty(),
        p.priority.is_some(),
        p.targets.is_empty(),
    ) {
        (true, true, _) => targets_ok(&name, &p.targets)?,
        (false, false, true) => {
            unique(&name, "band", p.bands.iter().map(|b| b.id.as_str()))?;
            for b in &p.bands {
                targets_ok(&format!("{name} band {}", b.id), &b.targets)?;
            }
        }
        _ => {
            return Err(format!(
                "suspicion pathways: {name} needs bands, or its own priority and targets, not both"
            ));
        }
    }
    unique(&name, "factor", p.factors.iter().map(|f| f.id.as_str()))?;
    if let Some(l) = &p.subtypes {
        if l.scheme.trim().is_empty()
            || l.title.trim().is_empty()
            || l.source.trim().is_empty()
            || l.codes.is_empty()
            || l.codes.iter().any(|c| c.name.trim().is_empty())
        {
            return Err(format!(
                "suspicion pathways: {name} subtypes need a scheme, a title, a source and named codes"
            ));
        }
        unique(&name, "subtype", l.codes.iter().map(|c| c.code.as_str()))?;
    }
    let any_target = !p.targets.is_empty() || p.bands.iter().any(|b| !b.targets.is_empty());
    if any_target == !p.no_target_reason.trim().is_empty() {
        return Err(format!(
            "suspicion pathways: {name} gives no_target_reason exactly when it has no target"
        ));
    }
    let by = p.approved_by.trim().is_empty();
    let on = p.approved_on.trim();
    if by != on.is_empty() || (!on.is_empty() && NaiveDate::parse_from_str(on, "%Y-%m-%d").is_err())
    {
        return Err(format!(
            "suspicion pathways: {name} needs both an approver and a YYYY-MM-DD day, or neither"
        ));
    }
    Ok(())
}

/// Parse a catalogue.
///
/// # Errors
///
/// A message naming the problem: not TOML of this shape; versions that do not run 1, 2, 3; a pathway
/// with bands and its own targets, or neither priority nor bands; a blank or repeated id; a target over
/// [`MAX_DAYS`]; a missing or needless `no_target_reason`; an approver without a day.
pub fn parse(text: &str) -> Result<Vec<Pathway>, String> {
    let file: File = toml::from_str(text).map_err(|e| format!("suspicion pathways: {e}"))?;
    let mut all = file.pathway;
    all.sort_by(|a, b| (a.id.as_str(), a.version).cmp(&(b.id.as_str(), b.version)));
    for (i, p) in all.iter().enumerate() {
        let previous = i.checked_sub(1).map(|j| &all[j]).filter(|q| q.id == p.id);
        let expected = previous.map_or(1, |q| q.version + 1);
        if p.version != expected {
            return Err(format!(
                "suspicion pathways: {} v{} should be version {expected}",
                p.id, p.version
            ));
        }
        check(p)?;
    }
    Ok(all)
}

static PATHWAYS: LazyLock<Vec<Pathway>> = LazyLock::new(|| {
    // The built-in catalogue is proved to parse by the tests below.
    parse(SOURCE).unwrap_or_default()
});

/// Every pathway version, by id then version.
#[must_use]
pub fn catalogue() -> &'static [Pathway] {
    &PATHWAYS
}

/// One pathway version.
#[must_use]
pub fn pathway(id: &str, version: i32) -> Option<&'static Pathway> {
    PATHWAYS.iter().find(|p| p.id == id && p.version == version)
}

/// The newest version of a pathway.
#[must_use]
pub fn latest(id: &str) -> Option<i32> {
    PATHWAYS
        .iter()
        .filter(|p| p.id == id)
        .map(|p| p.version)
        .max()
}

/// What the writer chose within a pathway.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Choice<'a> {
    /// The band, for a pathway with bands.
    pub band: Option<&'a str>,
    /// The factors that apply.
    pub factors: &'a [String],
    /// Whether the grounds are a result.
    pub grounds_is_result: bool,
    /// The person's agreement, where recorded.
    pub agreed: Option<bool>,
    /// The subtype named, as its scheme and code.
    pub subtype: Option<(&'a str, &'a str)>,
}

/// Every problem with a choice against its pathway, as words naming ids, never patient data.
#[must_use]
pub fn problems(p: &Pathway, c: &Choice<'_>) -> Vec<String> {
    let mut out = Vec::new();
    match (p.bands.is_empty(), c.band) {
        (true, Some(_)) => out.push(format!("pathway {} has no bands", p.id)),
        (false, None) => out.push(format!("pathway {} needs a band", p.id)),
        (false, Some(b)) => match p.band(b) {
            None => out.push(format!("pathway {} has no band {b}", p.id)),
            Some(band) if band.needs_result && !c.grounds_is_result => {
                out.push(format!("band {b} needs grounds of kind result"));
            }
            Some(_) => {}
        },
        (true, None) => {}
    }
    let mut seen = BTreeSet::new();
    for f in c.factors {
        if !seen.insert(f.as_str()) {
            out.push(format!("factor {f} is given twice"));
        } else if !p.factors.iter().any(|x| &x.id == f) {
            out.push(format!("pathway {} has no factor {f}", p.id));
        }
    }
    match (p.needs_agreement, c.agreed) {
        (true, None) => out.push(format!(
            "pathway {} needs the person's agreement recorded, yes or no",
            p.id
        )),
        (false, Some(_)) => out.push(format!("pathway {} does not record agreement", p.id)),
        _ => {}
    }
    match (&p.subtypes, c.subtype) {
        (None, Some(_)) => out.push(format!("pathway {} has no subtype list", p.id)),
        (Some(l), None) if l.required => {
            out.push(format!(
                "pathway {} needs a subtype from {}",
                p.id, l.scheme
            ));
        }
        (Some(l), Some((scheme, _))) if scheme != l.scheme => out.push(format!(
            "pathway {} takes subtypes from {}, not {scheme}",
            p.id, l.scheme
        )),
        (Some(l), Some((_, code))) if !l.codes.iter().any(|x| x.code == code) => {
            out.push(format!("{} has no code {code}", l.scheme));
        }
        _ => {}
    }
    out
}

/// One target with its due day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Due {
    /// The target.
    pub id: String,
    /// What should have happened.
    pub text: String,
    /// Days from the clock start.
    pub days: u32,
    /// The last day it is on time, in the plan's time zone.
    pub due_on: String,
    /// Whether that day has passed. An outcome stops nothing yet (RS-3), so this is the day alone.
    pub overdue: bool,
}

/// What a pathway makes of one suspicion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Plan {
    /// The pathway, its version and title.
    pub pathway: String,
    /// The version.
    pub version: i32,
    /// The title.
    pub title: String,
    /// Whether that version is approved; `false` is a draft.
    pub approved: bool,
    /// The newest version of the pathway.
    pub latest_version: i32,
    /// When the clock starts, and from what.
    pub clock_from: ClockFrom,
    /// The clock start (RFC 3339).
    pub clock_start: String,
    /// The time zone whose days the due days are (the site's).
    pub time_zone: String,
    /// The highest of the pathway's (or band's) priority and every factor's.
    pub priority: Priority,
    /// The instruction for the receiving team, if any.
    pub instruction: Option<String>,
    /// The targets with their due days, in order.
    pub targets: Vec<Due>,
    /// Why there are none, when there are none.
    pub no_target_reason: Option<String>,
}

/// The plan for a choice already checked by [`problems`]: the priority and each target's due day,
/// counted in whole days from the clock start's day in `zone` (the site's, `EPR_SITE_TIME_ZONE`), and
/// judged overdue on today's day there. `None` when the choice does not fit the pathway.
#[must_use]
pub fn plan(
    p: &Pathway,
    c: &Choice<'_>,
    clock_start: DateTime<Utc>,
    now: DateTime<Utc>,
    zone: chrono_tz::Tz,
) -> Option<Plan> {
    if !problems(p, c).is_empty() {
        return None;
    }
    let band = c.band.and_then(|b| p.band(b));
    let base = band.map(|b| b.priority).or(p.priority)?;
    let priority = c
        .factors
        .iter()
        .filter_map(|f| p.factors.iter().find(|x| &x.id == f))
        .map(|f| f.priority)
        .fold(base, |a, b| if rank(b) > rank(a) { b } else { a });
    let targets = band.map_or(&p.targets, |b| &b.targets);
    let start = clock_start.with_timezone(&zone).date_naive();
    let today = now.with_timezone(&zone).date_naive();
    let targets = targets
        .iter()
        .map(|t| {
            let due = start + chrono::Days::new(u64::from(t.days));
            Due {
                id: t.id.clone(),
                text: t.text.clone(),
                days: t.days,
                due_on: due.format("%Y-%m-%d").to_string(),
                overdue: today > due,
            }
        })
        .collect::<Vec<_>>();
    Some(Plan {
        pathway: p.id.clone(),
        version: p.version,
        title: p.title.clone(),
        approved: p.approved(),
        latest_version: latest(&p.id).unwrap_or(p.version),
        clock_from: p.clock_from,
        clock_start: clock_start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        time_zone: zone.name().to_string(),
        priority,
        instruction: p.instruction.clone(),
        no_target_reason: Some(p.no_target_reason.trim().to_string()).filter(|r| !r.is_empty()),
        targets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::{Europe::London, UTC};

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn the_built_in_catalogue_parses_and_every_pathway_is_a_draft() {
        let all = parse(SOURCE).expect("src/suspicion_pathways.toml must parse");
        let ids: BTreeSet<&str> = all.iter().map(|p| p.id.as_str()).collect();
        for id in [
            "suspected-cancer-wales",
            "suspected-cancer-england",
            "type-1-diabetes-child",
            "heart-failure",
            "parkinsons",
            "dementia",
            "obstructive-sleep-apnoea",
        ] {
            assert!(ids.contains(id), "{id}");
        }
        assert!(
            all.iter().all(|p| !p.approved()),
            "nobody has approved them"
        );
        assert_eq!(catalogue().len(), all.len());
    }

    #[test]
    fn wales_cancer_counts_from_the_point_of_suspicion() {
        let p = pathway("suspected-cancer-wales", 1).unwrap();
        let plan = plan(
            p,
            &Choice::default(),
            at("2026-11-09T09:30:00Z"),
            at("2026-12-08T10:00:00Z"),
            UTC,
        )
        .unwrap();
        assert_eq!(plan.clock_from, ClockFrom::PointOfSuspicion);
        assert_eq!(plan.priority, Priority::Urgent);
        let due: Vec<(&str, &str, bool)> = plan
            .targets
            .iter()
            .map(|t| (t.id.as_str(), t.due_on.as_str(), t.overdue))
            .collect();
        assert_eq!(
            due,
            vec![
                ("diagnosis", "2026-12-07", true),
                ("first-treatment", "2027-01-10", false)
            ]
        );
        assert!(!plan.approved);
    }

    #[test]
    fn due_days_are_the_sites_days() {
        let p = pathway("suspected-cancer-wales", 1).unwrap();
        let start = at("2026-06-30T23:30:00Z");
        let now = at("2026-07-29T22:30:00Z");
        let utc = plan(p, &Choice::default(), start, now, UTC).unwrap();
        let local = plan(p, &Choice::default(), start, now, London).unwrap();
        assert_eq!(utc.targets[0].due_on, "2026-07-28");
        assert!(utc.targets[0].overdue);
        assert_eq!(
            local.targets[0].due_on, "2026-07-29",
            "23:30 UTC is already 1 July in London"
        );
        assert!(!local.targets[0].overdue, "and 29 July there still");
        assert_eq!(local.time_zone, "Europe/London");
    }

    #[test]
    fn a_same_day_target_is_overdue_only_the_next_day() {
        let p = pathway("type-1-diabetes-child", 1).unwrap();
        let start = at("2026-11-09T16:00:00Z");
        let late = plan(
            p,
            &Choice::default(),
            start,
            at("2026-11-09T23:59:00Z"),
            UTC,
        )
        .unwrap();
        assert_eq!(late.priority, Priority::Stat);
        assert!(!late.targets[0].overdue);
        let next = plan(
            p,
            &Choice::default(),
            start,
            at("2026-11-10T00:01:00Z"),
            UTC,
        )
        .unwrap();
        assert!(next.targets[0].overdue);
    }

    #[test]
    fn a_band_follows_the_result_and_sets_the_interval() {
        let p = pathway("heart-failure", 1).unwrap();
        let start = at("2026-11-09T09:00:00Z");
        let no_band = Choice::default();
        assert_eq!(
            problems(p, &no_band),
            vec!["pathway heart-failure needs a band"]
        );
        let judgement = Choice {
            band: Some("nt-probnp-above-2000"),
            ..Choice::default()
        };
        assert_eq!(
            problems(p, &judgement),
            vec!["band nt-probnp-above-2000 needs grounds of kind result"]
        );
        for (band, days, priority) in [
            ("nt-probnp-above-2000", 14, Priority::Urgent),
            ("nt-probnp-400-to-2000", 42, Priority::Routine),
        ] {
            let c = Choice {
                band: Some(band),
                grounds_is_result: true,
                ..Choice::default()
            };
            let plan = plan(p, &c, start, start, UTC).unwrap();
            assert_eq!((plan.targets[0].days, plan.priority), (days, priority));
        }
    }

    #[test]
    fn a_factor_raises_the_priority_and_an_unknown_one_is_refused() {
        let p = pathway("obstructive-sleep-apnoea", 1).unwrap();
        let start = at("2026-11-09T09:00:00Z");
        let plain = plan(p, &Choice::default(), start, start, UTC).unwrap();
        assert_eq!(plain.priority, Priority::Routine);
        assert!(plain.targets.is_empty());
        assert!(plain.no_target_reason.is_some(), "no interval says why");
        let driver = ["vocational-driver".to_string()];
        let c = Choice {
            factors: &driver,
            ..Choice::default()
        };
        assert_eq!(
            plan(p, &c, start, start, UTC).unwrap().priority,
            Priority::Urgent
        );
        let odd = ["pilot".to_string()];
        let c = Choice {
            factors: &odd,
            ..Choice::default()
        };
        assert_eq!(
            problems(p, &c),
            vec!["pathway obstructive-sleep-apnoea has no factor pilot"]
        );
    }

    #[test]
    fn dementia_needs_the_persons_agreement_recorded_either_way() {
        let p = pathway("dementia", 1).unwrap();
        assert_eq!(
            problems(p, &Choice::default()),
            vec!["pathway dementia needs the person's agreement recorded, yes or no"]
        );
        let declined = Choice {
            agreed: Some(false),
            ..Choice::default()
        };
        assert!(problems(p, &declined).is_empty(), "declined is a state");
        let wales = pathway("suspected-cancer-wales", 1).unwrap();
        assert_eq!(
            problems(wales, &declined),
            vec!["pathway suspected-cancer-wales does not record agreement"]
        );
    }

    #[test]
    fn a_subtype_comes_from_the_pathways_own_list() {
        let v1 = pathway("suspected-cancer-wales", 1).unwrap();
        let v2 = pathway("suspected-cancer-wales", 2).unwrap();
        let lower_gi = Choice {
            subtype: Some(("wales-primary-cancer-site", "03")),
            ..Choice::default()
        };
        assert_eq!(
            problems(v1, &lower_gi),
            vec!["pathway suspected-cancer-wales has no subtype list"],
            "version 1 stays as it was"
        );
        assert!(problems(v2, &lower_gi).is_empty());
        assert_eq!(
            problems(v2, &Choice::default()),
            vec!["pathway suspected-cancer-wales needs a subtype from wales-primary-cancer-site"]
        );
        let other_list = Choice {
            subtype: Some(("generic-dementia-type", "03")),
            ..Choice::default()
        };
        assert_eq!(
            problems(v2, &other_list),
            vec![
                "pathway suspected-cancer-wales takes subtypes from wales-primary-cancer-site, not generic-dementia-type"
            ]
        );
        let unknown = Choice {
            subtype: Some(("wales-primary-cancer-site", "99")),
            ..Choice::default()
        };
        assert_eq!(
            problems(v2, &unknown),
            vec!["wales-primary-cancer-site has no code 99"]
        );
        let sites = &v2.subtypes.as_ref().unwrap().codes;
        assert_eq!(sites.len(), 15, "the NHS Wales list, 01 to 14 and 98");
        let dementia = pathway("dementia", 2).unwrap();
        let declined = Choice {
            agreed: Some(false),
            ..Choice::default()
        };
        assert!(problems(dementia, &declined).is_empty(), "optional there");
        let start = at("2026-11-09T09:00:00Z");
        assert_eq!(
            plan(v2, &lower_gi, start, start, UTC).unwrap().targets,
            plan(v1, &Choice::default(), start, start, UTC)
                .unwrap()
                .targets,
            "the site changes no date"
        );
    }

    #[test]
    fn a_bad_catalogue_is_refused() {
        let head = |v: i32, extra: &str| {
            format!(
                "[[pathway]]\nid = \"x\"\nversion = {v}\ntitle = \"T\"\nsources = [\"S\"]\napproved_by = \"\"\napproved_on = \"\"\nclock_from = \"point-of-suspicion\"\n{extra}"
            )
        };
        let target = "[[pathway.target]]\nid = \"t\"\ntext = \"Seen\"\ndays = 14\n";
        let ok = head(1, "priority = \"urgent\"\nno_target_reason = \"\"\n") + target;
        assert!(parse(&ok).is_ok(), "{ok}");
        for (text, words) in [
            (
                head(2, "priority = \"urgent\"\nno_target_reason = \"\"\n") + target,
                "should be version 1",
            ),
            (
                head(1, "priority = \"urgent\"\nno_target_reason = \"\"\n"),
                "no_target_reason",
            ),
            (
                head(1, "priority = \"urgent\"\nno_target_reason = \"none\"\n") + target,
                "no_target_reason",
            ),
            (head(1, "no_target_reason = \"\"\n") + target, "not both"),
            (
                head(1, "priority = \"urgent\"\nno_target_reason = \"\"\n")
                    + &target.replace("14", "400"),
                "0 to 365 days",
            ),
            (
                ok.clone()
                    + "[pathway.subtypes]\nscheme = \"s\"\ntitle = \"T\"\nsource = \"\"\nrequired = false\n[[pathway.subtypes.code]]\ncode = \"a\"\nname = \"A\"\n",
                "subtypes need",
            ),
            (
                ok.clone()
                    + "[pathway.subtypes]\nscheme = \"s\"\ntitle = \"T\"\nsource = \"S\"\nrequired = false\n[[pathway.subtypes.code]]\ncode = \"a\"\nname = \"A\"\n[[pathway.subtypes.code]]\ncode = \"a\"\nname = \"B\"\n",
                "repeated subtype",
            ),
        ] {
            let err = parse(&text).expect_err(&text);
            assert!(err.contains(words), "{text}: {err}");
        }
    }
}
