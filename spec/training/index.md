# Training: what kinds will be available

Public-source research read 2026-11-04 plus this repository's own files. A proposal: nothing here is built, and no clinician or trainer has reviewed it.

The question: what kinds of training should a site adopting this EPR expect, what does public evidence say about how much is enough, and what can an open-source project provide itself? Public material is mostly trust newsletters, supplier pages and job advertisements; peer-reviewed evidence on training is thin, and no AMIA or HIMSS training guidance was found (see Not verified).

## Kinds of training seen in public sources

- **Role-based classroom.** Training by job role is the norm in the UK health service programmes and supplier descriptions seen. Epic describes (through third-party guides) credentialed trainers who deliver a curriculum written by a principal trainer, must pass testing and teach-backs before training others, and specialty trainers for areas such as anaesthesia, oncology and radiology.
- **E-learning.** Used for refreshers and for staff who cannot attend; the sources seen give no evidence of effectiveness.
- **Simulation and sandbox.** A practice environment that mirrors the live system is widely recommended (trade guidance). A cancer centre reported two mandatory one-hour simulation sessions after classroom teaching (trade article).
- **Super-users.** Frontline colleagues given extra training, first point of contact on their ward, and a link to the project. One trust placed a super-user in every ward and department over go-live; a large programme reported about 3,000 super-users and several hundred floorwalkers for about 30,000 staff.
- **Floorwalkers (at-the-elbow support).** Support staff present on the ward in early live use. One trust kept them on one service for the whole early-life period for continuity and had them log the issues they resolved. Oracle Health says its adoption coaches support customers before, during and after go-live.
- **Go-live command centre.** One trust ran command centres for five weeks after go-live; a practitioner guide describes daily huddles (vendor-style source).
- **Just-in-time help.** Quick reference guides, job aids and in-app help; one job advertisement describes rapid refresher training driven by incident and service-desk trends.
- **Competency sign-off before access.** Training completion is tracked against access: a trust reported 97% completion of staff needing access by go-live day, another targeted 95%. Whether access is withheld until sign-off varies and was not confirmed in the sources.
- **Downtime training.** The ONC SAFER contingency planning guide lists training and testing for downtime and recovery. Most training sources seen do not mention downtime at all.
- **Refresher and upgrade training.** An upgrade-training study used Epic's own proficiency data (below).
- **Patients and proxies, administrators and integrators, accessibility, languages.** No public source seen describes these for any large EPR supplier, which is itself a finding.

## Evidence on length and proficiency

- No study found links a number of pre-go-live hours to later proficiency. A trade playbook proposes super-user training from five to six months before go-live and end-user training in the final four to six weeks (a vendor-adjacent source, not evidence).
- A study compared 133 ambulatory clinicians who attended evidence-based upgrade training with 14 who did not, using Epic Signal proficiency data; the result was not seen.
- A family medicine residency study found a 10-hour programme raised proficiency but not efficiency (research brief; seen only as a search result).
- A review cited in search results says the best timing (heavy before go-live, or light before and heavy after) is unsettled, and one source claims post-go-live support matters more than pre-go-live training. Neither claim was checked at source.
- A Veterans Affairs Cerner study found EHR-use metrics improved over 12 months while users still reported usability problems, so training alone does not remove the burden of a hard-to-use system.

## Compared with this project

| Need | This project today | Gap |
|---|---|---|
| Role-based curriculum | None. Roles exist in [stakeholders](../03-stakeholders-and-users.md) and access control | No role-to-lesson map |
| Practice environment | Seeded and synthetic data are used in tests; the privacy spec plans a synthetic-data spike. No named training environment | No isolated training instance, no reset |
| Training patients | Test fixtures only | No curated synthetic patients covering specialties |
| Competency sign-off before access | Access control exists ([access control](../access-control/index.md)) | No training-completion attribute that gates a role |
| Just-in-time help | None in the UI spec | No in-app help or tour |
| Downtime training | FR-8.7 downtime snapshot is specified; a data-age notice exists on one page | Not built beyond that; no downtime drill material |
| Super-user and floorwalker support | None | No role definition, no issue log |
| Accessibility | WCAG 2.2 AA targeted ([usability](../usability/index.md)) | Not shown to be met; no accessible training formats |
| Languages | Nine locales; machine-written until a speaker reviews them ([locales](../locales.md)) | No reviewed translation; training text would inherit this |
| Patient and proxy help | Portal with proxy access specified (FR-8.5), not built | No patient guide |
| Administrator and integrator training | Service and API specs, OpenAPI | No tutorials or walkthroughs |

## Candidate hazards

For the Clinical Safety Officer to assess; none is accepted, and none is yet in the hazard log.

- **TR-H1** A user acts in the live system without training and enters or reads wrongly (wrong patient, wrong allergy).
- **TR-H2** The training environment is mistaken for live (or live data leaks into it), so a real patient is trained on or a trainee changes a real record.
- **TR-H3** Training material goes out of date after an upgrade, so staff learn a workflow that no longer exists.
- **TR-H4** Staff are signed off on completion, not competence; a click-through certificate.
- **TR-H5** Downtime procedures are never practised, so paper fallback fails when it is needed.
- **TR-H6** Machine-written translated training teaches a wrong instruction.
- **TR-H7** Over-reliance on a few super-users who leave or burn out.
- **TR-H8** Training pushes staff to accept alerts or defaults without reading them.

