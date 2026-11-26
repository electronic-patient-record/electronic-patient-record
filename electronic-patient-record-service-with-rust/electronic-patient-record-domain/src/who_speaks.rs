//! Who speaks for a person who cannot, for a decision (RF-9; `spec/patient-refusal-of-consent`): an attorney, a guardian or
//! deputy, a court, each parent apart, a relative with no power, or nobody. **Nothing here decides who wins.**
//!
//! - **A power not seen is not a power.** One that is only said to exist is listed as claimed, apart from those whose
//!   paper someone saw ([`summarise`]).
//! - **Each person is their own entry.** Two parents are two entries, and when their views differ the disagreement is
//!   shown, never averaged or settled by order.
//! - **A relative's word is a source, not a decision.** A relative with no power has a view and never a power.

use serde::{Deserialize, Serialize};

/// The longest words of any field.
pub const WORDS_LIMIT: usize = 500;

/// Who they are, to the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// An attorney or agent for health and welfare.
    Attorney,
    /// A guardian, or a deputy appointed by a court.
    Guardian,
    /// A court's order.
    Court,
    /// A parent, for a child. Each parent is an entry.
    Parent,
    /// A relative or friend with no power to decide.
    Relative,
    /// Nobody: it was asked, and there is no one.
    Nobody,
}

impl Role {
    /// Whether this role can hold a power to decide, given paper that was seen.
    #[must_use]
    pub fn can_hold_power(self) -> bool {
        matches!(
            self,
            Role::Attorney | Role::Guardian | Role::Court | Role::Parent
        )
    }
}

/// What of the paper that gives the power was seen, and by whom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Paper {
    /// The original, or a copy certified as one.
    Original,
    /// A copy or a photograph.
    Copy,
    /// A register was checked and holds it.
    Register,
    /// Not seen: the power is only said to exist.
    NotSeen,
}

/// What they say about a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Says {
    /// They agree to it.
    Agrees,
    /// They refuse it.
    Refuses,
    /// They have not decided, or would not say.
    Undecided,
}

/// One person's view on one decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    /// The decision, in words.
    pub about: String,
    /// What they say.
    pub says: Says,
}

/// One person who speaks, or the record that nobody does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Speaker {
    /// Who they are to the person.
    pub role: Role,
    /// Their name, or for a court the court and the order. Empty for nobody.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// What of the paper was seen. Absent for a relative and for nobody, who hold no power.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paper: Option<Paper>,
    /// Who saw the paper, by role, and when, in words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seen_by: Option<String>,
    /// What the power covers, as the paper says, in words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub covers: Option<String>,
    /// Their views, one per decision.
    #[serde(default)]
    pub views: Vec<View>,
}

fn words(out: &mut Vec<String>, what: &str, value: &str) {
    if value.trim().is_empty() {
        out.push(format!("{what} must not be blank"));
    } else if value.chars().count() > WORDS_LIMIT {
        out.push(format!("{what} must be up to {WORDS_LIMIT} characters"));
    }
}

fn same_decision(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        s.split_whitespace()
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
    };
    norm(a) == norm(b)
}

/// What is wrong with one entry. The words name a field and never quote it.
#[must_use]
pub fn validate(s: &Speaker) -> Vec<String> {
    let mut out = Vec::new();
    match (s.role, &s.name) {
        (Role::Nobody, Some(_)) => out.push("nobody has no name".into()),
        (Role::Nobody, None) => {}
        (_, None) => out.push("name is required".into()),
        (_, Some(name)) => words(&mut out, "name", name),
    }
    match (s.role.can_hold_power(), s.paper) {
        (true, None) => out.push(
            "paper is required for a role that can hold a power: not-seen is an answer".into(),
        ),
        (false, Some(_)) => out.push("paper is for a role that can hold a power".into()),
        _ => {}
    }
    let seen = matches!(
        s.paper,
        Some(Paper::Original | Paper::Copy | Paper::Register)
    );
    if seen != s.seen_by.is_some() {
        out.push("seen_by is required when the paper was seen, and only then".into());
    }
    if s.covers.is_some() && !seen {
        out.push("covers is what seen paper says".into());
    }
    for (what, value) in [("seen_by", &s.seen_by), ("covers", &s.covers)] {
        if let Some(value) = value {
            words(&mut out, what, value);
        }
    }
    if s.role == Role::Nobody && !s.views.is_empty() {
        out.push("nobody has no views".into());
    }
    for (i, v) in s.views.iter().enumerate() {
        words(&mut out, "views.about", &v.about);
        if s.views
            .iter()
            .take(i)
            .any(|w| same_decision(&w.about, &v.about))
        {
            out.push("one view per decision for each speaker".into());
        }
    }
    out
}

