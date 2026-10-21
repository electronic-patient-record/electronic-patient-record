//! The policy engine (SE-1): who may do what to which kind of record, and under which
//! conditions. Pure and deterministic: no I/O, no clock (the caller passes `now`), no panics.
//!
//! Two layers, one engine:
//!
//! 1. **The route gate** ([`gate`]): may this subject *ever* perform this action on this kind of
//!    resource? It runs in middleware for every route, so a route without a policy entry is
//!    refused (see [`crate::routes`]). It is coarse role-based access control (RBAC).
//! 2. **The record decision** ([`evaluate`]): may this subject perform this action on *this*
//!    record, given the relationship to the patient, the record's security labels, the purpose of
//!    use, the patient's consent and any break-the-glass grant? It is attribute-based (ABAC) and
//!    is called by a handler once it has loaded the record. No handler reads a stored record
//!    today, so nothing calls it yet; the tests exercise it.
//!
//! Rules that hold for every input (and are property-tested across the whole matrix):
//!
//! * Deny by default. A role the engine does not know grants nothing.
//! * There is no delete action: the clinical record is append-only. Withdrawing an entry is
//!   [`Action::MarkEnteredInError`].
//! * Every allow carries [`Obligation::Audit`].
//! * Break-the-glass lifts only the missing care relationship, only for reads, only for a
//!   treating role. It never overrides a consent refusal and never unmasks a security label.
//! * A subject sees a labelled record unmasked only if its role is on that label's list; otherwise
//!   the decision is an allow with [`Obligation::Mask`] naming the labels to hide.
//!
//! **The permission table, the label lists and the purpose rules below are a first draft by an
//! engineer, not clinical governance.** They are unvalidated guesses that need review by a
//! clinical safety officer and a privacy officer before any real use (tasks AS-1, SE-3, SE-4).

use std::collections::{BTreeMap, BTreeSet};

/// A role a subject can hold, read from the token's `role` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Doctor or other independent prescriber.
    Clinician,
    /// Nurse or midwife.
    Nurse,
    /// Pharmacist.
    Pharmacist,
    /// Allied health professional (physio, OT, dietitian and so on).
    AlliedHealth,
    /// Front desk: registers and finds patients, sees no clinical content.
    Receptionist,
    /// Clinical administrator: registration, consent records, corrections.
    ClinicalAdmin,
    /// The patient, about their own record.
    Patient,
    /// A proxy (parent, carer) acting for a patient.
    Proxy,
    /// Reads the audit trail; no clinical content.
    Auditor,
    /// A system integration (service account).
    Integrator,
}

impl Role {
    /// Every role, in a fixed order.
    pub const ALL: [Role; 10] = [
        Role::Clinician,
        Role::Nurse,
        Role::Pharmacist,
        Role::AlliedHealth,
        Role::Receptionist,
        Role::ClinicalAdmin,
        Role::Patient,
        Role::Proxy,
        Role::Auditor,
        Role::Integrator,
    ];

    /// The token value for this role (`role=clinician`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Clinician => "clinician",
            Role::Nurse => "nurse",
            Role::Pharmacist => "pharmacist",
            Role::AlliedHealth => "allied-health",
            Role::Receptionist => "receptionist",
            Role::ClinicalAdmin => "clinical-admin",
            Role::Patient => "patient",
            Role::Proxy => "proxy",
            Role::Auditor => "auditor",
            Role::Integrator => "integrator",
        }
    }

    /// Parse a token value. Unknown values are not roles.
    #[must_use]
    pub fn parse(value: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|r| r.as_str() == value)
    }

    /// Whether the role treats patients (and so may use break-the-glass).
    #[must_use]
    pub fn treats(self) -> bool {
        matches!(
            self,
            Role::Clinician | Role::Nurse | Role::Pharmacist | Role::AlliedHealth
        )
    }
}

/// A kind of resource. FHIR names where there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Resource {
    /// Demographics and identifiers (FHIR `Patient`), and matching them.
    PatientIdentity,
    /// FHIR `Encounter`.
    Encounter,
    /// FHIR `Condition`.
    Condition,
    /// FHIR `Observation`.
    Observation,
    /// FHIR `MedicationRequest`.
    MedicationRequest,
    /// FHIR `AllergyIntolerance`.
    AllergyIntolerance,
    /// FHIR `DocumentReference` / `Composition` (notes).
    Document,
    /// FHIR `Consent`.
    Consent,
    /// FHIR `AuditEvent`: the access log.
    AuditLog,
    /// Who is on a patient's care team, and who is the patient or their proxy (`relationships.rs`).
    CareRelationship,
    /// A merge of two patient records, and its reversal (`links.rs`).
    PatientLink,
    /// HL7 messages the feed refused, waiting for a person (`feed_queue.rs`). They hold patient
    /// data but belong to no patient on record, so no care relationship applies.
    FeedQueue,
}

