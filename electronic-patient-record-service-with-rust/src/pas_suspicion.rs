//! What a suspicion becomes in the PAS (RS-5, `spec/referral-suspicion/index.md`): the request bodies,
//! or why a person must look first. Pure: no network, no database, no sender.
//!
//! The PAS (`~/git/patient-administration-system`, its spec 20 and WPAS-9) tracks a suspicion on its own
//! referral, which carries the EPR referral id as `external_ref` under the source `epr`. This module
//! maps the EPR's three events onto the PAS's calls:
//!
//! - a suspicion raised: [`new_suspicion`], for `POST /api/referrals/{id}/suspicions`;
//! - an amendment: [`amendment`], for `POST /api/suspicions/{id}/amend` (with `If-Match`), or nothing
//!   when no field the PAS holds changed;
//! - an outcome: [`stop`], for `POST /api/suspicions/{id}/stop` (with `If-Match`): `ruled-out` is
//!   `downgraded`, `withdrawn` is `withdrawn`, and `confirmed` sends nothing, because the PAS clock runs
//!   on to the first definitive treatment.
//!
//! Every mapping is checked against the PAS's **live** profile (`GET /api/suspicion-profile`), never a
//! copy: the condition must be one it tracks, a subtype must be from that condition's list with the
//! list's scheme, and a required list must be met. A day sent (`suspected_on`, `stopped_on`) is the
//! day in the profile's `time_zone`, not a UTC day. The PAS takes neither the grounds nor whether the
//! patient was told, and needs named people, so a suspicion raised by a screening programme or a result
//! is held for a person to name who suspected it.
//!
//! A [`Hold`] names ids and codes, never the condition in words, the grounds or a person.

use crate::suspicion::{Outcome, OutcomeKind, Source, Suspicion};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The PAS's active profile, as `GET /api/suspicion-profile` answers; fields the mapping does not use
/// are ignored.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ActiveProfile {
    /// The profile's name (`generic`, `wales`, or an operator's own).
    pub name: String,
    /// The IANA time zone a day is counted in (`PAS_TIMEZONE`).
    pub time_zone: String,
    /// The conditions it tracks.
    pub conditions: Vec<ProfileCondition>,
}

/// One condition the PAS tracks.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProfileCondition {
    /// Its code (`cancer`, `dementia`, ...).
    pub code: String,
    /// Its subtype list, if it has one.
    #[serde(default)]
    pub subtypes: Option<ProfileSubtypes>,
}

/// A condition's subtype list in the PAS.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProfileSubtypes {
    /// The list's name, sent as `subtype_scheme`.
    pub scheme: String,
    /// Whether a suspicion of the condition must name one.
    #[serde(default)]
    pub required: bool,
    /// The codes.
    pub codes: Vec<ProfileCode>,
}

/// One code in a PAS subtype list.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProfileCode {
    /// The code.
    pub code: String,
}

/// Why a mapping is not sent: a person must look first. Words name ids and codes only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hold(pub String);

/// The body of `POST /api/referrals/{id}/suspicions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NewSuspicion {
    /// The PAS condition code.
    pub condition: String,
    /// The subtype code, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    /// The list it is from, sent whenever a subtype is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype_scheme: Option<String>,
    /// The point of suspicion's day in the profile's time zone, `YYYY-MM-DD`.
    pub suspected_on: String,
    /// The clinician who suspected it.
    pub suspected_by: String,
    /// The EPR user who recorded it.
    pub recorded_by: String,
}

/// The body of `POST /api/suspicions/{id}/amend`. A field left out is kept by the PAS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Amend {
    /// A new point of suspicion day.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspected_on: Option<String>,
    /// A new clinician who suspected it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspected_by: Option<String>,
    /// A new subtype code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    /// Its list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype_scheme: Option<String>,
    /// Why, as the EPR amendment gave it.
    pub reason: String,
    /// The EPR user who amended it.
    pub amended_by: String,
}

/// The body of `POST /api/suspicions/{id}/stop`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Stop {
    /// `downgraded` or `withdrawn`; the other PAS reasons are not the EPR's to send.
    pub reason: &'static str,
    /// The outcome's day in the profile's time zone.
    pub stopped_on: String,
    /// The EPR user who recorded the outcome.
    pub stopped_by: String,
}

/// A PAS subtype as its scheme and code.
pub type Implied = Option<(&'static str, &'static str)>;

