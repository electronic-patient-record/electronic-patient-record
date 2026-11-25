# One state table for every order (EH-4)

**Built 2026-11-25 as a pure library (`order.rs`, 11 tests); nothing calls it and nothing is stored.** No clinician, pharmacist or laboratory has reviewed it. NOT FOR CLINICAL USE. For [order entry](index.md) (OE), [medication administration](../medication-administration/index.md) (MA) and, if PC-5 agrees, referrals, which keep their own table today. The idea of one machine for every order, and of keeping the orderer's words, is from [openEHR and `openehr-rust`](../lessons/openehr-rust/index.md); **the two sides, the flags and the table are the author's** and are not openEHR's states.

## Two sides

One word for an order hides the cases that hurt: an order can be cancelled **and** done.

| Side | Words | Moved by |
|---|---|---|
| What the orderer **wants** (`state`) | `draft`, `active`, `on-hold`, `cancel-requested`, `revoked`, `completed`, `entered-in-error` | The orderer, and the filler's answer to a cancel |
| What the filler has **got to** (`filling`) | `not-sent`, `sent`, `accepted`, `refused`, `started`, `abandoned`, `done` | The service (sending) and the filler |

## Rules

1. **Asking is checked; a fact is not.** A step that asks for something is refused unless the table allows it. That something *was done* is never refused in any state: refusing to record a dose given after a cancel would lose the dose. It never changes what the orderer wants; it is kept and **flagged** for a person.
2. **A cancel is a request** once the order was sent. It ends with the filler's answer. Nobody is asked when nobody is going to do it (not sent, refused, abandoned). One that was all done meanwhile cannot be confirmed as cancelled.
3. **A refusal comes back.** A refused order stays active and the orderer's, until they end it.
4. **Done is not finished.** An order completes only when the orderer closes one that was done, or that stopped part of the way.
5. **A draft is thrown away; a signed order is called a mistake**, from any state, and the filler must be told if it was sent. A draft on which something was done, as after a spoken order, is signed or called a mistake, and cannot be thrown away; nobody has to sign what they never ordered.
6. **The order's own words are kept**, cannot be changed and are never generated. A change is another order. This alters rule 3 of [order entry](index.md): the order it replaces is not revoked in the same transaction when it has been sent; the cancel is **asked for**, since the filler may have done it.
7. **No order is stuck**: from every place an order can get to, some steps lead to an end without calling it a mistake and without pretending something was done (a test walks them all).

## Steps

| Step | From | Allowed when | To |
|---|---|---|---|
| Sign | orderer | draft | active |
| Discard | orderer | draft, not sent, nothing done | revoked |
| Send | service | active, not sent | sent |
| Hold / Release | orderer | active and not done / on hold | on hold / active |
| Request cancel | orderer | active or on hold: not sent, refused, abandoned | revoked |
| Request cancel | orderer | active or on hold: sent, accepted, started | cancel-requested |
| Confirm cancel | filler | cancel-requested, not all done | revoked |
| Decline cancel | filler | cancel-requested | active |
| Accept / Refuse | filler | live and sent / sent or accepted | accepted / refused |
| Abandon | filler | live and started | abandoned |
| Close | orderer | active, and done or abandoned | completed |
| Retract | orderer | anything but a mistake, or a draft with nothing done | entered-in-error |
| Part done, Done | filler | **always** | started, done; flagged unless wanted and not refused |

Flags: `done-before-signed`, `done-while-on-hold`, `done-after-cancel-asked`, `done-after-revoked`, `done-after-refused`, `done-again`, `done-on-mistake`, `tell-filler`.

## FHIR R4 words

`cancel-requested` is sent as **`active`**: it may still be done. A medicine that was revoked is `stopped` if some was given and `cancelled` if none was. `filling` has no place in either status; a `Task` would carry it (not designed).

## Not decided

| Id | Question | Who |
|---|---|---|
| OQ-OS1 | Who sees each flag, how soon, and who answers when the orderer has left? | clinical lead, Clinical Safety Officer |
| OQ-OS2 | Is a spoken order done before it was signed ever "wanted", and how long may it wait for a signature? | clinical lead, pharmacist |
| OQ-OS3 | May an order end by time alone (a course that is over, a test nobody took)? There is no `expired` here: an order whose time passed is **overdue**, which is worked out and not a state. | clinical lead, pharmacist |
| OQ-OS4 | Does a referral move to this table (PC-5)? Its no-show, information and redirect steps have no place here. | maintainer |

## Not done, not verified

No store, route, screen, time, dose or timetable; no link to a result. Who may send a step is the policy engine's. A fact recorded in error (the wrong patient's dose) has no step here. One order for several doses is `started` until someone says it is done; what each dose is belongs to MA-2. Hazards OE-H2 and OE-H7 of [order entry](index.md) go in the log with the first code that stores an order.
