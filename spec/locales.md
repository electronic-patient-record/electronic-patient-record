# Locales

A locale is named `<language>-<region>`, lowercase and hyphenated (`en-001`, `zh-cn`); `-001` is the UN M.49 code for
"world" and stands for a language with no regional variant. A bare two-letter code is not a locale: `/en/patients` is an
ordinary unprefixed path. The supported set is `LOCALES` in `electronic-patient-record-ui-with-svelte/src/lib/locales.ts`:
`ar-001 cy-001 de-001 de-de en-001 es-001 fr-001 hi-001 zh-001 zh-cn`. URLs carry the prefix (`/en-001/patients`); a bare
URL gains the remembered, browser-preferred or default (`en-001`) locale. **Only English strings exist today**; the other
locales fall back to it, and no translation is claimed to be reviewed.