/// The PAS condition for an EPR pathway, and a subtype the pathway itself implies (a child's type 1
/// diabetes). `None`: the pathway has no PAS condition.
#[must_use]
pub fn condition(pathway: &str) -> Option<(&'static str, Implied)> {
    Some(match pathway {
        "suspected-cancer-wales" | "suspected-cancer-england" => ("cancer", None),
        "type-1-diabetes-child" => ("diabetes", Some(("generic-diabetes-type", "type_1"))),
        "heart-failure" => ("cardiovascular_disease", None),
        "parkinsons" => ("parkinsons_disease", None),
        "dementia" => ("dementia", None),
        "obstructive-sleep-apnoea" => ("obstructive_sleep_apnoea", None),
        _ => return None,
    })
}

/// The day a time falls on in an IANA time zone, `YYYY-MM-DD`.
///
/// # Errors
///
/// A [`Hold`] when the zone is not an IANA name or the time is not RFC 3339.
pub fn day_in(time_zone: &str, time: &str) -> Result<String, Hold> {
    let tz: chrono_tz::Tz = time_zone
        .parse()
        .map_err(|_| Hold(format!("the PAS time zone {time_zone} is not an IANA name")))?;
    let t = DateTime::parse_from_rfc3339(time)
        .map_err(|_| Hold("a time to send is not RFC 3339".into()))?;
    Ok(t.with_timezone(&tz).format("%Y-%m-%d").to_string())
}

fn mapped(s: &Suspicion) -> Result<(&'static str, Implied), Hold> {
    condition(&s.pathway).ok_or_else(|| Hold(format!("pathway {} has no PAS condition", s.pathway)))
}

/// The subtype and scheme to send, checked against the live profile.
fn subtype(
    s: &Suspicion,
    code: &str,
    implied: Implied,
    p: &ActiveProfile,
) -> Result<(Option<String>, Option<String>), Hold> {
    let Some(pc) = p.conditions.iter().find(|c| c.code == code) else {
        return Err(Hold(format!(
            "the PAS profile {} does not track {code}",
            p.name
        )));
    };
    let given = s
        .subtype
        .as_ref()
        .map(|t| (t.scheme.as_str(), t.code.as_str()));
    let wanted = given.or(implied);
    match (&pc.subtypes, wanted) {
        // A subtype the pathway only implies is dropped where the PAS keeps no list; one the writer
        // chose is not.
        (None, Some(_)) if given.is_none() => Ok((None, None)),
        (None, Some(_)) => Err(Hold(format!(
            "the PAS profile {} takes no subtype for {code}",
            p.name
        ))),
        (Some(l), None) if l.required => Err(Hold(format!(
            "the PAS profile {} needs a {} subtype for {code}",
            p.name, l.scheme
        ))),
        (Some(l), Some((scheme, _))) if scheme != l.scheme => Err(Hold(format!(
            "the PAS profile {} lists {code} subtypes as {}, not {scheme}",
            p.name, l.scheme
        ))),
        (Some(l), Some((_, c))) if !l.codes.iter().any(|x| x.code == c) => {
            Err(Hold(format!("the PAS list {} has no code {c}", l.scheme)))
        }
        (Some(_), Some((scheme, c))) => Ok((Some(c.to_string()), Some(scheme.to_string()))),
        (_, None) => Ok((None, None)),
    }
}

fn named(s: &Suspicion) -> Result<(), Hold> {
    match s.source {
        Source::Clinician => Ok(()),
        Source::ScreeningProgramme | Source::Result => Err(Hold(
            "the PAS needs the clinician who suspected it; this suspicion was raised by a programme or a result"
                .into(),
        )),
    }
}

/// The PAS body for a suspicion as raised.
///
/// # Errors
///
/// A [`Hold`]: no PAS condition for the pathway, not tracked by the profile, a subtype the profile's
/// list does not hold or a required one missing, no named clinician, or a bad time zone.
pub fn new_suspicion(
    s: &Suspicion,
    recorded_by: &str,
    p: &ActiveProfile,
) -> Result<NewSuspicion, Hold> {
    let (code, implied) = mapped(s)?;
    named(s)?;
    let (subtype, subtype_scheme) = subtype(s, code, implied, p)?;
    Ok(NewSuspicion {
        condition: code.to_string(),
        subtype,
        subtype_scheme,
        suspected_on: day_in(&p.time_zone, &s.point_of_suspicion)?,
        suspected_by: s.by.clone(),
        recorded_by: recorded_by.to_string(),
    })
}

