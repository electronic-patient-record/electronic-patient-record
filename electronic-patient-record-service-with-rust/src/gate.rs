//! Gated features (PA-4): the rule that decides what a feature that scores or drafts may do. It is pure: the
//! caller passes the gate's state and the time, and gets back one [`Effect`]. Nothing in the service uses it
//! yet; DO-8 (drafting) and SE-11 (agents) are the first intended callers.
//!
//! A feature has three things in front of it: a **switch** (`Off`, `Shadow`, `On`), a list of **named approver
//! roles** (for example the Clinical Safety Officer), and **approvals**, each recorded by a person for one
//! role. The effect is the weakest the three allow:
//!
//! | switch | approvals | effect |
//! |---|---|---|
//! | `Off` | any | [`Effect::Inert`]: the feature does nothing and records nothing |
//! | `Shadow` | any | [`Effect::Record`]: it runs and keeps its output for review, and shows nothing |
//! | `On` | every required role approved, in force | [`Effect::Show`] |
//! | `On` | any required role missing, expired or revoked | [`Effect::Inert`] |
//!
//! Shadow needs no approval, so that the evidence an approver asks for can be gathered; it can never show
//! anything. A gate with no required roles can never be `Show`: a feature that scores or drafts must name at
//! least one approver. One person cannot approve two roles, and an approval by anything other than a person
//! (a service or script identity) does not count. Approving is a separate, audited act that this module does
//! not perform.

/// The switch on a gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    /// The feature does nothing.
    Off,
    /// The feature runs and its output is recorded, never shown.
    Shadow,
    /// The feature may show its output, if every required approver has approved.
    On,
}

/// Who gave an approval. A script or service identity is recorded as such and never counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Approver {
    /// A person, by their token `sub`.
    Person(String),
    /// A service, script or agent. Kept so the refusal can be shown; never satisfies a role.
    Automated(String),
}

/// One approval, for one role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approval {
    /// The role approved for (for example `clinical-safety-officer`).
    pub role: String,
    /// Who gave it.
    pub by: Approver,
    /// When it took effect, seconds since 1970-01-01 UTC.
    pub from: i64,
    /// When it ends, or `None` for no end. An approval with no end is allowed but the caller should say so.
    pub until: Option<i64>,
    /// Withdrawn by a person: it no longer counts.
    pub revoked: bool,
}

/// The state of one gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    /// The switch.
    pub switch: Switch,
    /// The roles that must each have an approval in force.
    pub required: Vec<String>,
    /// The approvals recorded so far, in any order.
    pub approvals: Vec<Approval>,
}

/// What the feature may do now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Do nothing and record nothing.
    Inert,
    /// Run and keep the output for review; show nothing.
    Record,
    /// Run and show the output.
    Show,
}

/// Why a gate is not showing, for the screen that explains it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Missing {
    /// The gate names no approver at all.
    NoApproversNamed,
    /// These roles have no approval in force.
    Roles(Vec<String>),
}

fn in_force(a: &Approval, now: i64) -> bool {
    !a.revoked && a.from <= now && a.until.is_none_or(|u| now < u)
}

/// The roles that have an approval in force by a person, where no person approves two roles.
fn satisfied(gate: &Gate, now: i64) -> Vec<&str> {
    let mut used: Vec<&str> = Vec::new();
    let mut roles = Vec::new();
    for role in &gate.required {
        let found = gate.approvals.iter().find(|a| {
            a.role == *role
                && in_force(a, now)
                && matches!(&a.by, Approver::Person(p) if !used.contains(&p.as_str()))
        });
        if let Some(Approval {
            by: Approver::Person(p),
            ..
        }) = found
        {
            used.push(p.as_str());
            roles.push(role.as_str());
        }
    }
    roles
}

/// What the feature may do at `now`.
#[must_use]
pub fn effect(gate: &Gate, now: i64) -> Effect {
    match gate.switch {
        Switch::Off => Effect::Inert,
        Switch::Shadow => Effect::Record,
        Switch::On => {
            if missing(gate, now).is_none() {
                Effect::Show
            } else {
                Effect::Inert
            }
        }
    }
}

