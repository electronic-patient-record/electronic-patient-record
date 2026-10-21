//! The patient's consent, as data and as a decision (task SE-2).
//!
//! A consent record is a FHIR-shaped list of provisions. Each provision permits or denies a
//! disclosure to some actors (roles), for some purposes of use, of some kinds of record, carrying
//! some security labels, during some period. An empty list means "any". [`decide`] reduces the
//! active consents of one patient to the [`policy::Consent`] value the policy engine takes.
//!
//! The rules, none of which the patient or a clinician can loosen by how a provision is written:
//!
//! - **Deny beats permit.** One matching deny gives [`policy::Consent::Deny`] whatever else
//!   matches. A deny with a label list matches a record carrying *any* listed label; with none it
//!   matches every record.
//! - **A permit must cover every label on the record.** A permit with no labels covers only a
//!   record with none, so a general permit never unmasks substance-use content (42 CFR Part 2).
//! - **Only active consents count.** `inactive` (withdrawn, expired) and `entered-in-error` are
//!   ignored. A provision or consent outside its period is ignored.
//! - **Unreadable means deny.** A stored provision that no longer parses is treated as a deny of
//!   everything, so corrupt data can only withhold disclosure, never grant it.
//! - **Roles.** An actor list names roles. A subject holding several roles is refused when a deny
//!   names any one of them, which can over-refuse (a nurse-only deny also stops a nurse who is
//!   also a pharmacist) but never over-discloses.
//!
//! Not here: who may read or write consent (the policy table), storing it (`consents_store`), the
//! national data opt-out service itself (see [`Kind::NationalDataOptOut`]) and any bulk export.

use crate::policy::{self, Label, Purpose, Resource, Role};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The most provisions one consent may carry.
pub const MAX_PROVISIONS: usize = 50;

/// Whether a provision permits or denies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    /// The disclosure is permitted.
    Permit,
    /// The disclosure is refused.
    Deny,
}

/// The state of a consent record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// In force.
    Active,
    /// Withdrawn or expired. Kept for the history; ignored by [`decide`].
    Inactive,
    /// Recorded by mistake. Ignored by [`decide`].
    EnteredInError,
}

impl Status {
    /// The text stored in the `status` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Inactive => "inactive",
            Status::EnteredInError => "entered-in-error",
        }
    }
}

/// What kind of consent this is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// The record's own provisions apply.
    #[default]
    General,
    /// The patient's choice to opt out of secondary use. The server supplies the provision (deny
    /// for the `research` purpose); a record of this kind carries none of its own.
    ///
    /// This records the choice **held by this service only**. The national data opt-out is a
    /// national service checked against the NHS number; no such check is built.
    NationalDataOptOut,
}

/// A date range, both ends optional and inclusive, as `YYYY-MM-DD` (UTC).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Period {
    /// First day covered.
    #[serde(default)]
    pub start: Option<String>,
    /// Last day covered.
    #[serde(default)]
    pub end: Option<String>,
}

/// One permit or deny. An empty list means "any".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provision {
    /// Permit or deny.
    pub effect: Effect,
    /// Roles it applies to ([`Role::as_str`] values).
    #[serde(default)]
    pub actors: Vec<String>,
    /// Purposes of use it applies to ([`Purpose::code`] values).
    #[serde(default)]
    pub purposes: Vec<String>,
    /// Kinds of record it applies to ([`Resource::name`] values).
    #[serde(default)]
    pub resources: Vec<String>,
    /// Security labels it applies to ([`Label::code`] values).
    #[serde(default)]
    pub labels: Vec<String>,
    /// When it applies.
    #[serde(default)]
    pub period: Option<Period>,
}

/// A consent record: the body of one version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentRecord {
    /// General, or the national data opt-out.
    #[serde(default)]
    pub kind: Kind,
    /// In force, withdrawn, or entered in error.
    pub status: Status,
    /// The provisions. Required for a general consent; none for an opt-out.
    #[serde(default)]
    pub provisions: Vec<Provision>,
    /// When the consent as a whole applies.
    #[serde(default)]
    pub period: Option<Period>,
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's `days_from_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Unix seconds at 00:00 UTC on a real `YYYY-MM-DD` date, else `None`.
#[must_use]
pub fn day_start(date: &str) -> Option<i64> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 || !crate::patients_store::valid_birth_date(date) {
        return None;
    }
    let n = |s: &str| s.parse::<i64>().ok();
    Some(days_from_civil(n(parts[0])?, n(parts[1])?, n(parts[2])?) * 86_400)
}