/// The PAS body for an amendment from `was` to `now`, or `None` when nothing the PAS holds changed
/// (the grounds, say).
///
/// # Errors
///
/// A [`Hold`]: the condition changed (a person stops the PAS suspicion and records it again); a
/// subtype was removed (the amend route keeps a field left out); or any problem [`new_suspicion`] has.
pub fn amendment(
    was: &Suspicion,
    now: &Suspicion,
    reason: &str,
    amended_by: &str,
    p: &ActiveProfile,
) -> Result<Option<Amend>, Hold> {
    let (before, _) = mapped(was)?;
    let (code, implied) = mapped(now)?;
    if before != code {
        return Err(Hold(format!(
            "the PAS condition changed from {before} to {code}: a person stops the PAS suspicion and records it again"
        )));
    }
    named(now)?;
    let day = |s: &Suspicion| day_in(&p.time_zone, &s.point_of_suspicion);
    let (old_day, new_day) = (day(was)?, day(now)?);
    let (old_sub, _) = subtype(was, code, implied, p).unwrap_or((None, None));
    let (new_sub, new_scheme) = subtype(now, code, implied, p)?;
    if old_sub.is_some() && new_sub.is_none() {
        return Err(Hold(
            "the subtype was removed: the PAS amend route cannot remove one".into(),
        ));
    }
    let sub_changed = new_sub != old_sub;
    let a = Amend {
        suspected_on: (new_day != old_day).then_some(new_day),
        suspected_by: (now.by != was.by).then(|| now.by.clone()),
        subtype: if sub_changed { new_sub } else { None },
        subtype_scheme: if sub_changed { new_scheme } else { None },
        reason: reason.to_string(),
        amended_by: amended_by.to_string(),
    };
    let any = a.suspected_on.is_some() || a.suspected_by.is_some() || a.subtype.is_some();
    Ok(any.then_some(a))
}

