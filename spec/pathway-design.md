# Clinical care pathway design and redesign by clinicians

**A direction from the author's instruction of 2026-11-04 (plan.md §27). Nothing here is built.** The care pathway is the organising unit of the record ([5.2.1](05-domain-model.md)). The people who know how a pathway should run are clinicians, so they, and not developers, must be able to design a pathway and to change it when it does not work.

## What a pathway template is

A reusable definition of a route through care: **steps** (a referral, a test, an appointment, a review), **decisions** (if the result is above a threshold, go here), **waits** and **timers** (expect an answer in N days), **owners** (a team per step), and **the loop-back** (what happens when a step is not answered; compare the [referral](interoperability/closed-loop-referral.md) timeouts). Applied to a patient it becomes that patient's `CarePlan` with its requests and tasks.
In FHIR the template is a `PlanDefinition`. HL7's Clinical Practice Guidelines guide layers process models (pathway, strategy, recommendation) onto BPMN, CMMN and DMN and publishes a "pathway definition" profile of `PlanDefinition`, with mappings from BPMN control-flow patterns; a bidirectional BPMN to `PlanDefinition` transformation has been published. Research notes that `PlanDefinition` is not readily understandable by clinicians and business analysts.
**No clinician-authored pathway design tool built on these standards was found.** That is a gap worth filling and a risk: the editor must speak in clinical words and hide the standard.

## Requirements (proposed)

1. **A constrained visual editor in clinical language**: steps, decisions, waits, owners; no scripting. It stores a `PlanDefinition` and shows the clinician only what they author. A small, fixed vocabulary of step types, not free-form BPMN.
2. **Templates are versioned and immutable once published.** A change makes a new version; a patient keeps the version they were started on; moving a patient to a newer version is an explicit, audited act with the diff shown, never silent.
3. **Draft, review, publish.** A named author, at least one named reviewer who is not the author, a named approver, each with a date; the review status is part of the template and is shown to the clinician using it. The project never claims a template is clinically reviewed ([translation review](../governance/translation-review.md) is the model). A change to a high-risk pathway adds or updates a hazard row first.
4. **Try it before it is live.** A simulation runs the draft against synthetic patients and shows which steps open, who owns each, and where it would wait or stall, so a loop with no owner or a step with no timer is found before a patient meets it.
5. **Redesign from evidence.** The pathway's own record shows where patients wait (time in each step), which loops are open, and which steps are skipped or overridden; a redesign starts from those counts. Aggregate analysis may use [privacy-preserving methods](privacy-analytics.md).
6. **Every pathway change is audited**: who changed what, when, from which version, reviewed by whom.
7. **No automatic clinical decisions.** A template proposes steps and reminds; the clinician decides. Decision rules that read patient data are the clinical-decision-support problem of FR-6.5 and carry its safeguards (alert fatigue, override capture).

## Three step types for a contact outside the record (SG-1)

A referral loop closes only if the recipient answers. A charity with no EPR and a service that takes self-referrals never will, so a referral to them stays open for ever, or a person believes someone is being supported when no one rang ([survivor grief](survivor-grief/index.md#referral-to-services-outside-the-health-service)). The vocabulary of step types therefore holds three, and a step says plainly what it will not tell you. A first draft: no clinician or bereavement service has reviewed these words.

| Step | Records | Closes | Says on the step |
|---|---|---|---|
| **Information given** | What was offered, to whom and on what date | At once | "This does not tell you it was read or helped." |
| **Signposted** | The named service the person was told about, and that the choice is theirs | At once | "This does not tell you the person made contact. The outcome will not be known." |
| **Referred** | The service that received it, and the person's consent | When the service answers, as a referral does today | "This does not tell you the person was seen until the service answers." |

- **Referred needs a recipient that takes referrals, and the person's consent.** No consent, no referral. A service that takes only self-referrals can be signposted to and not referred to, and the [directory](survivor-grief/index.md#referral-to-services-outside-the-health-service) says which it is.
- **The first two make no timer and no loop-back,** so no alarm appears on a worklist that nobody can clear. An unanswered step is not a stalled pathway, and the simulation in requirement 4 does not report it as one.
- **The words stay with the step**: on the patient's pathway, in a letter and in a handover, so that "signposted" is never read as "being supported".
- **A later question to the person** is optional, needs their agreement to be contacted, and is a step of its own.
- **None of these is safety cover.** A person in crisis needs a clinical route, and a template must not offer these three in its place.
- **None starts by itself.** A template proposes; a person offers (requirement 7, and [do not build](survivor-grief/index.md#do-not-build)).

Applies beyond grief, for example to a care home or a voluntary service. Nothing is built: this waits for the template model (`XP-12`).

## Order of work

Template model and versioning (`XP-12`) after the pathway basics (CA-2) and OQ-CP3; the editor (`XP-13`) after that; pathway analytics (`XP-14`) can start earlier because it reads data that exists.

## Not done, not verified

No tool was built or evaluated. The three step types are the project's reasoning from the survivor grief research, not taken from a guideline, and no one who would use them has seen them. The HL7 layering is from the guide's documentation pages and a few papers read by search result, not in full. Whether clinicians find a constrained editor usable is untested and needs summative testing with clinicians (NFR-10).