/// What the entries amount to. Nothing in it says who decides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Summary<'a> {
    /// Those whose power someone saw.
    pub powers_seen: Vec<&'a Speaker>,
    /// Those who say they hold a power that nobody has seen: **not a power**.
    pub claimed_not_seen: Vec<&'a Speaker>,
    /// Relatives and friends with no power: their word is a source.
    pub without_power: Vec<&'a Speaker>,
    /// Someone recorded that nobody speaks for the person.
    pub nobody_recorded: bool,
    /// "Nobody" is recorded beside someone who speaks: the record contradicts itself, and a person must look.
    pub contradicts: bool,
}

/// Sort the entries into powers seen, powers claimed and not seen, and no power.
#[must_use]
pub fn summarise(all: &[Speaker]) -> Summary<'_> {
    let seen = |s: &&Speaker| {
        matches!(
            s.paper,
            Some(Paper::Original | Paper::Copy | Paper::Register)
        )
    };
    let nobody_recorded = all.iter().any(|s| s.role == Role::Nobody);
    Summary {
        powers_seen: all
            .iter()
            .filter(|s| s.role.can_hold_power())
            .filter(seen)
            .collect(),
        claimed_not_seen: all
            .iter()
            .filter(|s| s.role.can_hold_power() && s.paper == Some(Paper::NotSeen))
            .collect(),
        without_power: all.iter().filter(|s| s.role == Role::Relative).collect(),
        nobody_recorded,
        contradicts: nobody_recorded && all.iter().any(|s| s.role != Role::Nobody),
    }
}

/// The views on `about` of each speaker who gave one, as recorded, with whether their power was seen. Two parents are two
/// lines.
#[must_use]
pub fn views_on<'a>(all: &'a [Speaker], about: &str) -> Vec<(&'a Speaker, Says)> {
    all.iter()
        .filter_map(|s| {
            s.views
                .iter()
                .find(|v| same_decision(&v.about, about))
                .map(|v| (s, v.says))
        })
        .collect()
}