/// Whether `now` (unix seconds) lies in the period. `None` if a date does not parse.
fn in_period(period: Option<&Period>, now: i64) -> Option<bool> {
    let Some(p) = period else {
        return Some(true);
    };
    if let Some(start) = &p.start
        && now < day_start(start)?
    {
        return Some(false);
    }
    if let Some(end) = &p.end
        && now >= day_start(end)? + 86_400
    {
        return Some(false);
    }
    Some(true)
}

/// Every problem with a record, or none if it may be stored.
#[must_use]
pub fn validate(record: &ConsentRecord) -> Vec<String> {
    let mut problems = Vec::new();
    match record.kind {
        Kind::General if record.provisions.is_empty() => {
            problems.push("a consent needs at least one provision".to_string());
        }
        Kind::NationalDataOptOut if !record.provisions.is_empty() => {
            problems.push(
                "a national data opt-out carries no provisions of its own: the server supplies it"
                    .to_string(),
            );
        }
        _ => {}
    }
    if record.provisions.len() > MAX_PROVISIONS {
        problems.push(format!("at most {MAX_PROVISIONS} provisions"));
    }
    let period = |label: &str, p: &Option<Period>, problems: &mut Vec<String>| {
        let Some(p) = p else { return };
        let start = p.start.as_deref().map(|d| (d, day_start(d)));
        let end = p.end.as_deref().map(|d| (d, day_start(d)));
        for (d, parsed) in start.iter().chain(end.iter()) {
            if parsed.is_none() {
                problems.push(format!("{label}: {d:?} is not a real YYYY-MM-DD date"));
            }
        }
        if let (Some((_, Some(a))), Some((_, Some(b)))) = (start, end)
            && a > b
        {
            problems.push(format!("{label}: the start is after the end"));
        }
    };
    period("period", &record.period, &mut problems);
    for (i, p) in record.provisions.iter().enumerate() {
        let at = format!("provision {}", i + 1);
        let unknown = |what: &str, values: &[String], known: &dyn Fn(&str) -> bool| {
            values
                .iter()
                .filter(|v| !known(v))
                .map(|v| format!("{at}: unknown {what} {v:?}"))
                .collect::<Vec<_>>()
        };
        problems.extend(unknown("actor", &p.actors, &|v| Role::parse(v).is_some()));
        problems.extend(unknown("purpose", &p.purposes, &|v| {
            Purpose::parse(v).is_some()
        }));
        problems.extend(unknown("resource", &p.resources, &|v| {
            Resource::parse(v).is_some()
        }));
        problems.extend(unknown("label", &p.labels, &|v| Label::parse(v).is_some()));
        period(&at, &p.period, &mut problems);
    }
    problems
}

/// The provisions that apply: the record's own, or the opt-out's fixed deny for research.
fn effective(record: &ConsentRecord) -> Vec<Provision> {
    match record.kind {
        Kind::General => record.provisions.clone(),
        Kind::NationalDataOptOut => vec![Provision {
            effect: Effect::Deny,
            actors: vec![],
            purposes: vec![Purpose::Research.code().to_string()],
            resources: vec![],
            labels: vec![],
            period: None,
        }],
    }
}

/// What a disclosure is, for matching against provisions.
#[derive(Debug, Clone)]
pub struct Context {
    /// The subject's roles.
    pub roles: BTreeSet<Role>,
    /// The declared purpose of use.
    pub purpose: Purpose,
    /// The kind of record.
    pub resource: Resource,
    /// The record's security labels.
    pub labels: BTreeSet<Label>,
    /// Now, in unix seconds.
    pub now: i64,
}

/// The reduced consent, and which consent records decided it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// What to give the policy engine.
    pub consent: policy::Consent,
    /// Ids of the consent records whose provisions produced a deny or a permit. Empty when
    /// nothing was recorded. Ids only: provisions are not copied into audit rows.
    pub decided_by: Vec<String>,
}

