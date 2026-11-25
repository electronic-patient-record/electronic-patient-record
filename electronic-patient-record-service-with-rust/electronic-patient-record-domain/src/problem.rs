//! The problem list (EH-1, `spec/problem-list/index.md`): the rules, with no database.
//!
//! A problem is a condition, diagnosis or complaint that a clinician keeps on the patient's running list. It follows
//! three ideas from the openEHR Reference Model (`spec/lessons/openehr-rust/index.md`): it is an opinion held at the
//! time of recording, so a change of mind is a new version and never an edit; it belongs to the **patient**, not to a
//! visit, so it carries no encounter; and its **certainty** is recorded apart from whether it is current, so a
//! suspected diagnosis cannot be read as a confirmed one.
//!
//! A problem may name another as its cause ([`ProblemRecord::due_to`]), so that one condition with effects in several
//! organs reads as one cause with its manifestations. A cause must be a standing problem of the same patient, and no
//! problem may be, through any chain, its own cause.
//!
//! An empty list means *not recorded*, never *none*. Nothing here says "no known problems".

use crate::coding::{self, Coding};
use crate::dates;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;

/// The longest title kept.
pub const TITLE_LIMIT: usize = 200;
/// The longest body site kept.
pub const SITE_LIMIT: usize = 200;

/// Whether the problem is current.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Current: it counts.
    Active,
    /// Signs have gone but it may return.
    Remission,
    /// Not current, and not known to be resolved.
    Inactive,
    /// Over.
    Resolved,
    /// Recorded by mistake. Final: it cannot be reopened.
    EnteredInError,
}

impl Status {
    /// Every status (the stored words are checked against the database `CHECK`, PR-3).
    pub const ALL: [Status; 5] = [
        Status::Active,
        Status::Remission,
        Status::Inactive,
        Status::Resolved,
        Status::EnteredInError,
    ];

    /// The stored word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Remission => "remission",
            Status::Inactive => "inactive",
            Status::Resolved => "resolved",
            Status::EnteredInError => "entered-in-error",
        }
    }

    /// The status a stored word names, or `None` for one the service does not write.
    #[must_use]
    pub fn parse(s: &str) -> Option<Status> {
        Status::ALL.into_iter().find(|v| v.as_str() == s)
    }
}

/// How sure the author was. Required: it is never assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Certainty {
    /// Reported or noticed, with no view yet on whether it is right.
    Unconfirmed,
    /// A working diagnosis.
    Provisional,
    /// One of several that would explain the picture.
    Differential,
    /// Established.
    Confirmed,
    /// Ruled out. Kept, so that it is not looked for again without knowing.
    Refuted,
}

impl Certainty {
    /// Every certainty.
    pub const ALL: [Certainty; 5] = [
        Certainty::Unconfirmed,
        Certainty::Provisional,
        Certainty::Differential,
        Certainty::Confirmed,
        Certainty::Refuted,
    ];
}

/// How severe. Absent means not stated, never mild.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Mild.
    Mild,
    /// Moderate.
    Moderate,
    /// Severe.
    Severe,
}

impl Severity {
    /// Every severity.
    pub const ALL: [Severity; 3] = [Severity::Mild, Severity::Moderate, Severity::Severe];
}

/// One problem as recorded. There is no free-text note: a note has its own author, time and signature (CN-1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProblemRecord {
    /// The problem in words. Required: a code alone is not readable at the bedside.
    pub title: String,
    /// The same, coded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coding: Option<Coding>,
    /// How sure the author was.
    pub certainty: Certainty,
    /// Current or not.
    pub status: Status,
    /// How severe, if stated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<Severity>,
    /// Where in the body, in words, if it matters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_site: Option<String>,
    /// When it started: `YYYY`, `YYYY-MM` or `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onset: Option<String>,
    /// When it ended or went into remission, the same way. Not for an active problem.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abated: Option<String>,
    /// The problem (its id) that causes this one, on the same patient's list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_to: Option<String>,
}

/// A line of words up to `limit` characters, without control characters.
fn line(out: &mut Vec<String>, what: &str, value: &str, limit: usize) {
    if value.trim().is_empty() {
        out.push(format!("the {what} must not be blank"));
    } else if value.chars().count() > limit || value.chars().any(char::is_control) {
        out.push(format!(
            "the {what} must be up to {limit} characters, without control characters"
        ));
    }
}

