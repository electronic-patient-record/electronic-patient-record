# Access control and consent

**Break-the-glass (SE-3, built 2026-10-21):** see [the changelog](../../CHANGELOG.md). A treating role with no care relationship is refused with an offer, opens a 4-hour grant with a reason, and reads clinical content; the grant lifts the missing relationship and every label (2026-11-25; three lists can carry one), and not a patient's refusal and not writes, and every audit row of the access is flagged and names the grant; each grant waits for an auditor's review. No real-time notification.

RBAC roles (clinician, nurse, pharmacist, allied health, receptionist, clinical admin, patient, proxy, auditor, integrator) plus ABAC (care team / legitimate relationship, encounter, location, purpose of use, security labels), FHIR Consent, national data opt-out, 42 CFR Part 2 via `SUD`, break-the-glass (reason, time-boxed grant, alert, review), proxy and adolescent confidentiality. Evaluated in the service on every read. Tasks SE-1 to SE-6. **Built today (SE-1, 2026-10-21): a role-based route gate on every route, and a policy engine for record-level decisions that no handler calls yet.** The token's `role` attribute (PASETO attrs, or Keycloak realm roles through `role-map.json`) is checked against a table of role × resource × action; a route with no table entry is refused; each refusal is audited. The engine also decides care relationship, security labels, purpose of use, consent and break-the-glass, and is tested across a 0.9-million-request matrix, and the table is an unreviewed first draft. **Care relationships (2026-10-21):** an administrator records who is on a patient's team, who is the patient and who is their proxy; the patient and proxy roles work only through such a record. **Consent (SE-2, 2026-10-21) is built for patient identity only:** append-only consent records with permit and deny provisions, applied to identity reads, search and the duplicate check; break-the-glass grants and masking (SE-3, SE-4) are not built, and no clinical record exists yet for consent to hide.

## Life-or-limb emergencies

**Emergencies that are life-or-limb threatening override normal access restrictions** (FR-4.6, decided by the maintainer 2026-11-25). An emergency that threatens a life or a limb is one where waiting for a restriction to be lifted in the ordinary way could cost either. The rule's way in is break-the-glass: a person says why, the access is flagged and it is reviewed afterwards. **The audit row, the reason, the sign-in and the review are not access restrictions, and nothing overrides them.**

| Restriction | Overridden by break-the-glass today |
|---|---|
| No care relationship | Yes, for a treating role reading clinical content |
| A security label, one this version cannot read included (OQ-PI2) | Yes, for a treating role, on the team or not. Three lists can carry a label: problems, relative history and typing |
| Substance-use content without the patient's consent (`SUD`) | Yes, as above. No lawyer has reviewed it against 42 CFR Part 2 |
| The patient's refusal of consent | **No.** Whether a refusal is a normal access restriction is not decided (OQ-PI8) |
| Why an appointment was missed | No: the care team and the patient (OQ-XP1). It is of no use in an emergency |
| A role that reads no clinical content (reception, an auditor, an integrator), a token's scope, the purpose of use | No |
| Writing, and exporting | No |

**What the service does not know:** the reason is free text, and nothing asks whether a life or a limb is at risk, so a grant for any other reason lifts as much (PI-10).

Planned; nothing here is built yet unless stated. Source: [`../../plan.md`](../../plan.md).