/// Whether, among those whose power was seen, one agrees to `about` and another refuses it. A relative's word, and a power
/// nobody saw, take no side here: they are shown, and do not make a dispute between powers.
#[must_use]
pub fn powers_disagree(all: &[Speaker], about: &str) -> bool {
    let seen = |s: &Speaker| {
        matches!(
            s.paper,
            Some(Paper::Original | Paper::Copy | Paper::Register)
        )
    };
    let says: Vec<Says> = views_on(all, about)
        .into_iter()
        .filter(|(s, _)| s.role.can_hold_power() && seen(s))
        .map(|(_, says)| says)
        .collect();
    says.contains(&Says::Agrees) && says.contains(&Says::Refuses)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent(name: &str, paper: Paper, says: Says) -> Speaker {
        let seen = paper != Paper::NotSeen;
        Speaker {
            role: Role::Parent,
            name: Some(name.into()),
            paper: Some(paper),
            seen_by: seen.then(|| "ward clerk, birth certificate, today".to_string()),
            covers: None,
            views: vec![View {
                about: "whether to give blood".into(),
                says,
            }],
        }
    }

    #[test]
    fn two_parents_who_disagree_are_two_entries_and_the_disagreement_is_shown() {
        let all = [
            parent("Parent A", Paper::Original, Says::Agrees),
            parent("Parent B", Paper::Copy, Says::Refuses),
        ];
        assert!(all.iter().all(|s| validate(s).is_empty()));
        let views = views_on(&all, "Whether to give  BLOOD");
        assert_eq!(views.len(), 2, "two lines, one for each parent");
        assert!(powers_disagree(&all, "whether to give blood"));
        assert!(!powers_disagree(&all, "whether to operate"));
    }

    #[test]
    fn a_power_not_seen_is_not_a_power_and_makes_no_dispute() {
        let all = [
            parent("Parent A", Paper::Original, Says::Agrees),
            parent("Says they are the guardian", Paper::NotSeen, Says::Refuses),
        ];
        let summary = summarise(&all);
        assert_eq!(summary.powers_seen.len(), 1);
        assert_eq!(summary.claimed_not_seen.len(), 1);
        assert!(!powers_disagree(&all, "whether to give blood"));
        // Seen by whom is required when seen, and refused when not.
        let mut unseen_but_seen_by = all[1].clone();
        unseen_but_seen_by.seen_by = Some("x".into());
        assert_eq!(validate(&unseen_but_seen_by).len(), 1);
        let mut seen_without_who = all[0].clone();
        seen_without_who.seen_by = None;
        assert_eq!(validate(&seen_without_who).len(), 1);
    }

    #[test]
    fn a_relatives_word_is_a_source_and_never_a_power() {
        let sister = Speaker {
            role: Role::Relative,
            name: Some("their sister".into()),
            paper: None,
            seen_by: None,
            covers: None,
            views: vec![View {
                about: "whether to give blood".into(),
                says: Says::Refuses,
            }],
        };
        assert!(validate(&sister).is_empty());
        let with_paper = Speaker {
            paper: Some(Paper::Original),
            seen_by: Some("x".into()),
            ..sister.clone()
        };
        assert!(
            !validate(&with_paper).is_empty(),
            "a relative cannot be given a power here"
        );
        let all = [parent("Parent A", Paper::Original, Says::Agrees), sister];
        assert_eq!(summarise(&all).without_power.len(), 1);
        assert!(!powers_disagree(&all, "whether to give blood"));
        assert_eq!(
            views_on(&all, "whether to give blood").len(),
            2,
            "the sister's word is shown"
        );
    }

    #[test]
    fn nobody_is_an_answer_and_beside_someone_it_is_a_contradiction() {
        let nobody = Speaker {
            role: Role::Nobody,
            name: None,
            paper: None,
            seen_by: None,
            covers: None,
            views: vec![],
        };
        assert!(validate(&nobody).is_empty());
        assert!(summarise(std::slice::from_ref(&nobody)).nobody_recorded);
        assert!(!summarise(std::slice::from_ref(&nobody)).contradicts);
        let both = [
            nobody.clone(),
            parent("Parent A", Paper::Original, Says::Agrees),
        ];
        assert!(summarise(&both).contradicts);
        let named = Speaker {
            name: Some("x".into()),
            ..nobody
        };
        assert_eq!(validate(&named).len(), 1);
    }

    #[test]
    fn an_attorney_court_and_guardian_need_their_paper_answered_and_one_view_per_decision() {
        for role in [Role::Attorney, Role::Guardian, Role::Court] {
            let s = Speaker {
                role,
                name: Some("A".into()),
                paper: None,
                seen_by: None,
                covers: None,
                views: vec![],
            };
            assert_eq!(validate(&s).len(), 1, "{role:?}");
        }
        let mut twice = parent("Parent A", Paper::Register, Says::Agrees);
        twice.views.push(View {
            about: "WHETHER to give blood".into(),
            says: Says::Refuses,
        });
        assert_eq!(validate(&twice).len(), 1);
        let covers_unseen = Speaker {
            covers: Some("health decisions".into()),
            ..parent("P", Paper::NotSeen, Says::Undecided)
        };
        assert_eq!(validate(&covers_unseen).len(), 1);
    }
}