/// Whether one provision matches the disclosure. `None` when the provision does not parse.
fn matches(p: &Provision, ctx: &Context) -> Option<bool> {
    if !in_period(p.period.as_ref(), ctx.now)? {
        return Some(false);
    }
    let actors = p
        .actors
        .iter()
        .map(|a| Role::parse(a))
        .collect::<Option<Vec<_>>>()?;
    let purposes = p
        .purposes
        .iter()
        .map(|a| Purpose::parse(a))
        .collect::<Option<Vec<_>>>()?;
    let resources = p
        .resources
        .iter()
        .map(|a| Resource::parse(a))
        .collect::<Option<Vec<_>>>()?;
    let labels = p
        .labels
        .iter()
        .map(|a| Label::parse(a))
        .collect::<Option<BTreeSet<_>>>()?;
    let actor_ok = actors.is_empty() || actors.iter().any(|a| ctx.roles.contains(a));
    let purpose_ok = purposes.is_empty() || purposes.contains(&ctx.purpose);
    let resource_ok = resources.is_empty() || resources.contains(&ctx.resource);
    let label_ok = match p.effect {
        // A deny reaches a record carrying any listed label, or every record if none is listed.
        Effect::Deny => labels.is_empty() || ctx.labels.iter().any(|l| labels.contains(l)),
        // A permit must name every label the record carries.
        Effect::Permit => ctx.labels.is_subset(&labels),
    };
    Some(actor_ok && purpose_ok && resource_ok && label_ok)
}

