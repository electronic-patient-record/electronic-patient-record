# Mobile viewability

**A direction from the author's instruction of 2026-11-04 (plan.md §27). Nothing here is built.** The record must be readable and usable on a phone, not only on a ward workstation:
community, home-visit and mental-health staff, and clinicians on call, work from a phone or small tablet.

## What the UI has today (checked 2026-11-04, by reading the code)

- `src/app.html` sets `<meta name="viewport" content="width=device-width, initial-scale=1">`; `src/app.css` has one media query (dark mode). A search of `src/` for breakpoints, `@media` widths and the word "mobile" found nothing else.
- Playwright runs one project, `chromium` with `Desktop Chrome`; no phone-sized viewport is tested. `axe` runs on that desktop size only.
- Tables (patients list, compare, new, vitals) are `<table>`s. Whether they scroll sideways at 320 px was **not tested**.
- The idempotency key on every create (OF-1) already makes a retried write safe, which a phone on a poor connection needs.

## Requirements (proposed)

1. **Reflow at 320 CSS pixels** with no two-dimensional scrolling (WCAG 1.4.10 Reflow; 320 CSS px is 1280 px at 400% zoom), so the same pages serve a phone and a zoomed desktop.
2. **Touch targets** at least 24 by 24 CSS pixels, or enough spacing (WCAG 2.2 criterion 2.5.8, Level AA). Primary actions (save, acknowledge, sign) larger; the common 44 px guidance is a design choice to confirm, not a WCAG rule.
3. **The patient banner never scrolls away or truncates identity.** Name, date of birth and the identifier stay visible on a phone, compact but whole; a long name wraps rather than cuts. Wrong-patient error is the project's first hazard (H-001) and a small screen raises it.
4. **Safety information is not the first thing dropped.** Allergies, alerts and unreviewed results are never hidden behind a "more" control on a small screen without a visible count.
5. **Tables become stacked rows** at narrow widths (a row is a card with labelled values) rather than scrolling sideways; the same data, the same order.
6. **A shared or lost device is a threat.** Idle lock (the UI has `idle.ts`), no record data in push notifications, nothing from the record kept in local storage beyond the session, remote sign-out. Details belong to the [MFA plan](../governance/mfa-plan.md) and the [data flow](../governance/data-flow.md).
7. **Poor or no signal** (a home visit, a rural clinic): read the downtime snapshot and queue writes with their idempotency keys; whether offline entry is needed is OQ-5 and stays open.
8. **Camera and location are optional** and asked for only when used (a wound photo reuses the patient photo path; location is not recorded without a stated purpose).

## Tests (proposed)

Add Playwright projects for a phone (for example `Pixel 7`) and a small iPhone; run `axe` and a "no horizontal scroll at 320 px" check on the banner, patient list, record page and referral screens; fail on a truncated banner. Real-device checks by hand before any release, since an emulator does not show a real touch keyboard or a notch.

## What breaks at 320 px (XP-1, measured 2026-11-25)

The interface's own browser tests run on two phones (`playwright.mobile.config.ts`: a Pixel 7 at 320 by 640 in Chromium, an iPhone SE at 320 by 568 in WebKit), and each place where a test runs axe is also measured (`tests/e2e/mobile.ts`). 83 places on each phone, in populated screens with invented records; 246 of 250 tests pass. **Not a gate and not in CI** until XP-2 passes.

**Since then (XP-3, the same day):** the first row is answered by the strip below. **And XP-2, the same day:** nothing scrolls sideways at 320 px any more in the 83 places, the message as received wraps, and links have 24 px or more; selects in WebKit are still 23 px, as that browser sets. The two tests that still fail on the iPhone SE are the last row.

| What | Where | Measured | Reason |
|---|---|---|---|
| **The banner covers the page** (H-070) | Every patient page, 53 places on each phone | It stays at the top and takes **41 to 86 % of the phone's height (median 65) and 50 to 97 % of the iPhone SE's (median 73)**. On the iPhone SE "Save changes" is under it and cannot be pressed | Sticky, and as tall as its content: identity, allergies, needs, notices. A desktop at 400 % zoom is about 320 by 256, so it must be worse there; that was not measured. It meets rule 3 below and breaks the page |
| Sideways scrolling | Vital signs trend on the patient page | 43 px | A table of four columns with no reflow |
| Sideways scrolling | Feed queue, with a message open | 131 px; on the Pixel "Close this entry" cannot be pressed | The message as received does not wrap |
| Sideways scrolling | Patient search results | 6 px | A table of results |
| Small targets | "Back to search", "Open the summary", "Open the patient", "Back to the patient", "Open" in results, "Privacy" | 18 or 19 px high | Links on a line of their own with no padding. WCAG 2.5.8 allows a smaller target with space around it, which was not worked out |
| Small targets | Every select, in WebKit only | 23 px high | The browser's own size |
| Not a fault of the page, or not known | Forced colours; the photo upload's size, in WebKit | 2 tests fail | WebKit has no `forced-color-adjust`; the photo's bytes are not given to the test by that browser (believed, not checked on a phone) |

**Nothing was cut off in the banner** at 320 px in any place, and it was in sight at the end of every page but a patient's own record. Axe found nothing at 320 px in any place. Not seen: the pages no test opens, landscape, a tablet, a real device, a screen reader on a phone, and a long name or a long list of allergies, which make the banner taller.

## The strip (XP-3, 2026-11-25, unreviewed)

On a window under 640 px wide or 480 px high the banner scrolls with the page, and a strip stays at the top: the name, birth date and an identifier (the national number, or the other one) whole and never cut; in words, a drill, a reported or confirmed death, a failed look for one, a temporary identity or a merge; and the allergies. **Every allergy is named that is marked high risk, or whose risk was not stated or not assessed, however many there are** (the maintainer's answers to OQ-XP2: what is life-critical, and the identifier; an unknown risk is not a low one); of those recorded as low risk a long list is cut to what fits 60 characters, with the number left out (`Penicillin (high risk), Latex (high risk), Trimethoprim, +5`). The text of a reaction is not read; "not recorded", "not shown to you" and "could not be loaded" are never shortened. What gets the focus or is jumped to stops below it. A screen reader is given the banner and not the strip. On a larger window nothing changed. Rule 3 above is met for the strip and not for the whole banner, and what else is life-critical is not agreed (OQ-XP2). On an iPhone SE "Save changes" can be pressed again.

Measured at 320 by 640, 320 by 568 and 640 by 320 on one machine: 13 to 15 % of the height for an ordinary record, and 30 to 34 % for a crowded one (a long name, 8 allergies, a drill, a reported death and a temporary identity). **Fonts differ between machines**: on CI's the crowded strip was 47 % before the notices were put on one line, so the test allows 25 and 50 %, and a real phone may differ again. With no risk stated for seven of its eight allergies, so that all are named, the crowded strip was 41 % of the iPhone SE's height, and the test allows 60.

## Tables and targets (XP-2, 2026-11-25)

On a window under 640 px wide the search results, the comparison, the possible duplicate and the vital signs trend are cards: a row is a card and each cell says what it is, in the same order. It is still a table to a screen reader (the roles are written out, as some browsers drop them with the layout), and a cell's name is its value alone. The privacy table scrolls in its own region, as before. Wherever a browser test runs axe, the page is now also looked at 320 px wide: it must not scroll sideways, and no link on its own, button or summary may be under 24 px high.

## Not done, not verified

One screen was changed (the strip) and no real phone was used (emulated), and the 24 px and 320 px values come from secondary summaries of WCAG 2.2, to be checked against the W3C pages. Tasks: `XP-1` to `XP-4`.