/// A partial date that is real and not after `today`.
fn day(out: &mut Vec<String>, what: &str, value: &str, today: jiff::civil::Date) -> bool {
    if !dates::valid_birth_date(value) {
        out.push(format!("{what} must be YYYY, YYYY-MM or YYYY-MM-DD"));
        return false;
    }
    if dates::partial_order(value, &today.to_string()) == Some(Ordering::Greater) {
        out.push(format!("{what} is in the future"));
        return false;
    }
    true
}

/// What is wrong with a record on its own, in words that name a field and never quote it; empty if nothing. `today` is
/// the site's day. The cause is checked against the rest of the list by [`cause_problem`].
#[must_use]
pub fn validate(r: &ProblemRecord, today: jiff::civil::Date) -> Vec<String> {
    let mut out = Vec::new();
    line(&mut out, "title", &r.title, TITLE_LIMIT);
    if let Some(c) = &r.coding {
        out.extend(coding::problems(c));
    }
    if let Some(site) = &r.body_site {
        line(&mut out, "body site", site, SITE_LIMIT);
    }
    let onset = r
        .onset
        .as_deref()
        .is_some_and(|d| day(&mut out, "onset", d, today));
    let abated = r
        .abated
        .as_deref()
        .is_some_and(|d| day(&mut out, "abated", d, today));
    if let (true, true, Some(from), Some(to)) = (onset, abated, &r.onset, &r.abated)
        && dates::partial_order(to, from) == Some(Ordering::Less)
    {
        // Not established (2020 against 2020-05) is accepted: only a certain contradiction is refused.
        out.push("abated is before onset".into());
    }
    if r.status == Status::Active && r.abated.is_some() {
        out.push("an active problem cannot have an abated date".into());
    }
    if r.certainty == Certainty::Refuted && matches!(r.status, Status::Active | Status::Remission) {
        out.push("a refuted problem cannot be active or in remission".into());
    }
    if r.due_to
        .as_deref()
        .is_some_and(|id| crate::ids::uuid(id).is_none())
    {
        out.push("due_to must be the id of a problem".into());
    }
    out
}

/// What the cause rules need to know about a problem on the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The problem.
    pub id: String,
    /// Whether it stands (it is not entered in error).
    pub standing: bool,
    /// Whether it is thought not to be so ([`Certainty::Refuted`]).
    pub refuted: bool,
    /// The cause it names.
    pub due_to: Option<String>,
}

/// Why `cause` cannot be the cause of the problem `id` (`None` for one not recorded yet), or `None` if it can. `list` is
/// the current version of every problem of the patient. Ids are compared without regard to case.
#[must_use]
pub fn cause_problem(id: Option<&str>, cause: &str, list: &[Link]) -> Option<&'static str> {
    let find = |wanted: &str| list.iter().find(|l| l.id.eq_ignore_ascii_case(wanted));
    if id.is_some_and(|own| own.eq_ignore_ascii_case(cause)) {
        return Some("a problem cannot be its own cause");
    }
    match find(cause) {
        None => return Some("due_to names no problem on this patient's list"),
        Some(l) if !l.standing => return Some("due_to names a problem entered in error"),
        Some(_) => {}
    }
    // Follow the causes upward. `seen` ends the walk if a loop is already stored.
    let mut seen = BTreeSet::new();
    let mut at = Some(cause.to_ascii_lowercase());
    while let Some(current) = at {
        if id.is_some_and(|own| own.eq_ignore_ascii_case(&current)) {
            return Some("due_to would make a problem, through others, its own cause");
        }
        if !seen.insert(current.clone()) {
            break;
        }
        at = find(&current)
            .and_then(|l| l.due_to.as_deref())
            .map(str::to_ascii_lowercase);
    }
    None
}

/// How many standing problems name `id` as their cause. One that others name cannot be marked entered in error: they
/// would point at something that never happened.
#[must_use]
pub fn named_as_cause(id: &str, list: &[Link]) -> usize {
    list.iter()
        .filter(|l| {
            l.standing
                && !l.id.eq_ignore_ascii_case(id)
                && l.due_to
                    .as_deref()
                    .is_some_and(|c| c.eq_ignore_ascii_case(id))
        })
        .count()
}

/// Problems that share one cause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CauseGroup {
    /// The problem at the top: it names no cause that is on the list.
    pub cause: String,
    /// Every problem that comes from it, directly or through others, in the list's order.
    pub effects: Vec<String>,
}