impl Resource {
    /// Every resource kind, in a fixed order.
    pub const ALL: [Resource; 12] = [
        Resource::PatientIdentity,
        Resource::Encounter,
        Resource::Condition,
        Resource::Observation,
        Resource::MedicationRequest,
        Resource::AllergyIntolerance,
        Resource::Document,
        Resource::Consent,
        Resource::AuditLog,
        Resource::CareRelationship,
        Resource::PatientLink,
        Resource::FeedQueue,
    ];

    /// A stable name for audit rows and route tables.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Resource::PatientIdentity => "PatientIdentity",
            Resource::Encounter => "Encounter",
            Resource::Condition => "Condition",
            Resource::Observation => "Observation",
            Resource::MedicationRequest => "MedicationRequest",
            Resource::AllergyIntolerance => "AllergyIntolerance",
            Resource::Document => "Document",
            Resource::Consent => "Consent",
            Resource::AuditLog => "AuditLog",
            Resource::CareRelationship => "CareRelationship",
            Resource::PatientLink => "PatientLink",
            Resource::FeedQueue => "FeedQueue",
        }
    }

    /// Parse a name as [`Resource::name`] writes it. Unknown names are not resources.
    #[must_use]
    pub fn parse(name: &str) -> Option<Resource> {
        Resource::ALL.into_iter().find(|r| r.name() == name)
    }

    /// Whether the resource holds clinical content: reading it needs a care relationship.
    #[must_use]
    pub fn clinical(self) -> bool {
        !matches!(
            self,
            Resource::PatientIdentity
                | Resource::Consent
                | Resource::AuditLog
                | Resource::CareRelationship
                | Resource::PatientLink
                | Resource::FeedQueue
        )
    }
}

/// What is being done. There is deliberately no `Delete`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    /// Read one record.
    Read,
    /// Search or list.
    Search,
    /// Add a new record or version.
    Create,
    /// Change a record (which appends a new version).
    Update,
    /// Withdraw an entry as wrongly recorded; the original stays visible to those allowed to see it.
    MarkEnteredInError,
    /// Bulk export.
    Export,
}

impl Action {
    /// Every action, in a fixed order.
    pub const ALL: [Action; 6] = [
        Action::Read,
        Action::Search,
        Action::Create,
        Action::Update,
        Action::MarkEnteredInError,
        Action::Export,
    ];

    /// A stable name for audit rows.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Action::Read => "read",
            Action::Search => "search",
            Action::Create => "create",
            Action::Update => "update",
            Action::MarkEnteredInError => "entered-in-error",
            Action::Export => "export",
        }
    }

    /// Whether the action only discloses (as opposed to changing the record).
    #[must_use]
    pub fn reads(self) -> bool {
        matches!(self, Action::Read | Action::Search | Action::Export)
    }
}

/// A FHIR confidentiality or sensitivity label on a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Label {
    /// `R`: restricted.
    Restricted,
    /// `V`: very restricted.
    VeryRestricted,
    /// `PSY`: psychiatry.
    Psychiatry,
    /// `HIV`.
    Hiv,
    /// `SUD`: substance use disorder (42 CFR Part 2).
    SubstanceUse,
    /// `ETH`: ethnicity-related.
    Ethnicity,
}

impl Label {
    /// Every label, in a fixed order.
    pub const ALL: [Label; 6] = [
        Label::Restricted,
        Label::VeryRestricted,
        Label::Psychiatry,
        Label::Hiv,
        Label::SubstanceUse,
        Label::Ethnicity,
    ];

    /// The FHIR code.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Label::Restricted => "R",
            Label::VeryRestricted => "V",
            Label::Psychiatry => "PSY",
            Label::Hiv => "HIV",
            Label::SubstanceUse => "SUD",
            Label::Ethnicity => "ETH",
        }
    }

    /// Parse a FHIR code as [`Label::code`] writes it. Unknown codes are not labels.
    #[must_use]
    pub fn parse(code: &str) -> Option<Label> {
        Label::ALL.into_iter().find(|l| l.code() == code)
    }

    /// Roles that see a record with this label unmasked, before the patient's consent is applied.
    fn unmasked_for(self) -> &'static [Role] {
        use Role::{AlliedHealth, Clinician, Nurse, Patient, Pharmacist};
        match self {
            Label::Restricted => &[Clinician, Nurse, Pharmacist, AlliedHealth, Patient],
            Label::VeryRestricted => &[Clinician, Patient],
            Label::Psychiatry => &[Clinician, Nurse, AlliedHealth, Patient],
            Label::Hiv => &[Clinician, Nurse, Pharmacist, Patient],
            Label::SubstanceUse => &[Clinician, Patient],
            Label::Ethnicity => &[Clinician, Nurse, AlliedHealth, Patient],
        }
    }
}