/// What stops the gate showing, or `None` when nothing does (the switch is not looked at).
#[must_use]
pub fn missing(gate: &Gate, now: i64) -> Option<Missing> {
    if gate.required.is_empty() {
        return Some(Missing::NoApproversNamed);
    }
    let have = satisfied(gate, now);
    let lacking: Vec<String> = gate
        .required
        .iter()
        .filter(|r| !have.contains(&r.as_str()))
        .cloned()
        .collect();
    (!lacking.is_empty()).then_some(Missing::Roles(lacking))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000;

    fn person(sub: &str) -> Approver {
        Approver::Person(sub.into())
    }

    fn approval(role: &str, by: Approver) -> Approval {
        Approval {
            role: role.into(),
            by,
            from: 0,
            until: None,
            revoked: false,
        }
    }

    fn gate(switch: Switch, required: &[&str], approvals: Vec<Approval>) -> Gate {
        Gate {
            switch,
            required: required.iter().map(ToString::to_string).collect(),
            approvals,
        }
    }

    #[test]
    fn off_is_inert_even_when_everyone_has_approved() {
        let g = gate(Switch::Off, &["cso"], vec![approval("cso", person("a"))]);
        assert_eq!(effect(&g, NOW), Effect::Inert);
    }

    #[test]
    fn shadow_records_and_never_shows_with_or_without_approval() {
        let none = gate(Switch::Shadow, &["cso"], vec![]);
        let all = gate(Switch::Shadow, &["cso"], vec![approval("cso", person("a"))]);
        assert_eq!(effect(&none, NOW), Effect::Record);
        assert_eq!(effect(&all, NOW), Effect::Record);
    }

    #[test]
    fn on_without_every_approval_does_nothing() {
        let g = gate(
            Switch::On,
            &["cso", "lead"],
            vec![approval("cso", person("a"))],
        );
        assert_eq!(effect(&g, NOW), Effect::Inert);
        assert_eq!(
            missing(&g, NOW),
            Some(Missing::Roles(vec!["lead".to_string()]))
        );
    }

    #[test]
    fn on_with_every_approval_in_force_shows() {
        let g = gate(
            Switch::On,
            &["cso", "lead"],
            vec![approval("cso", person("a")), approval("lead", person("b"))],
        );
        assert_eq!(effect(&g, NOW), Effect::Show);
        assert_eq!(missing(&g, NOW), None);
    }

    #[test]
    fn a_gate_that_names_no_approver_never_shows() {
        let g = gate(Switch::On, &[], vec![approval("cso", person("a"))]);
        assert_eq!(effect(&g, NOW), Effect::Inert);
        assert_eq!(missing(&g, NOW), Some(Missing::NoApproversNamed));
    }

    #[test]
    fn a_script_cannot_approve() {
        let g = gate(
            Switch::On,
            &["cso"],
            vec![approval("cso", Approver::Automated("ci-bot".into()))],
        );
        assert_eq!(effect(&g, NOW), Effect::Inert);
    }

    #[test]
    fn one_person_cannot_approve_two_roles() {
        let g = gate(
            Switch::On,
            &["cso", "lead"],
            vec![approval("cso", person("a")), approval("lead", person("a"))],
        );
        assert_eq!(effect(&g, NOW), Effect::Inert);
        assert_eq!(
            missing(&g, NOW),
            Some(Missing::Roles(vec!["lead".to_string()]))
        );
    }

    #[test]
    fn an_approval_counts_only_inside_its_window_and_until_revoked() {
        let mut a = approval("cso", person("a"));
        a.from = 500;
        a.until = Some(2_000);
        let g = |a: &Approval| gate(Switch::On, &["cso"], vec![a.clone()]);
        assert_eq!(effect(&g(&a), 499), Effect::Inert, "not yet in force");
        assert_eq!(effect(&g(&a), 500), Effect::Show, "from is inclusive");
        assert_eq!(effect(&g(&a), 1_999), Effect::Show);
        assert_eq!(effect(&g(&a), 2_000), Effect::Inert, "until is exclusive");
        a.revoked = true;
        assert_eq!(effect(&g(&a), 1_000), Effect::Inert, "revoked");
    }

    #[test]
    fn a_second_valid_approval_covers_a_role_whose_first_one_expired() {
        let mut old = approval("cso", person("a"));
        old.until = Some(10);
        let g = gate(
            Switch::On,
            &["cso"],
            vec![old, approval("cso", person("b"))],
        );
        assert_eq!(effect(&g, NOW), Effect::Show);
    }
}