/// How the problems on a list are joined by their causes. It is only what people recorded: nothing is worked out from a
/// code or a title, and a problem in no group has no recorded link, which is not the same as no link.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Causes {
    /// One for each problem that causes others and is not itself caused by one on the list.
    pub groups: Vec<CauseGroup>,
    /// Problems whose cause does not stand on this list (two records were unmerged after it was named).
    pub cause_not_on_list: Vec<String>,
    /// Problems whose own cause is refuted: the reason recorded for them may no longer hold.
    pub cause_refuted: Vec<String>,
}

/// The causes on a list. `list` is the current version of every problem of the patient, in the order shown. Entered in
/// error is left out. Ids are compared without regard to case.
#[must_use]
pub fn causes(list: &[Link]) -> Causes {
    let find = |wanted: &str| {
        list.iter()
            .find(|l| l.standing && l.id.eq_ignore_ascii_case(wanted))
    };
    let mut out = Causes::default();
    for effect in list.iter().filter(|l| l.standing) {
        let Some(named) = effect.due_to.as_deref() else {
            continue;
        };
        let Some(cause) = find(named) else {
            out.cause_not_on_list.push(effect.id.clone());
            continue;
        };
        if cause.refuted {
            out.cause_refuted.push(effect.id.clone());
        }
        // To the top. `seen` ends the walk if a loop is stored, which the rules refuse.
        let mut seen = BTreeSet::from([effect.id.to_ascii_lowercase()]);
        let mut top = cause;
        while seen.insert(top.id.to_ascii_lowercase()) {
            match top.due_to.as_deref().and_then(find) {
                Some(above) => top = above,
                None => break,
            }
        }
        if top.id == effect.id {
            continue;
        }
        match out.groups.iter_mut().find(|g| g.cause == top.id) {
            Some(group) => group.effects.push(effect.id.clone()),
            None => out.groups.push(CauseGroup {
                cause: top.id.clone(),
                effects: vec![effect.id.clone()],
            }),
        }
    }
    out
}

/// What the list amounts to. There is no "none": an empty list is unknown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Summary {
    /// At least one active problem.
    Active {
        /// How many are active and confirmed.
        confirmed: usize,
        /// How many are active and not confirmed.
        unconfirmed: usize,
    },
    /// Problems are recorded and none is active.
    NoneActive {
        /// How many are recorded (entered in error left out).
        recorded: usize,
    },
    /// Nothing is recorded. Unknown, not none.
    NotRecorded,
}