/// How the subject relates to the patient the record belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relationship {
    /// No established relationship.
    None,
    /// On the patient's care team.
    CareTeam,
    /// Involved in an active encounter with the patient.
    ActiveEncounter,
    /// The subject is the patient.
    SelfPatient,
    /// The subject is a registered proxy for the patient.
    Proxy,
}

impl Relationship {
    /// Whether this counts as a legitimate treating relationship.
    #[must_use]
    pub fn treating(self) -> bool {
        matches!(self, Relationship::CareTeam | Relationship::ActiveEncounter)
    }
}

/// Why the data is wanted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Purpose {
    /// Direct care.
    Treatment,
    /// An emergency.
    Emergency,
    /// Running the service (registration, scheduling, corrections).
    Operations,
    /// Billing or claims.
    Payment,
    /// Research or other secondary use.
    Research,
    /// The patient or proxy looking at their own record.
    PatientAccess,
    /// Reviewing the audit trail.
    Audit,
}

impl Purpose {
    /// Parse a purpose-of-use code as sent in `X-Purpose-Of-Use`.
    #[must_use]
    pub fn parse(code: &str) -> Option<Purpose> {
        Some(match code {
            "treatment" => Purpose::Treatment,
            "emergency" => Purpose::Emergency,
            "operations" => Purpose::Operations,
            "payment" => Purpose::Payment,
            "research" => Purpose::Research,
            "patient-access" => Purpose::PatientAccess,
            "audit" => Purpose::Audit,
            _ => return None,
        })
    }

    /// The code, as recorded in the audit trail.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Purpose::Treatment => "treatment",
            Purpose::Emergency => "emergency",
            Purpose::Operations => "operations",
            Purpose::Payment => "payment",
            Purpose::Research => "research",
            Purpose::PatientAccess => "patient-access",
            Purpose::Audit => "audit",
        }
    }
}

/// The patient's recorded consent for this access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Consent {
    /// Nothing recorded: the default rules apply.
    NotRecorded,
    /// The patient permits the disclosure.
    Permit,
    /// The patient refuses it.
    Deny,
}

/// A time-boxed break-the-glass grant (SE-3 creates and stores these; here it is only checked).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakGlass {
    /// The reason the subject gave. Must not be blank.
    pub reason: String,
    /// Unix seconds at which the grant ends.
    pub expires_at: i64,
}

/// The subject: roles from the token's `role` attribute.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Subject {
    /// Roles held. Values the engine does not know were dropped, so they grant nothing.
    pub roles: BTreeSet<Role>,
    /// Organisations the token names, from its `organisation` attribute: short lowercase tokens
    /// only (anything else is dropped, since it ends up in the audit trail and the patient's log).
    /// With Keycloak the attribute comes from the role map, so an organisation is a realm role
    /// that `role-map.json` maps to `{"organisation": ["…"]}`.
    pub organisations: BTreeSet<String>,
}

impl Subject {
    /// Roles from a token's attributes: every value of the `role` attribute that is a known role.
    #[must_use]
    pub fn from_attrs(attrs: &BTreeMap<String, Vec<String>>) -> Subject {
        let roles = attrs
            .get("role")
            .into_iter()
            .flatten()
            .filter_map(|v| Role::parse(v))
            .collect();
        let organisations = attrs
            .get("organisation")
            .into_iter()
            .flatten()
            .filter(|v| {
                !v.is_empty()
                    && v.len() <= 64
                    && v.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
            .cloned()
            .collect();
        Subject {
            roles,
            organisations,
        }
    }
}

/// What the caller must do if the decision is an allow.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Obligation {
    /// Write an audit row for the access.
    Audit,
    /// Hide content carrying these labels and show "restricted information exists".
    Mask(BTreeSet<Label>),
    /// The access used break-the-glass for this reason; flag the audit row.
    BreakTheGlass(String),
    /// Tell the privacy officer in real time.
    NotifyPrivacyOfficer,
}

/// Why a request was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenyReason {
    /// No role of the subject permits this action on this kind of resource.
    NotPermitted,
    /// The purpose of use does not fit the role.
    PurposeNotAllowed,
    /// There is no care relationship (see `btg_offer`).
    NoRelationship,
    /// The patient refuses this disclosure.
    ConsentDenied,
}

/// The engine's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Allowed, subject to the obligations.
    Allow(Vec<Obligation>),
    /// Refused. `btg_offer` says break-the-glass could lift the refusal.
    Deny {
        /// Why.
        reason: DenyReason,
        /// Whether to offer break-the-glass.
        btg_offer: bool,
    },
}

impl Decision {
    /// Whether this is an allow.
    #[must_use]
    pub fn allowed(&self) -> bool {
        matches!(self, Decision::Allow(_))
    }
}

