# Translation review register

Translations are machine-written until a speaker reviews them. **Never claim a translation is reviewed** unless its row below names the reviewer and the date. `tests/unit/translation-register.test.ts` checks that every locale in `src/lib/locales.ts` has exactly one row, that a status is `source`, `machine-written` or `reviewed`, and that `reviewed` names a reviewer and a date. `tests/unit/translations.test.ts` checks that every translated key is present in every locale, differs from English and keeps its placeholders.

Welsh (`cy-001`) has the whole UI in a machine-written table (`src/lib/strings.cy.ts`, since 2026-11-09, WL-8), and every page in it says, in Welsh and in English, that the words were translated by machine and no speaker has checked them, with a link to the same page in English (H-051). `REVIEW` in `src/lib/i18n.ts` must agree with this register. The other locales translate one surface, the access-log filter form on `/my-record` (`src/lib/translations.ts`), and fall back to English for everything else. The clinical wording in English (notices, grounds, allergy and vital-sign text) is also unreviewed by a clinician.

| Locale | Translated surfaces | Status | Reviewer | Date |
| --- | --- | --- | --- | --- |
| ar-001 | access-log filter form on `/my-record` | machine-written | none | none |
| cy-001 | the whole UI: every key of `STRINGS` (`src/lib/strings.cy.ts`, WL-8), and the access-log filter form | machine-written | none | none |
| de-001 | access-log filter form on `/my-record` | machine-written | none | none |
| de-de | access-log filter form on `/my-record` | machine-written | none | none |
| en-001 | source language | source | n/a | n/a |
| es-001 | access-log filter form on `/my-record` | machine-written | none | none |
| fr-001 | access-log filter form on `/my-record` | machine-written | none | none |
| hi-001 | access-log filter form on `/my-record` | machine-written | none | none |
| zh-001 | access-log filter form on `/my-record` | machine-written | none | none |
| zh-cn | access-log filter form on `/my-record` | machine-written | none | none |