/// Reduce a patient's consent records to one value for this disclosure.
///
/// `records` are `(id, record)` pairs: the *current* version of each consent of the patient.
#[must_use]
pub fn decide<'a>(
    records: impl IntoIterator<Item = (&'a str, &'a ConsentRecord)>,
    ctx: &Context,
) -> Verdict {
    let mut denied_by = Vec::new();
    let mut permitted_by = Vec::new();
    for (id, record) in records {
        if record.status != Status::Active {
            continue;
        }
        // A whole-record period that does not parse, or has not begun or has ended.
        match in_period(record.period.as_ref(), ctx.now) {
            Some(true) => {}
            Some(false) => continue,
            None => {
                denied_by.push(id.to_string());
                continue;
            }
        }
        for p in effective(record) {
            match matches(&p, ctx) {
                Some(false) => {}
                Some(true) if p.effect == Effect::Deny => denied_by.push(id.to_string()),
                Some(true) => permitted_by.push(id.to_string()),
                // Unreadable: withhold.
                None => denied_by.push(id.to_string()),
            }
        }
    }
    let (consent, mut decided_by) = if !denied_by.is_empty() {
        (policy::Consent::Deny, denied_by)
    } else if !permitted_by.is_empty() {
        (policy::Consent::Permit, permitted_by)
    } else {
        (policy::Consent::NotRecorded, Vec::new())
    };
    decided_by.sort();
    decided_by.dedup();
    Verdict {
        consent,
        decided_by,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_790_000_000; // 2026-09-21

    fn ctx() -> Context {
        Context {
            roles: BTreeSet::from([Role::Clinician]),
            purpose: Purpose::Treatment,
            resource: Resource::PatientIdentity,
            labels: BTreeSet::new(),
            now: NOW,
        }
    }

    fn provision(effect: Effect) -> Provision {
        Provision {
            effect,
            actors: vec![],
            purposes: vec![],
            resources: vec![],
            labels: vec![],
            period: None,
        }
    }

    fn record(provisions: Vec<Provision>) -> ConsentRecord {
        ConsentRecord {
            kind: Kind::General,
            status: Status::Active,
            provisions,
            period: None,
        }
    }

    fn verdict(records: &[ConsentRecord], ctx: &Context) -> policy::Consent {
        let ids: Vec<String> = (0..records.len()).map(|i| format!("c{i}")).collect();
        decide(ids.iter().map(String::as_str).zip(records), ctx).consent
    }

    #[test]
    fn dates_convert_to_unix_seconds() {
        assert_eq!(day_start("1970-01-01"), Some(0));
        assert_eq!(day_start("1970-01-02"), Some(86_400));
        assert_eq!(day_start("2000-03-01"), Some(951_868_800));
        assert_eq!(day_start("2024-02-29"), Some(1_709_164_800));
        assert_eq!(day_start("1969-12-31"), Some(-86_400));
        for bad in [
            "2023-02-29",
            "2026-13-01",
            "2026-1-1",
            "2026-05",
            "2026",
            "x",
            "",
        ] {
            assert_eq!(day_start(bad), None, "{bad}");
        }
    }

    #[test]
    fn nothing_recorded_means_not_recorded() {
        assert_eq!(verdict(&[], &ctx()), policy::Consent::NotRecorded);
        // A provision that does not match is nothing either.
        let mut p = provision(Effect::Deny);
        p.purposes = vec!["research".into()];
        assert_eq!(
            verdict(&[record(vec![p])], &ctx()),
            policy::Consent::NotRecorded
        );
    }

    #[test]
    fn a_deny_with_no_conditions_refuses_everyone() {
        let r = record(vec![provision(Effect::Deny)]);
        assert_eq!(verdict(&[r], &ctx()), policy::Consent::Deny);
    }

    #[test]
    fn a_deny_is_scoped_by_actor_purpose_and_resource() {
        let mut p = provision(Effect::Deny);
        p.actors = vec!["nurse".into()];
        p.purposes = vec!["treatment".into()];
        p.resources = vec!["PatientIdentity".into()];
        let r = vec![record(vec![p])];
        // A clinician is not the named actor.
        assert_eq!(verdict(&r, &ctx()), policy::Consent::NotRecorded);
        let mut nurse = ctx();
        nurse.roles = BTreeSet::from([Role::Nurse]);
        assert_eq!(verdict(&r, &nurse), policy::Consent::Deny);
        nurse.purpose = Purpose::Operations;
        assert_eq!(verdict(&r, &nurse), policy::Consent::NotRecorded);
        nurse.purpose = Purpose::Treatment;
        nurse.resource = Resource::Condition;
        assert_eq!(verdict(&r, &nurse), policy::Consent::NotRecorded);
    }

    #[test]
    fn a_deny_for_one_role_also_stops_a_subject_who_holds_it_among_others() {
        let mut p = provision(Effect::Deny);
        p.actors = vec!["nurse".into()];
        let mut both = ctx();
        both.roles = BTreeSet::from([Role::Nurse, Role::Pharmacist]);
        assert_eq!(verdict(&[record(vec![p])], &both), policy::Consent::Deny);
    }

    #[test]
    fn deny_beats_permit_in_any_order_and_across_records() {
        let permit = record(vec![provision(Effect::Permit)]);
        let deny = record(vec![provision(Effect::Deny)]);
        assert_eq!(
            verdict(&[permit.clone(), deny.clone()], &ctx()),
            policy::Consent::Deny
        );
        assert_eq!(
            verdict(&[deny, permit.clone()], &ctx()),
            policy::Consent::Deny
        );
        assert_eq!(verdict(&[permit], &ctx()), policy::Consent::Permit);
    }

    #[test]
    fn a_general_permit_does_not_cover_a_labelled_record_but_a_named_one_does() {
        let mut sud = ctx();
        sud.labels = BTreeSet::from([Label::SubstanceUse]);
        let general = record(vec![provision(Effect::Permit)]);
        assert_eq!(verdict(&[general], &sud), policy::Consent::NotRecorded);

        let mut named = provision(Effect::Permit);
        named.labels = vec!["SUD".into()];
        let named = record(vec![named]);
        assert_eq!(
            verdict(std::slice::from_ref(&named), &sud),
            policy::Consent::Permit
        );
        // Named for SUD, but the record also carries HIV: not covered.
        sud.labels.insert(Label::Hiv);
        assert_eq!(
            verdict(std::slice::from_ref(&named), &sud),
            policy::Consent::NotRecorded
        );
        // A permit that names labels still covers an unlabelled record.
        assert_eq!(verdict(&[named], &ctx()), policy::Consent::Permit);
    }

    #[test]
    fn a_labelled_deny_reaches_a_record_with_any_of_those_labels() {
        let mut d = provision(Effect::Deny);
        d.labels = vec!["HIV".into(), "PSY".into()];
        let r = vec![record(vec![d])];
        assert_eq!(verdict(&r, &ctx()), policy::Consent::NotRecorded);
        let mut c = ctx();
        c.labels = BTreeSet::from([Label::Psychiatry, Label::Ethnicity]);
        assert_eq!(verdict(&r, &c), policy::Consent::Deny);
        c.labels = BTreeSet::from([Label::Ethnicity]);
        assert_eq!(verdict(&r, &c), policy::Consent::NotRecorded);
    }

    #[test]
    fn only_active_records_count() {
        for status in [Status::Inactive, Status::EnteredInError] {
            let mut r = record(vec![provision(Effect::Deny)]);
            r.status = status;
            assert_eq!(
                verdict(&[r], &ctx()),
                policy::Consent::NotRecorded,
                "{status:?}"
            );
        }
    }

    #[test]
    fn periods_are_inclusive_of_both_days() {
        let day = |s: &str| day_start(s).unwrap();
        let mut p = provision(Effect::Deny);
        p.period = Some(Period {
            start: Some("2026-09-21".into()),
            end: Some("2026-09-22".into()),
        });
        let r = vec![record(vec![p])];
        let at = |now| Context { now, ..ctx() };
        assert_eq!(
            verdict(&r, &at(day("2026-09-21") - 1)),
            policy::Consent::NotRecorded
        );
        assert_eq!(verdict(&r, &at(day("2026-09-21"))), policy::Consent::Deny);
        assert_eq!(
            verdict(&r, &at(day("2026-09-22") + 86_399)),
            policy::Consent::Deny
        );
        assert_eq!(
            verdict(&r, &at(day("2026-09-23"))),
            policy::Consent::NotRecorded
        );

        // The same bounds on the whole record.
        let mut whole = record(vec![provision(Effect::Deny)]);
        whole.period = Some(Period {
            start: None,
            end: Some("2026-09-20".into()),
        });
        assert_eq!(
            verdict(&[whole], &at(day("2026-09-21"))),
            policy::Consent::NotRecorded
        );
    }

    #[test]
    fn something_unreadable_withholds() {
        let mut p = provision(Effect::Permit);
        p.actors = vec!["wizard".into()];
        assert_eq!(verdict(&[record(vec![p])], &ctx()), policy::Consent::Deny);
        let mut p = provision(Effect::Permit);
        p.period = Some(Period {
            start: Some("not-a-date".into()),
            end: None,
        });
        assert_eq!(verdict(&[record(vec![p])], &ctx()), policy::Consent::Deny);
        let mut whole = record(vec![provision(Effect::Permit)]);
        whole.period = Some(Period {
            start: None,
            end: Some("2026-02-30".into()),
        });
        assert_eq!(verdict(&[whole], &ctx()), policy::Consent::Deny);
    }

    #[test]
    fn the_opt_out_refuses_research_and_nothing_else() {
        let optout = ConsentRecord {
            kind: Kind::NationalDataOptOut,
            status: Status::Active,
            provisions: vec![],
            period: None,
        };
        let mut research = ctx();
        research.purpose = Purpose::Research;
        assert_eq!(
            verdict(std::slice::from_ref(&optout), &research),
            policy::Consent::Deny
        );
        assert_eq!(
            verdict(std::slice::from_ref(&optout), &ctx()),
            policy::Consent::NotRecorded
        );
        // Withdrawn, it no longer refuses.
        let mut withdrawn = optout;
        withdrawn.status = Status::Inactive;
        assert_eq!(
            verdict(&[withdrawn], &research),
            policy::Consent::NotRecorded
        );
    }

    #[test]
    fn the_verdict_names_the_deciding_records_once_each() {
        let deny = record(vec![provision(Effect::Deny), provision(Effect::Deny)]);
        let permit = record(vec![provision(Effect::Permit)]);
        let v = decide([("b", &deny), ("a", &permit)], &ctx());
        assert_eq!(v.consent, policy::Consent::Deny);
        assert_eq!(v.decided_by, vec!["b".to_string()]);
        let v = decide([("a", &permit)], &ctx());
        assert_eq!(v.decided_by, vec!["a".to_string()]);
        assert!(decide([], &ctx()).decided_by.is_empty());
    }

    #[test]
    fn validation_reports_every_problem() {
        assert!(validate(&record(vec![provision(Effect::Deny)])).is_empty());
        assert_eq!(
            validate(&record(vec![])),
            ["a consent needs at least one provision"]
        );

        let mut bad = provision(Effect::Permit);
        bad.actors = vec!["wizard".into()];
        bad.purposes = vec!["fun".into()];
        bad.resources = vec!["Spleen".into()];
        bad.labels = vec!["XYZ".into()];
        bad.period = Some(Period {
            start: Some("2026-10-02".into()),
            end: Some("2026-10-01".into()),
        });
        let problems = validate(&record(vec![bad]));
        assert_eq!(problems.len(), 5, "{problems:?}");

        let mut dated = record(vec![provision(Effect::Deny)]);
        dated.period = Some(Period {
            start: Some("2026-02-30".into()),
            end: None,
        });
        assert_eq!(validate(&dated).len(), 1);

        let many = record(vec![provision(Effect::Deny); MAX_PROVISIONS + 1]);
        assert_eq!(validate(&many).len(), 1);
    }

    #[test]
    fn an_opt_out_must_not_bring_its_own_provisions() {
        let mut r = record(vec![]);
        r.kind = Kind::NationalDataOptOut;
        assert!(validate(&r).is_empty());
        r.provisions = vec![provision(Effect::Permit)];
        assert_eq!(validate(&r).len(), 1);
    }

    #[test]
    fn the_json_shape_round_trips_and_defaults_are_empty() {
        let r: ConsentRecord = serde_json::from_str(
            r#"{"status":"active","provisions":[{"effect":"deny","purposes":["research"]}]}"#,
        )
        .unwrap();
        assert_eq!(r.kind, Kind::General);
        assert_eq!(r.provisions[0].actors, Vec::<String>::new());
        let back: ConsentRecord =
            serde_json::from_value(serde_json::to_value(&r).unwrap()).unwrap();
        assert_eq!(back, r);
        assert!(
            serde_json::from_str::<ConsentRecord>(
                r#"{"status":"active","provisions":[{"effect":"maybe"}]}"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<ConsentRecord>(r#"{"status":"paused"}"#).is_err());
        assert_eq!(
            serde_json::to_value(Status::EnteredInError).unwrap(),
            "entered-in-error"
        );
        assert_eq!(Status::EnteredInError.as_str(), "entered-in-error");
    }

    /// The engine, given this module's verdict: a deny stops a clinician; a general permit does
    /// not unmask substance-use content, a named one does.
    #[test]
    fn the_verdict_drives_the_policy_engine() {
        let request = |consent, labels: &[Label]| policy::Request {
            subject: policy::Subject {
                roles: BTreeSet::from([Role::Clinician]),
            },
            resource: Resource::Condition,
            action: policy::Action::Read,
            labels: labels.iter().copied().collect(),
            relationship: policy::Relationship::CareTeam,
            purpose: Purpose::Treatment,
            consent,
            break_glass: None,
            now: NOW,
        };
        let c = Context {
            resource: Resource::Condition,
            ..ctx()
        };
        let deny = record(vec![provision(Effect::Deny)]);
        let v = verdict(&[deny], &c);
        assert!(matches!(
            policy::evaluate(&request(v, &[])),
            policy::Decision::Deny {
                reason: policy::DenyReason::ConsentDenied,
                ..
            }
        ));

        let mut sud_ctx = c.clone();
        sud_ctx.labels = BTreeSet::from([Label::SubstanceUse]);
        let masked = |d: policy::Decision| match d {
            policy::Decision::Allow(o) => o.iter().any(
                |o| matches!(o, policy::Obligation::Mask(l) if l.contains(&Label::SubstanceUse)),
            ),
            policy::Decision::Deny { .. } => panic!("expected an allow"),
        };
        let general = verdict(&[record(vec![provision(Effect::Permit)])], &sud_ctx);
        assert!(masked(policy::evaluate(&request(
            general,
            &[Label::SubstanceUse]
        ))));
        let mut named = provision(Effect::Permit);
        named.labels = vec!["SUD".into()];
        let named = verdict(&[record(vec![named])], &sud_ctx);
        assert!(!masked(policy::evaluate(&request(
            named,
            &[Label::SubstanceUse]
        ))));
    }
}
