# Roles, assignments, permissions and the operations of access

Public-source research read 2026-11-04 plus this repository's own files. A proposal and a reading, not legal or clinical advice: nothing here is built, and no lawyer, clinician or data protection officer has reviewed it.

Related: [Access control and consent](index.md), [Protected information](protected-information.md) (the companion page: what is especially sensitive and how to hold it), [Audit](../audit/index.md), [Patient identity](../patient-identity/index.md), [Care settings](../care-settings.md), [Maternity](../maternity/index.md), [Measuring clinical safety](../clinical-safety/measurement.md), the [DPIA](../../governance/dpia.md) and the [MFA plan](../../governance/mfa-plan.md). **NOT FOR CLINICAL USE.** Examples from the UK, the US and Australia are examples, never "the rule".

## Why this page exists

The service decides two things today: may this role ever do this to this kind of record (the route gate), and may this person do it to this patient's record now (the record decision). What it does not yet have is the human side: who is given a role, how a role ends, who is "on" a patient, who covers for whom, and who looks at the access log. Most real-world snooping and over-access is not a failed algorithm; it is a person who held access they no longer needed, or a log nobody read.

## Models: RBAC, ABAC, ReBAC

- **RBAC.** The NIST/ANSI role-based access control standard (INCITS 359, adopted 2004, revised 2012) has a reference model of four components: core RBAC, hierarchical RBAC (role inheritance), static separation of duty (a user may not hold conflicting roles) and dynamic separation of duty (a user may hold them but not activate them together in one session). Core is required; the rest are independent. I saw the NIST FAQ (archived) and secondary summaries, not the 2012 text [NISTRBAC].
- **ABAC.** NIST SP 800-162 (January 2014, updated August 2019) defines it as deciding an operation by checking attributes of the user, the resource, the action and sometimes the environment against rules [NISTABAC]. This is what the policy engine's record decision does.
- **ReBAC and the HL7 catalogue.** HL7's Healthcare (Security and Privacy) Access Control Catalog Release 3 (ANSI/HL7 V3 HACC R3-2016, reaffirmed 2021) extends the earlier HL7 RBAC permission catalogue with concepts for ABAC and relationship-based access control; permissions are {operation, object} pairs [HLHACC]. A relationship-based rule ("the clinician is on this patient's team") is the heart of clinical access.
- **XACML** (the OASIS policy language) was asked about and **not researched** in this pass; see Not verified.

## The UK NHS model, as publicly described