## Ideas

1. A role-to-task matrix for each setting ([care settings](../care-settings.md)), from which lessons are listed rather than invented.
2. A training instance that is a separate deployment, visibly branded, with a persistent banner and no route to the live data store.
3. A synthetic-patient set covering common scenarios, with a reset command and a documented generator; no real person's data.
4. Timed scenario scripts that double as both a lesson and the critical-task measurement in [productivity measurement](../productivity-measurement/index.md).
5. In-app help linked to the page and role, with the same text as the documentation so there is one source.
6. A training-status field on a user that a site may use to gate a role (a site policy, not a fixed rule).
7. A downtime drill pack using the downtime snapshot and paper forms.
8. Accessible formats: text first, captions on any video, and no training that only works with a mouse.
9. Release notes written for trainers, listing what changed for each role.
10. Translations labelled machine-written until reviewed, with a reviewer field per language.

## Open questions

- OQ-TR1 Is gating access on training a project feature or a site policy?
- OQ-TR2 Who certifies a trainer, given there is no vendor programme?
- OQ-TR3 What evidence will show training worked: proficiency score, error rate, time, or all three?
- OQ-TR4 How are low-resource sites served if they cannot staff floorwalkers (equity rule, [scope](../02-scope.md))?
- OQ-TR5 What is the right minimum for patients and proxies who cannot use the portal?

## Not verified

- No AMIA or HIMSS training guidance was found by search; nothing is claimed about either.
- No national standard from England's national health body for EPR training or super-users was found; the UK examples are single-trust documents.
- Epic and Oracle practices come from third-party guides, job advertisements and one Oracle services page, not from their training catalogues.
- The upgrade-training study (133 against 14) and the residency study were seen as search results only; results and URLs-to-claims mapping should be checked.
- Completion percentages are single-trust reports, not audited.
- Nothing was found on patient or proxy training, administrator or integrator training, or accessibility of training.
- Link check 2026-11-04: these cited pages returned 404. They may have moved, but what rests on them is unconfirmed: https://healthit.gov/sites/default/files/safer/guides/safer_contingency_planning.pdf

## Sources

- Epic super-user description (third party): https://epicadventure.womans.org/resources/super-users/
- Epic credentialed and specialty trainers (third party): https://docs.dualcode.com/epic/credentialed-specialty-trainers-1
- Oracle Health services page: https://www.oracle.com/lt/health/services
- Hospital trust super-user poster (Milton Keynes): https://digital.mkuh.nhs.uk/wp-content/uploads/2018/03/Super-User-Briefing-Poster.pdf
- Floorwalker case study: https://it-works-health.sites.sourceflow.co.uk/case-studies/floorwalker
- Queen Victoria Hospital super-users: https://www.techuk.org/insights/guest-insights/the-power-of-super-users-and-how-clinical-champions-enabled-a-safe-epr-go-live-at-queen-victoria-hospital
- Manchester HIVE webinar slides: https://swdhcresources.blob.core.windows.net/$web/events/231013%20-%20Manchester%20HIVE%20webinar%20-%20Slides.pdf
- EHR training readiness playbook (trade): https://ehrsource.com/articles/ehr-training-readiness-playbook/
- Go-live ingredients (trade): https://healthdatamanagement.com/list/10-key-ingredients-to-a-successful-ehr-go-live
- Upgrade training and proficiency (search result): https://www.frontiersin.org/journals/digital-health/articles/10.3389/fdgth.2021.689646/pdf
- Residency training research brief (search result): https://journals.stfm.org/primer/2025/stewart-2025-0026/
- Veterans Affairs Cerner transition, JGIM: https://link.springer.com/10.1007/s11606-023-08318-w
- ONC SAFER contingency planning guide: https://healthit.gov/sites/default/files/safer/guides/safer_contingency_planning.pdf
- This repository: [usability](../usability/index.md), [locales](../locales.md), [care settings](../care-settings.md), [access control](../access-control/index.md), [productivity measurement](../productivity-measurement/index.md)

## A training overlay

Requirements drawn from the research above. The design belongs in [the training overlay spec](../training-overlay/index.md), written separately.

1. It must be unmistakable that a user is in training, on every screen, and training must have no path to live patient data (TR-H2).
2. It must use synthetic patients only, with a documented generator and a reset (idea 3).
3. It must follow the role and setting of the trainee, so lessons match the work (role-based practice).
4. It must support scenario scripts that can be timed and scored, to give both a competence check and a measurement (TR-H4).
5. It must include downtime drills (SAFER contingency guidance; TR-H5).
6. It must show help in context, from the same text as the documentation (just-in-time help; TR-H3).
7. It must record completion and competence per person, in a form a site may use to gate access, and say which it is (TR-H4, OQ-TR1).
8. It must be updated with each release, with change notes by role (upgrade training; TR-H3).
9. It must meet WCAG 2.2 AA and work without a mouse.
10. It must label each language as machine-written until a speaker has reviewed it, and never claim review that has not happened (TR-H6).
11. It must be usable by a super-user or floorwalker, with a log of issues resolved.
12. It must run on modest hardware and offline-capable where possible, for low-resource sites (equity rule).
