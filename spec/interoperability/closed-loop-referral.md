# Closed-loop referral: a draft state model

**Draft, proposed 2026-11-02.** The table below is written as a pure function in `src/referral.rs` (PC-1) with tests; nothing calls it yet, so no referral is stored or shown. It sharpens task CA-3 (referrals with closed loop) and PA-14 (outcomes beyond accept and reject) from one outside source, read in part: the *Care Coordination and 360 Referrals* proof-of-concept FHIR guide ([home](https://kevinmayfield.github.io/PatientCareCoordination/en/index.html), v0.1.0, "Draft as of 2026-11-02", one author, no licence stated). That guide says it is "not an authoritative source" and "must not be used for implementation or clinical purposes", and it marks several of its own rules "To confirm". Its authority is IHE's 360X profile, which I have **not** read. See [`plan.md` §24](../../plan.md). Copy nothing from it without finding its licence.

## The loop

A clinician (the **referrer**) sends a request; a service (the **recipient**) accepts or declines, may book, may report in between, and ends with an outcome that tells the referrer what happened. The loop is closed only when the referrer receives a decline or an outcome. The EPR owns the clinical request and the loop; referral administration stays with a PAS ([`../02-scope.md`](../02-scope.md), PA-1).

## Proposed states

| State | Entered by | Left by |
|---|---|---|
| `requested` | the referrer sends the request | accept, decline, `information-requested`, `redirected`, a cancel request |
| `accepted` | the recipient accepts | `appointment-booked`, `interim-report`, `completed`, `declined`, `deferred`, a cancel request |
| `declined` | the recipient declines, **at any time, including after acceptance** | (end) |
| `appointment-booked` | the recipient books | `no-show`, `interim-report`, `completed` |
| `no-show` | the recipient records a missed appointment | `appointment-booked` (rebooked), `declined` |
| `interim-report` | the recipient sends a note while care continues; **repeatable** | `interim-report`, `completed` |
| `completed` | the recipient sends the outcome | (end) |
| `cancel-requested` | the referrer asks to cancel | `cancelled` (the recipient confirms), `completed` (the recipient reports an outcome instead) |
| `cancelled` | the recipient confirms | (end) |
| `information-requested` *(provisional)* | the recipient needs more | `requested` when it is supplied |
| `deferred` *(provisional)* | the recipient puts it on hold | `accepted`, a cancel request |
| `redirected` *(provisional)* | the recipient sends it elsewhere | (end; a **new** referral with a **new** id replaces it) |

The three provisional states are marked "To confirm" in the source, as is who raises the replacement referral. Treat them as proposals.

## Rules (each from the source, to be checked against 360X)

1. **One referral id, set by the referrer, never changed.** Every later message and every report carries it. A local second id (England's UBRN is one) is a pack's identifier system, not core ([`../02-scope.md`](../02-scope.md) equity rule).
2. **A cancellation is a request.** The recipient may not honour it and may report an outcome instead. The referrer's screen must say "cancellation requested" and never "cancelled" before confirmation (hazard to add before building).
3. **A decline can arrive after acceptance.** The referrer's worklist must show it.
4. **Status is explicit.** The source faults a national standard for inferring status from combinations of resource statuses; store one status and refuse a transition the table above does not list.
5. **An outcome that names no referral is not filed silently.** The source's most common gap is a letter that does not say which referral it answers. It goes to a queue of unmatched outcomes for a person, like a refused feed message.
6. **Do not copy a flag's detail into a referral.** A safeguarding flag points to a plan its owner holds ([`flags`](#patient-flags-an-idea-not-a-design)); the referral says that a flag exists and who to ask.
7. **Time is not in the source.** It has no timeout, retry, acknowledgement or escalation rule. Ours, from CA-3: no response after N days, accepted with no outcome after M days, and an appointment date passed with no outcome, each show on the referrer's worklist (PC-3).

## FHIR mapping the source proposes

`ServiceRequest` for the request (the referral id is `identifier`; the form is `supportingInfo`); `Task` for the state (`basedOn` the request; `businessStatus` for states FHIR's `Task.status` lacks; `on-hold` for `deferred`); `Appointment` (`basedOn` the referral); `Composition` for an interim note and for the outcome (the outcome as a HL7 Europe Hospital Discharge Report document). The source gives **no table from our states to `Task.status`**; that is ours to write and to test.

## Outcomes and the unmatched queue (PC-2, hazard H-044)

An outcome, decline or interim note names the referral it answers by the referral id. On arrival there are four cases, each audited:

| Case | What happens |
|---|---|
| The id matches a live referral and the event is allowed in its state | The referral moves (`next` in `src/referral.rs`) and the referrer's worklist shows it. |
| The id matches a referral but the event is not allowed (an outcome after `cancelled`, a decline after `completed`) | Refused by the state table; the message goes to the queue with the reason, and the referrer is told. It is never dropped. |
| The id matches no referral | The message goes to the queue. It is **not** matched by patient and date. |
| The message carries no id | The same queue, marked "no referral id", because this is the source's most common real gap. |

The **queue** is a list a person must clear, in the shape of the feed-refusal queue: each entry keeps the message as received, why it did not match, who cleared it, how (matched by hand to a named referral, or dismissed with a reason) and when. A clearance is a new row, never an edit. A count shows on the referrer's worklist and to the owning team, so an unmatched outcome has a place where someone looks. A hand match is a separate, audited act by a person with a role, and it records both ids; it does not change the sender's message.

Not designed here: who owns the queue when the sender is another organisation (OQ-PC2's neighbour), how long an uncleared entry may wait (PC-3), and the table for the referral itself (CA-3). A referral id mistyped as another real one cannot be caught by software.

## Patient flags: an idea, not a design

The source proposes a `Flag` that **points to** a plan someone else owns (child protection, looked-after status, a risk, a reasonable adjustment) instead of copying it, says who to contact, is not a `Condition`, and notifies the owner when it is read. R4's `Flag` cannot reference a `CarePlan` without an extension, and which extension is "To confirm". Task PC-7 holds the questions; PA-6 already covers reasonable adjustments.

## Not covered by the source, and so by nobody

Authorisation, consent, break-the-glass and tamper-evident audit of a referral; who may cancel or redirect; what happens when the recipient is unreachable. This repository's policy engine, consent and audit trail answer the first three; the rest are open questions OQ-PC2 and the timeouts above.