/// The summary of the current versions. Entered-in-error ones are left out: they never happened.
#[must_use]
pub fn summarise<'a>(records: impl IntoIterator<Item = &'a ProblemRecord>) -> Summary {
    let standing: Vec<&ProblemRecord> = records
        .into_iter()
        .filter(|r| r.status != Status::EnteredInError)
        .collect();
    let active: Vec<&ProblemRecord> = standing
        .iter()
        .copied()
        .filter(|r| r.status == Status::Active)
        .collect();
    if standing.is_empty() {
        Summary::NotRecorded
    } else if active.is_empty() {
        Summary::NoneActive {
            recorded: standing.len(),
        }
    } else {
        let sure = |r: &&&ProblemRecord| r.certainty == Certainty::Confirmed;
        Summary::Active {
            confirmed: active.iter().filter(sure).count(),
            unconfirmed: active.iter().filter(|r| !sure(r)).count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> jiff::civil::Date {
        crate::clock::parse_day("2026-11-24").unwrap()
    }

    fn rec(status: Status, certainty: Certainty) -> ProblemRecord {
        ProblemRecord {
            title: "Hypothyroidism".into(),
            coding: None,
            certainty,
            status,
            severity: None,
            body_site: None,
            onset: None,
            abated: None,
            due_to: None,
        }
    }

    fn link(id: &str, standing: bool, due_to: Option<&str>) -> Link {
        Link {
            id: id.into(),
            standing,
            refuted: false,
            due_to: due_to.map(String::from),
        }
    }

    #[test]
    fn problems_are_grouped_under_the_top_cause_they_come_from() {
        let groups = |list: &[Link]| {
            causes(list)
                .groups
                .into_iter()
                .map(|g| (g.cause, g.effects.join(" ")))
                .collect::<Vec<_>>()
        };
        // a <- b <- c, a <- d (named in capitals), e alone, f -> g apart.
        let list = [
            link("c", true, Some("b")),
            link("a", true, None),
            link("b", true, Some("a")),
            link("d", true, Some("A")),
            link("e", true, None),
            link("f", true, Some("g")),
            link("g", true, None),
        ];
        assert_eq!(
            groups(&list),
            [("a".into(), "c b d".into()), ("g".into(), "f".into())]
        );
        assert_eq!(causes(&[]), Causes::default());
        assert_eq!(causes(&[link("e", true, None)]), Causes::default());
        // One entered in error is in no group.
        assert_eq!(
            groups(&[link("a", true, None), link("b", false, Some("a"))]),
            []
        );
        // A loop that should not be there ends.
        let loops = causes(&[
            link("a", true, Some("b")),
            link("b", true, Some("a")),
            link("c", true, Some("a")),
        ]);
        assert!(loops.cause_not_on_list.is_empty());
        assert!(
            loops.groups.iter().all(|g| !g.effects.contains(&g.cause)),
            "{loops:?}"
        );
    }

    #[test]
    fn a_cause_that_is_missing_or_refuted_is_said() {
        let gone = causes(&[
            link("b", true, Some("a")),
            link("c", true, Some("b")),
            link("x", false, None),
            link("y", true, Some("x")),
        ]);
        assert_eq!(gone.cause_not_on_list, ["b", "y"]);
        // c's own cause is there, and it is the top of what can be seen.
        assert_eq!(gone.groups.len(), 1);
        assert_eq!(gone.groups[0].cause, "b");
        let refuted = Link {
            refuted: true,
            ..link("a", true, None)
        };
        let list = [
            refuted,
            link("b", true, Some("a")),
            link("c", true, Some("b")),
        ];
        assert_eq!(causes(&list).cause_refuted, ["b"], "only its own effects");
        assert_eq!(causes(&list).groups[0].effects, ["b", "c"]);
    }

    #[test]
    fn the_words_round_trip_and_an_unknown_one_is_refused() {
        for s in Status::ALL {
            assert_eq!(Status::parse(s.as_str()), Some(s));
            assert_eq!(serde_json::to_value(s).unwrap(), s.as_str());
        }
        assert_eq!(Status::parse("cured"), None);
    }

    #[test]
    fn certainty_is_never_assumed() {
        let missing = serde_json::json!({ "title": "x", "status": "active" });
        assert!(serde_json::from_value::<ProblemRecord>(missing).is_err());
        let extra = serde_json::json!({
            "title": "x", "status": "active", "certainty": "confirmed", "encounter": "e"
        });
        assert!(
            serde_json::from_value::<ProblemRecord>(extra).is_err(),
            "a problem belongs to the patient, not a visit"
        );
    }

    #[test]
    fn a_plain_record_passes_and_each_fault_is_named() {
        let good = rec(Status::Active, Certainty::Confirmed);
        assert!(validate(&good, today()).is_empty());
        let one = |edit: fn(&mut ProblemRecord), says: &str| {
            let mut r = rec(Status::Active, Certainty::Confirmed);
            edit(&mut r);
            let found = validate(&r, today());
            assert_eq!(found.len(), 1, "{says}: {found:?}");
            assert!(found[0].contains(says), "{found:?}");
        };
        one(|r| r.title = " ".into(), "title must not be blank");
        one(
            |r| r.title = "x".repeat(TITLE_LIMIT + 1),
            "title must be up to",
        );
        one(|r| r.title = "a\u{7}b".into(), "title must be up to");
        one(
            |r| r.body_site = Some(String::new()),
            "body site must not be blank",
        );
        one(|r| r.onset = Some("last year".into()), "onset must be");
        one(|r| r.onset = Some("2027".into()), "onset is in the future");
        one(
            |r| r.onset = Some("2026-11-25".into()),
            "onset is in the future",
        );
        one(
            |r| r.due_to = Some("the first one".into()),
            "due_to must be the id",
        );
        one(
            |r| r.abated = Some("2020".into()),
            "an active problem cannot have an abated date",
        );
        one(
            |r| r.certainty = Certainty::Refuted,
            "a refuted problem cannot be active",
        );
        one(
            |r| {
                r.coding = Some(Coding {
                    system: "nonsense".into(),
                    code: "1".into(),
                });
            },
            "code system",
        );
    }

    #[test]
    fn this_year_and_this_month_are_not_the_future() {
        for onset in ["2026", "2026-11", "2026-11-24"] {
            let mut r = rec(Status::Active, Certainty::Provisional);
            r.onset = Some(onset.into());
            assert!(validate(&r, today()).is_empty(), "{onset}");
        }
    }

    #[test]
    fn only_a_certain_contradiction_between_dates_is_refused() {
        let with = |onset: &str, abated: &str| {
            let mut r = rec(Status::Resolved, Certainty::Confirmed);
            r.onset = Some(onset.into());
            r.abated = Some(abated.into());
            validate(&r, today())
        };
        assert_eq!(with("2021", "2020"), ["abated is before onset"]);
        assert_eq!(with("2020-05-17", "2020-05-16"), ["abated is before onset"]);
        assert!(with("2020-05", "2020").is_empty(), "not established");
        assert!(with("2020-05-17", "2020-05-17").is_empty());
    }

    #[test]
    fn a_refuted_problem_may_be_kept_as_inactive_or_resolved() {
        for status in [Status::Inactive, Status::Resolved, Status::EnteredInError] {
            assert!(
                validate(&rec(status, Certainty::Refuted), today()).is_empty(),
                "{status:?}"
            );
        }
        assert_eq!(
            validate(&rec(Status::Remission, Certainty::Refuted), today()).len(),
            1
        );
    }

    #[test]
    fn a_cause_is_a_standing_problem_of_this_patient() {
        let list = [link("a", true, None), link("gone", false, None)];
        assert_eq!(cause_problem(None, "a", &list), None);
        assert_eq!(cause_problem(None, "A", &list), None, "case is ignored");
        assert!(
            cause_problem(None, "b", &list)
                .unwrap()
                .contains("no problem")
        );
        assert!(
            cause_problem(None, "gone", &list)
                .unwrap()
                .contains("entered in error")
        );
        assert!(
            cause_problem(Some("a"), "A", &list)
                .unwrap()
                .contains("its own cause")
        );
    }

    #[test]
    fn no_chain_of_causes_comes_back_to_its_start() {
        // lung -> syndrome; thyroid -> syndrome. The syndrome cannot then be due to the lung.
        let list = [
            link("syndrome", true, None),
            link("lung", true, Some("syndrome")),
            link("cough", true, Some("LUNG")),
        ];
        for cause in ["lung", "cough"] {
            assert!(
                cause_problem(Some("syndrome"), cause, &list)
                    .unwrap()
                    .contains("through others"),
                "{cause}"
            );
        }
        assert_eq!(cause_problem(Some("cough"), "syndrome", &list), None);
    }

    #[test]
    fn a_loop_already_stored_does_not_hang_the_check() {
        let list = [link("a", true, Some("b")), link("b", true, Some("a"))];
        assert_eq!(cause_problem(Some("c"), "a", &list), None);
    }

    #[test]
    fn what_is_named_as_a_cause_is_counted_without_the_entries_in_error() {
        let list = [
            link("a", true, Some("a")),
            link("b", true, Some("A")),
            link("c", false, Some("a")),
            link("d", true, None),
        ];
        assert_eq!(named_as_cause("a", &list), 1);
        assert_eq!(named_as_cause("d", &list), 0);
    }

    #[test]
    fn an_empty_list_is_unknown_and_never_none() {
        assert_eq!(summarise([]), Summary::NotRecorded);
        let mistaken = rec(Status::EnteredInError, Certainty::Confirmed);
        assert_eq!(summarise([&mistaken]), Summary::NotRecorded);
        assert_eq!(
            serde_json::to_value(summarise([])).unwrap(),
            serde_json::json!({ "state": "not-recorded" })
        );
    }

    #[test]
    fn the_summary_keeps_the_confirmed_apart_from_the_rest() {
        let all = [
            rec(Status::Active, Certainty::Confirmed),
            rec(Status::Active, Certainty::Differential),
            rec(Status::Active, Certainty::Unconfirmed),
            rec(Status::Resolved, Certainty::Confirmed),
        ];
        assert_eq!(
            summarise(&all),
            Summary::Active {
                confirmed: 1,
                unconfirmed: 2
            }
        );
        assert_eq!(summarise(&all[3..]), Summary::NoneActive { recorded: 1 });
    }
}