/// What a role may do to a kind of resource. The whole RBAC table.
fn permitted(role: Role, resource: Resource) -> &'static [Action] {
    use Action::{Create, Export, MarkEnteredInError as Eie, Read, Search, Update};
    use Resource::{
        AllergyIntolerance as Allergy, AuditLog, CareRelationship as Rel, Condition,
        Consent as Cons, Document, Encounter, FeedQueue as Queue, MedicationRequest as Meds,
        Observation, PatientIdentity as Identity, PatientLink as Link,
    };
    match (role, resource) {
        // Clinicians: full clinical work, and register or correct identity.
        (Role::Clinician, Identity) => &[Read, Search, Create, Update],
        (Role::Clinician, Encounter | Condition | Observation | Meds | Allergy | Document) => {
            &[Read, Search, Create, Update, Eie]
        }
        (Role::Clinician, Cons) => &[Read],
        // Nurses: record care; cannot withdraw others' entries; read-only on prescriptions.
        (Role::Nurse, Identity) => &[Read, Search],
        (Role::Nurse, Encounter | Condition | Observation | Allergy | Document) => {
            &[Read, Search, Create, Update]
        }
        (Role::Nurse, Meds) => &[Read, Search],
        (Role::Nurse, Cons) => &[Read],
        // Pharmacists: read the clinical picture; act on prescriptions.
        (Role::Pharmacist, Identity) => &[Read, Search],
        (Role::Pharmacist, Meds) => &[Read, Search, Update],
        (Role::Pharmacist, Condition | Observation | Allergy) => &[Read, Search],
        // Allied health: record their own work.
        (Role::AlliedHealth, Identity) => &[Read, Search],
        (Role::AlliedHealth, Encounter | Observation | Condition | Document) => {
            &[Read, Search, Create, Update]
        }
        // Reception: identity only, never clinical content.
        (Role::Receptionist, Identity) => &[Read, Search, Create, Update],
        // Clinical admin: identity and consent records. No clinical content: that would need a care
        // relationship an administrator does not have.
        (Role::ClinicalAdmin, Identity) => &[Read, Search, Create, Update],
        (Role::ClinicalAdmin, Cons) => &[Read, Search, Create, Update],
        // Only an administrator says who is a patient's team, self or proxy: never the person
        // who would gain access by it.
        (Role::ClinicalAdmin, Rel) => &[Read, Search, Create, Update],
        // Merging two records is an administrator's act; a clinician may see that one happened.
        (Role::ClinicalAdmin, Link) => &[Read, Search, Create, Update],
        (Role::Clinician, Link) => &[Read],
        // Refused feed messages: an administrator reads and closes them. They are patient data, so
        // no one else sees them, and the feed itself can never read its own refusals back.
        (Role::ClinicalAdmin, Queue) => &[Read, Search, Update],
        // The patient: read their own record and manage their own consent; see who looked.
        (Role::Patient, Identity) => &[Read],
        (Role::Patient, Encounter | Condition | Observation | Meds | Allergy | Document) => {
            &[Read, Search]
        }
        (Role::Patient, Cons) => &[Read, Search, Create, Update],
        (Role::Patient, AuditLog) => &[Read, Search],
        // A proxy: read-only, no consent changes, no access log.
        (Role::Proxy, Identity) => &[Read],
        (Role::Proxy, Encounter | Condition | Observation | Meds | Allergy | Document) => {
            &[Read, Search]
        }
        // The auditor: the audit trail only.
        (Role::Auditor, AuditLog) => &[Read, Search, Export],
        // Integrations: find patients, and feed identity in (an ADT feed over MLLP or HL7/FHIR
        // push): create and correct demographics. Never read a record back, never any clinical
        // content, no export until consent (SE-2).
        (Role::Integrator, Identity) => &[Search, Create, Update],
        _ => &[],
    }
}

/// Which purposes of use fit a role.
fn purpose_fits(role: Role, purpose: Purpose) -> bool {
    use Purpose::{Audit, Emergency, Operations, PatientAccess, Treatment};
    match role {
        Role::Clinician | Role::Nurse | Role::Pharmacist | Role::AlliedHealth => {
            matches!(purpose, Treatment | Emergency | Operations)
        }
        Role::Receptionist | Role::ClinicalAdmin | Role::Integrator => {
            matches!(purpose, Operations | Treatment)
        }
        Role::Patient | Role::Proxy => purpose == PatientAccess,
        Role::Auditor => purpose == Audit,
    }
}

/// The route gate: may a subject holding `roles` ever do `action` to `resource`?
#[must_use]
pub fn gate(subject: &Subject, resource: Resource, action: Action) -> bool {
    subject
        .roles
        .iter()
        .any(|r| permitted(*r, resource).contains(&action))
}

/// Everything the record decision looks at.
#[derive(Debug, Clone)]
pub struct Request {
    /// Who.
    pub subject: Subject,
    /// What kind of record.
    pub resource: Resource,
    /// Doing what.
    pub action: Action,
    /// The record's security labels (empty for none).
    pub labels: BTreeSet<Label>,
    /// The subject's relationship to the patient.
    pub relationship: Relationship,
    /// Why.
    pub purpose: Purpose,
    /// The patient's consent for this disclosure.
    pub consent: Consent,
    /// A break-the-glass grant, if the subject holds one.
    pub break_glass: Option<BreakGlass>,
    /// Now, in unix seconds.
    pub now: i64,
}

