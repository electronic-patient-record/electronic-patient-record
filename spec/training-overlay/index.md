# Training overlay: learn on the real screens, safely apart from real patients

**A design from the author's instruction of 2026-11-04 ("important: build a training overlay"). Nothing is built yet; the first slice is task `TO-1`.** Research on the kinds of training is in [training](../training/index.md); how adoption is measured is in [productivity measurement](../productivity-measurement/index.md).

## What it is

Two separable parts, on one rule: **training never touches real patients, and nothing a learner sees can be mistaken for real care.**

1. **A training environment.** A deployment (or tenant) started in training mode, holding only **synthetic** patients, in which every screen works as in production. Safe to make mistakes in.
2. **A guidance overlay.** A layer on top of the real screens that walks a learner through a scenario ("register a patient, record an allergy, send a referral"), highlights the control to use, explains why, and checks that the step was done. It is *on top of* the real UI, not a copy of it, so it cannot drift from what users will really see.

## The rule that matters most: no confusion with real care

A training mode that looks like production is a patient-safety hazard (a real clinician acts on a made-up patient, or training data reaches a real record). So:

1. **Separation is in the service, not only the page.** The service reads `EPR_ENVIRONMENT` (`production` or `training`; **no default**, so a deployment must say which it is, like the referral timeouts). In `training` it refuses to start if it is pointed at a database that holds non-synthetic marks, and every audit row carries the environment.
2. **Training data is marked synthetic at the source.** A seeded patient carries a synthetic tag in its data; a production service refuses to start if it finds one, and a training service refuses to import a record without it.
3. **Nothing leaves.** In training the outbox, the MLLP listener and every outbound route are off, whatever is configured; sign-in uses a separate realm so a training account cannot sign in to production and the reverse.
4. **The page cannot hide it.** A persistent banner "TRAINING: NOT REAL PATIENTS" on every page (including print views and exports), a different colour scheme, and the browser tab title prefixed. The overlay cannot dismiss it. A training screenshot is visibly training.
5. **The overlay never covers safety information.** The patient banner, allergies and alerts stay visible and reachable while a step is highlighted ([5.2.1](../05-domain-model.md), [mobile viewability](../mobile-viewability.md)); a step that would hide them moves the explanation instead.
6. **It is off in production** except where a site turns on *just-in-time help* (below), which never blocks a control and never changes data.

## The overlay

- **Scenarios** are data, not code: an ordered list of steps, each with the screen, the control to highlight (by a stable test id the UI already needs for its tests), the text, and a **check** (the API call that should have happened, or a field that should now hold a value). A scenario names the role it is for.
- **Guided mode:** the overlay highlights one control at a time; **free mode:** the learner works and the overlay only checks the end state ("did the referral get sent?").
- **Hints and reasons:** every step can say why (the safety reason as well as the click), because the aim is competence, not clicking.
- **Progress and sign-off:** a learner's progress and a trainer's sign-off are recorded in a **separate competency record** about the staff member, not the clinical record; it is staff personal data (see [privacy](../../docs/privacy/index.md) and the personal-data inventory) with a stated retention, visible to the learner and their trainer.
- **Reset:** a scenario can be reset to a known synthetic state, so a class starts the same each time.
- **Accessibility and phones:** keyboard-operable, focus-managed, readable at 320 CSS px, respects reduced-motion, strings through the same translation path as the UI (machine-written until a speaker reviews them; never claimed reviewed).
- **Authoring:** scenario packs are written and reviewed by clinicians and trainers, in the same way as pathway templates ([pathway design](../pathway-design.md)); each carries a version, a reviewer and the UI version it was written against.

## Just-in-time help in production (later)

Optional, per site: a "?" that opens a short, read-only explanation of the current screen. It does not highlight, block, check or record anything, and it is off unless the site turns it on.

## Hazards (candidate rows for the Clinical Safety Officer, never accepted)

Training mistaken for real care; training data reaching a real record or the reverse; a real account used in training or a training account in production; the overlay hiding the patient banner, an allergy or an alert; a scenario that teaches a step that is wrong after a release (stale content); a hint that conflicts with local policy; staff competency data used for performance management without agreement.

## Order of work

`TO-1` environment flag, synthetic marks and the banner (small, safety-first, the foundation). `TO-2` seed and reset a synthetic dataset. `TO-3` scenario format and the overlay component with one scenario (register a patient). `TO-4` checks and progress record. `TO-5` more scenarios, written with a trainer. `TO-6` just-in-time help.

## Not done, not verified

Nothing is built. No trainer, clinician or learner has seen this. The mechanism of stable test ids, the sign-in realm split and the audit field are designs, not checked against the code beyond the facts that the UI has stable `data-testid`-style hooks in its tests and that `EPR_*` start-up checks exist for other settings.