/// The PAS stop for an outcome recorded at `at`, or `None` for `confirmed`.
///
/// # Errors
///
/// A [`Hold`] for a bad time zone or time.
pub fn stop(
    o: &Outcome,
    at: DateTime<Utc>,
    stopped_by: &str,
    p: &ActiveProfile,
) -> Result<Option<Stop>, Hold> {
    let reason = match o.kind {
        OutcomeKind::Confirmed => return Ok(None),
        OutcomeKind::RuledOut => "downgraded",
        OutcomeKind::Withdrawn => "withdrawn",
    };
    Ok(Some(Stop {
        reason,
        stopped_on: day_in(
            &p.time_zone,
            &at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        )?,
        stopped_by: stopped_by.to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suspicion::{Grounds, GroundsKind, PatientTold, Subtype};

    fn profile(name: &str) -> ActiveProfile {
        let text = match name {
            "wales" => include_str!("../tests/fixtures/pas/profile-wales.json"),
            _ => include_str!("../tests/fixtures/pas/profile-generic.json"),
        };
        serde_json::from_str(text).unwrap()
    }

    fn cancer() -> Suspicion {
        Suspicion {
            condition: "Suspected lower gastrointestinal cancer".into(),
            code: None,
            subtype: Some(Subtype {
                scheme: "wales-primary-cancer-site".into(),
                code: "03".into(),
            }),
            point_of_suspicion: "2026-06-30T23:30:00Z".into(),
            source: Source::Clinician,
            by: "Dr Gwen Hughes".into(),
            grounds: Grounds {
                kind: GroundsKind::Criterion,
                detail: "NG12".into(),
                result_id: None,
            },
            instruction: None,
            patient_told: PatientTold::NotTold,
            pathway: "suspected-cancer-wales".into(),
            pathway_version: 2,
            band: None,
            factors: vec![],
            agreed: None,
        }
    }

    #[test]
    fn every_built_in_pathway_has_a_pas_condition() {
        for p in crate::pathways::catalogue() {
            assert!(condition(&p.id).is_some(), "{}", p.id);
        }
        assert_eq!(condition("unknown"), None);
    }

    #[test]
    fn a_welsh_cancer_suspicion_is_sent_with_its_site_and_the_local_day() {
        let body = new_suspicion(&cancer(), "Rhys Evans", &profile("wales")).unwrap();
        assert_eq!(body.condition, "cancer");
        assert_eq!(body.subtype.as_deref(), Some("03"));
        assert_eq!(
            body.subtype_scheme.as_deref(),
            Some("wales-primary-cancer-site")
        );
        assert_eq!(
            body.suspected_on, "2026-07-01",
            "23:30 UTC is the next day in summer time"
        );
        let json = serde_json::to_value(&body).unwrap();
        assert!(json.get("grounds").is_none() && json.get("patient_told").is_none());
    }

    #[test]
    fn the_live_profile_decides_and_a_mismatch_is_held() {
        assert_eq!(
            new_suspicion(&cancer(), "R", &profile("generic")),
            Err(Hold(
                "the PAS profile generic takes no subtype for cancer".into()
            ))
        );
        let mut none = cancer();
        none.subtype = None;
        none.pathway_version = 1;
        assert_eq!(
            new_suspicion(&none, "R", &profile("wales")),
            Err(Hold(
                "the PAS profile wales needs a wales-primary-cancer-site subtype for cancer".into()
            ))
        );
        assert!(new_suspicion(&none, "R", &profile("generic")).is_ok());
        let mut odd = cancer();
        odd.subtype.as_mut().unwrap().code = "99".into();
        assert_eq!(
            new_suspicion(&odd, "R", &profile("wales")),
            Err(Hold(
                "the PAS list wales-primary-cancer-site has no code 99".into()
            ))
        );
        let mut tz = profile("wales");
        tz.time_zone = "Mars/Olympus".into();
        assert!(new_suspicion(&cancer(), "R", &tz).is_err());
        let mut untracked = profile("wales");
        untracked.conditions.retain(|c| c.code != "cancer");
        assert_eq!(
            new_suspicion(&cancer(), "R", &untracked),
            Err(Hold("the PAS profile wales does not track cancer".into()))
        );
    }

    #[test]
    fn an_implied_subtype_is_sent_where_the_pas_keeps_the_list() {
        let mut s = cancer();
        s.subtype = None;
        s.pathway = "type-1-diabetes-child".into();
        s.pathway_version = 1;
        let body = new_suspicion(&s, "R", &profile("generic")).unwrap();
        assert_eq!(
            (body.subtype.as_deref(), body.subtype_scheme.as_deref()),
            (Some("type_1"), Some("generic-diabetes-type"))
        );
        let mut bare = profile("generic");
        for c in &mut bare.conditions {
            c.subtypes = None;
        }
        assert_eq!(new_suspicion(&s, "R", &bare).unwrap().subtype, None);
    }

    #[test]
    fn a_suspicion_with_no_named_clinician_is_held() {
        let mut s = cancer();
        s.source = Source::ScreeningProgramme;
        assert!(new_suspicion(&s, "R", &profile("wales")).is_err());
    }

    #[test]
    fn an_amendment_sends_only_what_the_pas_holds() {
        let p = profile("wales");
        let was = cancer();
        let mut now = was.clone();
        now.grounds.detail = "NG12, and a raised FIT".into();
        assert_eq!(amendment(&was, &now, "more", "R", &p), Ok(None));
        now.point_of_suspicion = "2026-06-29T10:00:00Z".into();
        now.subtype.as_mut().unwrap().code = "02".into();
        let a = amendment(&was, &now, "seen earlier", "R", &p)
            .unwrap()
            .unwrap();
        assert_eq!(a.suspected_on.as_deref(), Some("2026-06-29"));
        assert_eq!(a.suspected_by, None);
        assert_eq!(
            (a.subtype.as_deref(), a.subtype_scheme.as_deref()),
            (Some("02"), Some("wales-primary-cancer-site"))
        );
        let mut moved = was.clone();
        moved.pathway = "dementia".into();
        assert!(amendment(&was, &moved, "x", "R", &p).is_err());
        let mut cleared = was.clone();
        cleared.subtype = None;
        cleared.pathway_version = 1;
        assert!(amendment(&was, &cleared, "x", "R", &p).is_err());
    }

    #[test]
    fn only_ruled_out_and_withdrawn_stop_the_pas_clock() {
        let p = profile("wales");
        let at = DateTime::parse_from_rfc3339("2026-12-31T23:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let o = |kind| Outcome {
            kind,
            diagnosis: Some("d".into()),
            code: None,
            by: Some("b".into()),
            detail: "x".into(),
        };
        assert_eq!(stop(&o(OutcomeKind::Confirmed), at, "R", &p), Ok(None));
        let s = stop(&o(OutcomeKind::RuledOut), at, "R", &p)
            .unwrap()
            .unwrap();
        assert_eq!(
            (s.reason, s.stopped_on.as_str()),
            ("downgraded", "2026-12-31")
        );
        let s = stop(&o(OutcomeKind::Withdrawn), at, "R", &p)
            .unwrap()
            .unwrap();
        assert_eq!(s.reason, "withdrawn");
    }
}