/// The record decision. See the module notes for the rules.
#[must_use]
pub fn evaluate(request: &Request) -> Decision {
    let mut best: Option<Decision> = None;
    for role in &request.subject.roles {
        let decision = evaluate_role(*role, request);
        best = Some(match (best, decision) {
            (None, d) => d,
            // An allow beats a deny; among allows, the one hiding least wins.
            (Some(Decision::Allow(a)), Decision::Allow(b)) => {
                Decision::Allow(if masked(&b) < masked(&a) { b } else { a })
            }
            (Some(Decision::Allow(a)), Decision::Deny { .. }) => Decision::Allow(a),
            (Some(Decision::Deny { .. }), d @ Decision::Allow(_)) => d,
            // Among denials, prefer the one that offers break-the-glass.
            (Some(a @ Decision::Deny { .. }), b @ Decision::Deny { .. }) => {
                if matches!(
                    b,
                    Decision::Deny {
                        btg_offer: true,
                        ..
                    }
                ) {
                    b
                } else {
                    a
                }
            }
        });
    }
    best.unwrap_or(Decision::Deny {
        reason: DenyReason::NotPermitted,
        btg_offer: false,
    })
}

fn masked(obligations: &[Obligation]) -> usize {
    obligations
        .iter()
        .map(|o| match o {
            Obligation::Mask(labels) => labels.len(),
            _ => 0,
        })
        .sum()
}

fn deny(reason: DenyReason, btg_offer: bool) -> Decision {
    Decision::Deny { reason, btg_offer }
}