- **Smartcard RBAC.** Users hold job roles ("R" codes, for example Clinical practitioner R8000) and each role maps to activities ("B" codes, for example amending patient demographics B0825). A national baseline maps roles to activities. A user can hold several roles; staff on several sites need role codes for each [NHSRBAC].
- **Position-based access control (PBAC).** Access is narrowed further to the data items or functions a position needs, and to the places the card may be used [NHSRBAC]. Trust policies describe access as reflecting organisation, role, area of work and activities (snippet of one trust's SOP) [HUMBER].
- **Registration Authority (RA).** A local RA verifies identity before issuing a card, manages access, removes organisational access when staff leave or go on extended leave and reinstates it on return [NHSRBAC]. Trust SOPs describe sponsors who authorise new users, changes to access and terminations (snippet) [HUMBER]. The page I read did not mention sponsors itself.
- **Legitimate relationship.** NHS England's Summary Care Record guidance says a record should be viewed only by someone involved in the patient's care, and calls this a "legitimate relationship"; it also asks staff to ask the patient's permission to view, and says some roles can self-claim the relationship (seen through a mirrored page and a search snippet) [SCR]. So the NHS test is a declared relationship plus a role, with audit, not a pre-built roster.
- **Audit.** Access is traceable to the card holder [NHSRBAC]. The page did not mention routine log review.

## How commercial systems model it (public descriptions only)

I found no vendor design documents. Public hospital pages show the practical shape in an Epic setting: a record can be flagged (employee, VIP, confidential encounter); opening a flagged record asks for a reason, logs it and can notify compliance; a compliance office reviews each such access; and a separate monitoring tool flags "snooping" such as access to a colleague's chart, manager snooping and bypassed prompts [EPICBTG] [TCH] [FW]. The same pages describe security "points" and templates only in passing, and I did not read how they are built. A vendor page that automated the break-the-glass reason with a stock code is a warning sign, not a model [EPICBTG].

## Operations of access (the day-to-day)

Each item gives what good practice looks like in public sources, then a proposal.

1. **Joiners.** Identity is verified, a sponsor with authority says which role and which organisation, and access is granted to the position, not the person's wishes (NHS RA model [NHSRBAC] [HUMBER]). HIPAA's Security Rule expects procedures to authorise access and to check that a workforce member's access is appropriate (45 CFR 164.308(a)(3) and (4)) [HIPAA].
2. **Movers.** A change of ward, team or employer must remove the old access, not add to it. Role creep is the usual failure. Proposal: a role assignment has an end date and a reason, and a move creates a new assignment and ends the old one.
3. **Leavers.** Termination procedures are an addressable HIPAA specification [HIPAA]; the NHS RA removes access on leaving [NHSRBAC]. Proposal: leaving ends every assignment the same day and the audit shows no use afterwards.
4. **Periodic recertification.** A named owner re-confirms each assignment on a schedule (for example every six months for privileged roles). I found no public NHS or HIPAA source naming an interval; any figure is a proposal.
5. **Separation of duties.** From NIST RBAC [NISTRBAC]: the person who grants access should not be the person who gains it (the project already does this for care relationships); the person who opens a break-glass grant must not review it (already built); the auditor should not hold clinical roles; the person who lifts a protected label should not be the person it protects or concerns.
6. **Delegation and temporary cover.** A clinician going on leave hands a list to a named colleague for a bounded period. Proposal: a "cover" assignment with start, end and the patients or team it covers, shown to the patient in the access log as cover, not as the person's own team.
7. **Locums, agency staff and students.** No public source read. Proposal: time-limited assignment to one organisation, a supervisor named on every student action (the supervisor co-signs), and the student role has no clinical sign-off.
8. **Emergency access.** Break-glass: a reason, a time-boxed grant, a flagged audit row and a later review. The project has this; the Epic-style practice adds a real-time notice to compliance [EPICBTG]. Australia's My Health Record lets providers reach a record restricted by a patient's access code in an emergency (search snippet of the OAIC and Digital Health Agency pages) [MHR].
9. **Privileged and administrative access.** Administrators of the system, the database and the identity provider can see more than any clinician. Proposal: no standing access to clinical content, access granted per task, MFA on every administrative sign-in ([MFA plan](../../governance/mfa-plan.md)), and every use audited and reviewed by someone else.
10. **Service accounts.** The `integrator` role. Proposal: one account per integration, no human use, no clinical read, key rotation, an owner, and the same recertification as a person.
11. **Access to your own record, family and colleagues.** Staff opening their own, a relative's or a colleague's record is the commonest misuse. Proposal: treat "same surname", "same address", "same employer" as automatic flags for a reason prompt; staff records are an "employee" class like the HL7 EMP and CEL sensitivity codes ([Protected information](protected-information.md)).
12. **Proactive privacy monitoring.** A log nobody reads protects no one. HIPAA makes routine review of audit logs and access reports a required specification (164.308(a)(1)(ii)(D)) and audit controls a required standard (164.312(b)) [HIPAA]. In the UK, unlawful obtaining of personal data is an offence under section 170 of the Data Protection Act 2018; the ICO has prosecuted former NHS staff for looking at records of people they were not treating, usually in magistrates' courts [ICO170] [DACB]. A 2026 news item reports NHS England telling trusts to suspend staff suspected of snooping; I could not verify it independently [LGL]. Proposal: scheduled reports (same-surname access, high-volume access, off-hours, after-leave, break-glass reasons repeated, access to flagged records) and a recorded outcome for each reviewed item.

## A catalogue of clinical roles and sub-roles (a proposal)

Typical permissions are illustrative; each needs clinical governance. "Rel" means a care relationship is needed.

| Role family | Sub-roles (examples) | Typical permissions beyond the family baseline |
| --- | --- | --- |
| clinician | consultant, registrar, foundation doctor, GP, locum | prescribe (rights vary by grade), sign discharge, withdraw own entries; trainee entries co-signed |
| nurse | ward nurse, specialist nurse, nurse prescriber, midwife, health visitor | record care; midwife holds maternity pathway ([Maternity](../maternity/index.md)); prescribing only if qualified |
| pharmacist | ward, dispensing, prescribing pharmacist | medication read/act; clinical check |
| allied-health | physiotherapist, occupational therapist, dietitian, radiographer, psychologist | own-profession notes |
| receptionist / ward clerk | reception, ward clerk, medical secretary | identity and appointments; secretary may type letters but sees clinical text only for dictated letters (needs a role of its own) |
| clinical-admin | registration, health records, coder, bed manager | identity, consent and relationship records; coders read coded diagnoses for payment, not notes |
| patient / proxy | adult, child, parent, carer with lasting power of attorney | own record; scopes per proxy |
| auditor | privacy officer, information governance analyst | audit trail; no clinical content |
| integrator | ADT feed, lab interface, referral gateway | create/search identity only |
| clinical-safety-officer | CSO | hazard register |
| on-duty-clinician | the clinician on duty, for a shift, alongside `clinician` | accountable for reports of a death that wait; the death work and nothing else on its own (OQ-BV4). Given and taken away in the identity provider, by hand until a workforce system does it: the rota belongs to workforce planning management and this service will not hold one |
| medical-devices-safety-officer | MDSO | hazard register: read, raise |
| medication-safety-officer | MSO | hazard register: read, raise |
| patient-safety-officer | PSO | hazard register: read, raise |
| medical-incident-officer | MIO | hazard register: read, raise |
| (missing) | medical student, nursing student, agency or locum | time-boxed, supervised |
| (missing) | privacy officer, data protection officer, Caldicott Guardian-style role | decides on labels and complaints; may be the person who lifts a protected label |
| (missing) | break-glass reviewer, label steward, system administrator | see operations above |

## Safety officers

Four roles beside the Clinical Safety Officer, added on 2026-11-06 at the maintainer's request. **What each may do is a first draft by an engineer**, and nobody who holds such a post has reviewed it.

| Role | What the post is, as far as was found |
|---|---|
| Medical Devices Safety Officer (MDSO) | An NHS post from the 2014 NHS England and MHRA alert on device incidents: responds to requests about device incident reports and sits on the safety committee ([handbook](https://www.pslhub.org/learn/professionalising-patient-safety/medical-device-safety-officer-handbook-the-first-stop-for-professional-medical-device-advice-r4563/)) |
| Medication Safety Officer (MSO) | The matching post for medication errors, usually a senior pharmacy professional, and it may be the same person as the MDSO ([CQC](https://cqc.org.uk/node/3888)) |
| Patient Safety Officer (PSO) | No formal NHS definition was found. Job adverts describe oversight of a patient safety programme and of incident investigation |
| Medical Incident Officer (MIO) | No single national definition was found. It appears in hospital and ambulance major incident plans as the senior doctor at or for an incident ([EMJ](https://emj.bmj.com/content/16/5/336)) |

Each reads the hazard register and may raise a hazard, for the purpose of operations only. None sees clinical content, records a control, reopens or retires a hazard, or uses break-the-glass, and only the Clinical Safety Officer retires one. Nothing else is open to them: **an incident system, a device list and a medication error report do not exist here**, so these roles have little to do yet, and an MSO cannot see a prescription. A Medical Incident Officer in a major incident gets no emergency access from this role, because a person treating patients holds a clinical role for that.

These sources were read through search results, not in full, and the 2014 alerts themselves were not read. One person may hold several of these roles.

## Compared with this project

| Need | This project today | Gap |
| --- | --- | --- |
| Roles | 16 roles in `policy.rs`; unknown roles grant nothing | No sub-roles (consultant, midwife, ward clerk, student), no secretary, coder, privacy officer or administrator role |
| Role grant | Role comes from the token (PASETO attrs or Keycloak realm roles via `role-map.json`) | No assignment record in this service: no sponsor, no start or end date, no reason |
| Patient assignment | Care relationships (care team, active encounter, self, proxy) created by `clinical-admin`; the subject cannot grant themselves | No team model, no cover or delegation, no automatic end at discharge or move, no relationship by ward or service |
| Permissions | Route gate table role x resource x action; record decision with labels, purpose, consent, break-glass | Table unreviewed clinically; no field-level or minimum-necessary views |
| Joiners, movers, leavers | None in this service (identity provider holds users) | No JML process, no leaver evidence |
| Recertification | None | No periodic review of assignments or relationships |
| Separation of duties | Admin grants relationships; reviewer of break-glass is someone else | No static conflict rules between roles in one token |
| Emergency access | 4-hour read-only grant, reason, flagged rows, review queue; no real-time notice | No notification; review deadline (72 h) is a guess |
| Service accounts | `integrator` role, identity only | No ownership, rotation or review record |
| Own, family, colleague records | Not handled | No same-surname or employee flag |
| Privacy monitoring | Audit trail and verifier; auditor search and export | No scheduled analytics, no outcome record for reviewed items |
| Patient sees who looked | Patient access log shows roles and organisations, not names | Whether names should show for protected records is open |

## Candidate hazards

For the Clinical Safety Officer; none is accepted or closed by this page.

1. **Stale access after a move or departure.** Cause: no assignment end date or leaver link; the person still reads records they no longer treat.
2. **Over-broad role.** Cause: a role grants more than a sub-role needs (for example a receptionist role also used by a medical secretary), so a clerk sees clinical text, or a clinician cannot do a legitimate task and works around the system.
3. **Cover gap.** Cause: no delegation, so a colleague shares a login to cover absence, and the audit names the wrong person.
4. **Unreviewed break-glass.** Cause: grants wait in a queue nobody works; misuse is found late.
5. **Admin or service account misuse.** Cause: standing privileged access and unowned service accounts.
6. **Snooping on colleagues, relatives and public figures.** Cause: no prompt, no flag and no monitoring for these classes.
7. **Student or locum acting beyond role.** Cause: no supervised, time-boxed role; their entries look like a qualified person's.
8. **Access refused in an emergency.** Cause: over-strict controls with no workable break-glass for a legitimate treating role, delaying care.

## Ideas

1. Add a role assignment record (user, role, organisation, sponsor, start, end, reason, ended-by), append-only like the care relationship, with `policy::Subject` built from live assignments.
2. Add sub-roles as a second tier under the 11 families, mapped to the family for the route gate and to extra rights for the record decision.
3. Model teams (ward, service, pathway) and let a relationship derive from team membership, with an end date.
4. Add a cover assignment with a mandatory end date and a distinct audit marker.
5. Add a recertification report: assignments and relationships older than N days, grouped by owner, with confirm or end actions recorded.
6. Add conflict-of-role rules (static separation of duty) and test them in the existing role x resource matrix.
7. Add a student and locum role with supervisor co-sign and automatic expiry.
8. Add monitoring reports (same surname, same address, employee records, volume, after-leave use) feeding the auditor, with a recorded outcome.
9. Add a real-time notice to a privacy officer for break-glass and for opening a flagged record.
10. Add the missing roles: privacy officer, system administrator (no clinical content), label steward.
11. Use the HL7 permission vocabulary where it fits, so the table can be compared with a published catalogue.
12. Measure the controls as in [Measuring clinical safety](../clinical-safety/measurement.md): proportion of assignments recertified on time, time from leaving to access removed, proportion of break-glass reviewed within the deadline.

## Open questions

1. Who is the sponsor in a deployment without an NHS-style Registration Authority, and where do role assignments live: this service or the identity provider?
2. Is a team membership a relationship in itself, or only a way of proposing one for an administrator to confirm?
3. How long may cover last, and may a delegated person see what the absent person could not?
4. Should staff see that their own access is monitored (a deterrent) and can a patient see the reviewer's outcome?
5. Which sub-roles need rights the family does not give, and who in clinical governance decides?
6. What review interval is defensible for each role, and who audits the auditors?

## Not verified

- I did not read INCITS 359-2012, NIST SP 800-162 beyond its abstract, XACML, the HL7 catalogue itself or the NHS RBAC database; summaries only.
- The NHS England smartcard page I read did not describe sponsors or legitimate relationships; those come from trust SOP snippets and a mirrored SCR page. I did not find how legitimate relationships are configured in NHS systems.
- No vendor design documents on Epic, Cerner or others were read; only hospital pages about break-the-glass and monitoring.
- Locums, agency staff, students, delegation and recertification intervals: no source read; proposals only.
- The 2026 report of NHS England suspending suspected snoopers is a search result I could not confirm.
- The regulations on health data in any other country than those cited were not read; HIPAA citations come from a Cornell mirror and a search summary, not the eCFR.
- Nothing here was tested against the code beyond reading `policy.rs`, `relationships.rs` and `break_glass.rs` docs.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://ico.org.uk/about-the-ico/media-centre/news-and-blogs/2023/11/former-nhs-secretary-found-guilty-of-illegally-accessing-medical-records

## Sources

- [NISTRBAC] NIST RBAC FAQ (archived): https://webarchive.library.unt.edu/eot2008/20080916003800/http:/csrc.nist.gov/groups/SNS/rbac/faq.html (search result; summary only)
- [NISTABAC] NIST SP 800-162: https://csrc.nist.gov/pubs/sp/800/162/upd2/final (abstract and date seen)
- [HLHACC] HL7 Healthcare Access Control Catalog R3, via the Australian Digital Health Agency: https://implementer.digitalhealth.gov.au/standards/hl7-version-3-standard-healthcare-security-and-privacy-access-control-catalog-release-3 and https://standups.hl7.org/2021/08/18/normative-publication-of-hl7-version-3-standard-healthcare-security-and-privacy-access-control-catalog-release-3-reaffirmation-of-ansi-hl7-v3-rbac-r3-2016/ (snippets)
- [NHSRBAC] NHS England, Smartcards and role-based access control: https://www.england.nhs.uk/long-read/smartcards-and-access-controls/ (read)
- [HUMBER] Humber Registration Authority SOP: https://intranet.humber.nhs.uk/Policies/Corporate%20Policies/Corporate%20SOPs/Registration%20Authority%20SOP19-010.pdf (snippet)
- [SCR] NHS England SCR viewing guidance (mirror): https://nhsd-proxy.openprescribing.net/services/national-care-records-service/guidance/view-scr (snippet)
- [EPICBTG] University of Iowa Epic break glass: https://epicsupport.sites.uiowa.edu/epic-resources/break-glass (snippet)
- [TCH] Texas Children's: https://texaschildrensnews.org/compliance-and-privacy-spotlight-break-the-glass-epic/ (snippet)
- [FW] UConn FairWarning FAQ: https://health.uconn.edu/healthcare-compliance-privacy/wp-content/uploads/sites/253/2020/10/FairWarning-FAQ-for-Managers.pdf (snippet)
- [HIPAA] 45 CFR Part 164 Subpart C appendix A: https://www.law.cornell.edu/cfr/text/45/part-164/subpart-C/appendix-A (search summary)
- [ICO170] ICO: https://ico.org.uk/about-the-ico/media-centre/news-and-blogs/2023/11/former-nhs-secretary-found-guilty-of-illegally-accessing-medical-records (snippet)
- [DACB] DAC Beachcroft case note: https://www.dacbeachcroft.com/en/what-we-think/former-nhs-employee-prosecuted-and-fined-after-illegally-accessing-patient-records-without-a-valid-l (snippet)
- [LGL] Local Government Lawyer: https://localgovernmentlawyer.co.uk/employment/395-employment-news/101787-nhs-england-orders-immediate-suspension-for-staff-suspected-of-snooping-on-patient-records-2 (snippet)
- [MHR] OAIC My Health Record access controls: https://www.oaic.gov.au/privacy/guidance-and-advice/my-health-record/my-health-record-access-controls/ (snippet)
- This repository: `electronic-patient-record-service-with-rust/src/policy.rs`, `relationships.rs`, `break_glass.rs`; [Access control](index.md); [Audit](../audit/index.md).
