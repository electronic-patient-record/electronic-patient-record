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

## Not done, not verified

No screen was changed, no phone was used, and the 24 px and 320 px values come from secondary summaries of WCAG 2.2, to be checked against the W3C pages. Tasks: `XP-1` to `XP-4`.