fn evaluate_role(role: Role, r: &Request) -> Decision {
    if !permitted(role, r.resource).contains(&r.action) {
        return deny(DenyReason::NotPermitted, false);
    }
    if !purpose_fits(role, r.purpose) {
        return deny(DenyReason::PurposeNotAllowed, false);
    }

    // The patient and proxy roles work only through their own relationship.
    let own = match role {
        Role::Patient => Some(Relationship::SelfPatient),
        Role::Proxy => Some(Relationship::Proxy),
        _ => None,
    };
    if let Some(needed) = own
        && r.relationship != needed
    {
        return deny(DenyReason::NoRelationship, false);
    }

    // A refused consent stops every disclosure to someone other than the patient. Break-the-glass
    // does not lift it.
    if r.action.reads() && r.consent == Consent::Deny && role != Role::Patient {
        return deny(DenyReason::ConsentDenied, false);
    }

    let mut obligations = vec![Obligation::Audit];

    // Clinical content needs a care relationship (or break-the-glass for a treating role).
    if r.resource.clinical() && own.is_none() && !r.relationship.treating() {
        let btg_eligible = role.treats() && r.action.reads() && r.action != Action::Export;
        if !btg_eligible {
            return deny(DenyReason::NoRelationship, false);
        }
        match &r.break_glass {
            Some(grant) if !grant.reason.trim().is_empty() && grant.expires_at > r.now => {
                obligations.push(Obligation::BreakTheGlass(grant.reason.trim().to_string()));
                obligations.push(Obligation::NotifyPrivacyOfficer);
            }
            _ => return deny(DenyReason::NoRelationship, true),
        }
    }

    // Labels: hide what this role may not see. Substance-use records also need the patient's
    // consent for anyone but the patient (42 CFR Part 2).
    if r.action.reads() {
        let hidden: BTreeSet<Label> = r
            .labels
            .iter()
            .copied()
            .filter(|label| {
                let listed = label.unmasked_for().contains(&role);
                let part2 = *label == Label::SubstanceUse
                    && role != Role::Patient
                    && r.consent != Consent::Permit;
                !listed || part2
            })
            .collect();
        if !hidden.is_empty() {
            obligations.push(Obligation::Mask(hidden));
        }
    }
    Decision::Allow(obligations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(roles: &[Role]) -> Subject {
        Subject {
            roles: roles.iter().copied().collect(),
            ..Subject::default()
        }
    }

    fn request(role: Role, resource: Resource, action: Action) -> Request {
        Request {
            subject: subject(&[role]),
            resource,
            action,
            labels: BTreeSet::new(),
            relationship: match role {
                Role::Patient => Relationship::SelfPatient,
                Role::Proxy => Relationship::Proxy,
                _ => Relationship::CareTeam,
            },
            purpose: match role {
                Role::Patient | Role::Proxy => Purpose::PatientAccess,
                Role::Auditor => Purpose::Audit,
                _ => Purpose::Treatment,
            },
            consent: Consent::NotRecorded,
            break_glass: None,
            now: 1_000,
        }
    }

    #[test]
    fn roles_parse_from_token_attributes_and_unknown_values_grant_nothing() {
        let mut attrs = BTreeMap::new();
        attrs.insert(
            "role".to_string(),
            vec!["clinician".into(), "superuser".into(), "Clinician".into()],
        );
        attrs.insert("access".to_string(), vec!["write".into()]);
        let s = Subject::from_attrs(&attrs);
        assert_eq!(s.roles, BTreeSet::from([Role::Clinician]));
        assert!(Subject::from_attrs(&BTreeMap::new()).roles.is_empty());
        for role in Role::ALL {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }
    }

    #[test]
    fn resources_and_labels_parse_from_what_they_print() {
        for r in Resource::ALL {
            assert_eq!(Resource::parse(r.name()), Some(r));
        }
        for l in Label::ALL {
            assert_eq!(Label::parse(l.code()), Some(l));
        }
        assert_eq!(Resource::parse("patientidentity"), None);
        assert_eq!(Label::parse("sud"), None);
        assert_eq!(Label::parse(""), None);
    }

    #[test]
    fn a_subject_with_no_roles_may_do_nothing() {
        let none = Subject::default();
        for resource in Resource::ALL {
            for action in Action::ALL {
                assert!(!gate(&none, resource, action));
                let mut r = request(Role::Clinician, resource, action);
                r.subject = none.clone();
                assert!(!evaluate(&r).allowed());
            }
        }
    }

    #[test]
    fn the_gate_matches_the_documented_table_for_identity() {
        let can = |role, action| gate(&subject(&[role]), Resource::PatientIdentity, action);
        assert!(can(Role::Receptionist, Action::Create));
        assert!(can(Role::Integrator, Action::Search));
        assert!(can(Role::Integrator, Action::Create));
        assert!(can(Role::Integrator, Action::Update));
        assert!(!can(Role::Integrator, Action::Read));
        assert!(!can(Role::Auditor, Action::Search));
        assert!(!can(Role::Patient, Action::Search));
        assert!(!can(Role::Nurse, Action::Create));
    }

    #[test]
    fn a_receptionist_never_sees_clinical_content() {
        for resource in Resource::ALL.into_iter().filter(|r| r.clinical()) {
            for action in Action::ALL {
                assert!(!gate(&subject(&[Role::Receptionist]), resource, action));
            }
        }
    }

    #[test]
    fn only_the_auditor_reads_the_audit_log_besides_the_patient() {
        for role in Role::ALL {
            let can = gate(&subject(&[role]), Resource::AuditLog, Action::Read);
            assert_eq!(
                can,
                matches!(role, Role::Auditor | Role::Patient),
                "{role:?}"
            );
        }
        // ...and never writes it: the audit table is written by the service alone.
        for role in Role::ALL {
            for action in [Action::Create, Action::Update, Action::MarkEnteredInError] {
                assert!(
                    !gate(&subject(&[role]), Resource::AuditLog, action),
                    "{role:?}"
                );
            }
        }
    }

    #[test]
    fn a_clinician_on_the_care_team_reads_and_the_access_is_audited() {
        let d = evaluate(&request(Role::Clinician, Resource::Condition, Action::Read));
        assert_eq!(d, Decision::Allow(vec![Obligation::Audit]));
    }

    #[test]
    fn without_a_relationship_a_treating_role_is_refused_with_a_break_glass_offer() {
        let mut r = request(Role::Nurse, Resource::Observation, Action::Read);
        r.relationship = Relationship::None;
        assert_eq!(
            evaluate(&r),
            Decision::Deny {
                reason: DenyReason::NoRelationship,
                btg_offer: true
            }
        );
        // A writer gets no offer: break-the-glass is for reading.
        r.action = Action::Create;
        assert_eq!(
            evaluate(&r),
            Decision::Deny {
                reason: DenyReason::NoRelationship,
                btg_offer: false
            }
        );
    }

    #[test]
    fn break_glass_lifts_only_the_relationship_and_only_while_it_lasts() {
        let mut r = request(Role::Clinician, Resource::Condition, Action::Read);
        r.relationship = Relationship::None;
        r.break_glass = Some(BreakGlass {
            reason: "  collapsed in the waiting room ".into(),
            expires_at: 2_000,
        });
        let Decision::Allow(o) = evaluate(&r) else {
            panic!("should allow");
        };
        assert!(o.contains(&Obligation::BreakTheGlass(
            "collapsed in the waiting room".into()
        )));
        assert!(o.contains(&Obligation::NotifyPrivacyOfficer));
        assert!(o.contains(&Obligation::Audit));

        r.now = 2_000; // the grant ends at expires_at
        assert!(!evaluate(&r).allowed());
        r.now = 1_000;
        r.break_glass.as_mut().unwrap().reason = "   ".into();
        assert!(!evaluate(&r).allowed());
    }

    #[test]
    fn break_glass_does_not_override_a_consent_refusal_or_unmask_a_label() {
        let grant = Some(BreakGlass {
            reason: "emergency".into(),
            expires_at: 9_999,
        });
        let mut r = request(Role::Clinician, Resource::Condition, Action::Read);
        r.relationship = Relationship::None;
        r.break_glass = grant.clone();
        r.consent = Consent::Deny;
        assert_eq!(
            evaluate(&r),
            Decision::Deny {
                reason: DenyReason::ConsentDenied,
                btg_offer: false
            }
        );

        let mut r = request(Role::Nurse, Resource::Condition, Action::Read);
        r.relationship = Relationship::None;
        r.break_glass = grant;
        r.labels = BTreeSet::from([Label::VeryRestricted]);
        let Decision::Allow(o) = evaluate(&r) else {
            panic!("should allow");
        };
        assert!(o.contains(&Obligation::Mask(BTreeSet::from([Label::VeryRestricted]))));
    }

    #[test]
    fn labels_mask_for_roles_not_on_the_list() {
        let mut r = request(Role::Pharmacist, Resource::Condition, Action::Read);
        r.labels = BTreeSet::from([Label::Psychiatry, Label::Hiv]);
        let Decision::Allow(o) = evaluate(&r) else {
            panic!("should allow");
        };
        // A pharmacist may see HIV but not psychiatry.
        assert!(o.contains(&Obligation::Mask(BTreeSet::from([Label::Psychiatry]))));
        // The patient sees their own record unmasked.
        let mut own = request(Role::Patient, Resource::Condition, Action::Read);
        own.labels = BTreeSet::from(Label::ALL);
        assert_eq!(evaluate(&own), Decision::Allow(vec![Obligation::Audit]));
    }

    #[test]
    fn substance_use_records_need_the_patients_consent_for_staff() {
        let mut r = request(Role::Clinician, Resource::Document, Action::Read);
        r.labels = BTreeSet::from([Label::SubstanceUse]);
        let Decision::Allow(o) = evaluate(&r) else {
            panic!("should allow");
        };
        assert!(o.contains(&Obligation::Mask(BTreeSet::from([Label::SubstanceUse]))));
        r.consent = Consent::Permit;
        assert_eq!(evaluate(&r), Decision::Allow(vec![Obligation::Audit]));
    }

    #[test]
    fn patient_and_proxy_work_only_through_their_own_relationship() {
        let mut r = request(Role::Patient, Resource::Observation, Action::Read);
        r.relationship = Relationship::CareTeam; // having a care team link is not being the patient
        assert!(!evaluate(&r).allowed());
        r.relationship = Relationship::SelfPatient;
        assert!(evaluate(&r).allowed());

        let mut p = request(Role::Proxy, Resource::Observation, Action::Read);
        p.relationship = Relationship::SelfPatient; // a proxy is not the patient either
        assert!(!evaluate(&p).allowed());
        p.relationship = Relationship::Proxy;
        p.labels = BTreeSet::from([Label::Restricted]);
        let Decision::Allow(o) = evaluate(&p) else {
            panic!("should allow");
        };
        // Conservative default for proxies: restricted content is masked.
        assert!(o.contains(&Obligation::Mask(BTreeSet::from([Label::Restricted]))));
    }

    #[test]
    fn purpose_must_fit_the_role() {
        let mut r = request(Role::Clinician, Resource::Condition, Action::Read);
        r.purpose = Purpose::Research;
        assert_eq!(
            evaluate(&r),
            Decision::Deny {
                reason: DenyReason::PurposeNotAllowed,
                btg_offer: false
            }
        );
        let mut p = request(Role::Patient, Resource::Condition, Action::Read);
        p.purpose = Purpose::Treatment;
        assert!(!evaluate(&p).allowed());
    }

    #[test]
    fn several_roles_combine_to_the_most_permissive_single_role() {
        let mut r = request(Role::Nurse, Resource::Condition, Action::Create);
        r.subject = subject(&[Role::Auditor, Role::Nurse]);
        assert!(evaluate(&r).allowed());
        // Auditor + Nurse together still cannot export clinical content.
        r.action = Action::Export;
        assert!(!evaluate(&r).allowed());
    }

    /// The matrix: every role × resource × action × single label × relationship × consent ×
    /// break-glass, checked against the rules that must hold whatever the table says.
    #[test]
    fn invariants_hold_across_the_whole_matrix() {
        let relationships = [
            Relationship::None,
            Relationship::CareTeam,
            Relationship::ActiveEncounter,
            Relationship::SelfPatient,
            Relationship::Proxy,
        ];
        let consents = [Consent::NotRecorded, Consent::Permit, Consent::Deny];
        let purposes = [
            Purpose::Treatment,
            Purpose::Emergency,
            Purpose::Operations,
            Purpose::Payment,
            Purpose::Research,
            Purpose::PatientAccess,
            Purpose::Audit,
        ];
        let grants = [
            None,
            Some(BreakGlass {
                reason: "emergency".into(),
                expires_at: 5_000,
            }),
        ];
        let mut label_sets: Vec<BTreeSet<Label>> = vec![BTreeSet::new()];
        label_sets.extend(Label::ALL.map(|l| BTreeSet::from([l])));
        label_sets.push(BTreeSet::from(Label::ALL));

        let mut cells = 0_u64;
        let mut allows = 0_u64;
        for role in Role::ALL {
            for resource in Resource::ALL {
                for action in Action::ALL {
                    for labels in &label_sets {
                        for relationship in relationships {
                            for consent in consents {
                                for purpose in purposes {
                                    for grant in &grants {
                                        let r = Request {
                                            subject: subject(&[role]),
                                            resource,
                                            action,
                                            labels: labels.clone(),
                                            relationship,
                                            purpose,
                                            consent,
                                            break_glass: grant.clone(),
                                            now: 1_000,
                                        };
                                        cells += 1;
                                        let d = evaluate(&r);
                                        let Decision::Allow(o) = d else { continue };
                                        allows += 1;
                                        let at = format!(
                                            "{role:?} {resource:?} {action:?} {labels:?} {relationship:?} {consent:?} {purpose:?}"
                                        );

                                        // The route gate is never looser than the record decision.
                                        assert!(gate(&r.subject, resource, action), "{at}");
                                        // Every allow is audited.
                                        assert!(o.contains(&Obligation::Audit), "{at}");
                                        // A refused consent never discloses to anyone but the patient.
                                        if action.reads() && consent == Consent::Deny {
                                            assert_eq!(role, Role::Patient, "{at}");
                                        }
                                        // The patient and proxy roles never act through another relationship.
                                        if role == Role::Patient {
                                            assert_eq!(
                                                relationship,
                                                Relationship::SelfPatient,
                                                "{at}"
                                            );
                                        }
                                        if role == Role::Proxy {
                                            assert_eq!(relationship, Relationship::Proxy, "{at}");
                                        }
                                        // Clinical content needs a treating relationship, the patient's own
                                        // relationship, or an unexpired break-glass grant for a read.
                                        if resource.clinical()
                                            && !matches!(role, Role::Patient | Role::Proxy)
                                            && !relationship.treating()
                                        {
                                            assert!(action.reads() && role.treats(), "{at}");
                                            assert!(
                                                o.contains(&Obligation::NotifyPrivacyOfficer),
                                                "{at}"
                                            );
                                            assert!(grant.is_some(), "{at}");
                                        }
                                        // Break-glass is flagged on the audit row whenever it was needed.
                                        assert_eq!(
                                            o.iter()
                                                .any(|x| matches!(x, Obligation::BreakTheGlass(_))),
                                            resource.clinical()
                                                && !matches!(role, Role::Patient | Role::Proxy)
                                                && !relationship.treating(),
                                            "{at}"
                                        );
                                        // Every label the role is not listed for is masked on a read, and
                                        // nothing is masked that the role may see (SUD aside).
                                        let masked: BTreeSet<Label> = o
                                            .iter()
                                            .filter_map(|x| match x {
                                                Obligation::Mask(l) => Some(l.clone()),
                                                _ => None,
                                            })
                                            .flatten()
                                            .collect();
                                        for label in labels {
                                            let listed = label.unmasked_for().contains(&role);
                                            if action.reads() && !listed {
                                                assert!(masked.contains(label), "{at}");
                                            }
                                            if !action.reads() {
                                                assert!(masked.is_empty(), "{at}");
                                            }
                                        }
                                        assert!(masked.is_subset(labels), "{at}");
                                        // Roles with no clinical permission never see clinical content.
                                        if matches!(
                                            role,
                                            Role::Receptionist | Role::Auditor | Role::Integrator
                                        ) {
                                            assert!(!resource.clinical(), "{at}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // 10 roles × 12 resources × 6 × 8 × 5 × 3 × 7 × 2: guard against the loops silently shrinking.
        assert_eq!(cells, 10 * 12 * 6 * 8 * 5 * 3 * 7 * 2);
        assert!(allows > 0);
    }

    #[test]
    fn organisations_come_from_the_token_and_only_short_lowercase_tokens_survive() {
        let mut attrs = BTreeMap::new();
        attrs.insert(
            "organisation".to_string(),
            vec![
                "riverside".to_string(),
                "Riverside Clinic".to_string(),
                String::new(),
                "x".repeat(65),
                "a-1".to_string(),
            ],
        );
        let s = Subject::from_attrs(&attrs);
        assert_eq!(
            s.organisations
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["a-1", "riverside"]
        );
        assert!(
            Subject::from_attrs(&BTreeMap::new())
                .organisations
                .is_empty()
        );
    }
}
