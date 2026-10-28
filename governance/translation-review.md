# Translation review register

Translations are machine-written until a speaker reviews them. **Never claim a translation is reviewed** unless its row below names the reviewer and the date. `tests/unit/translation-register.test.ts` checks that every locale in `src/lib/locales.ts` has exactly one row, that a status is `source`, `machine-written` or `reviewed`, and that `reviewed` names a reviewer and a date. `tests/unit/translations.test.ts` checks that every translated key is present in every locale, differs from English and keeps its placeholders.

Only one surface is translated: the access-log filter form on `/my-record` (`src/lib/translations.ts`). Everything else is English, and a locale with no entry falls back to English. The clinical wording in English (notices, grounds, allergy and vital-sign text) is also unreviewed by a clinician.

| Locale | Translated surfaces | Status | Reviewer | Date |
| --- | --- | --- | --- | --- |
| ar-001 | access-log filter form on `/my-record` | machine-written | none | none |
| cy-001 | access-log filter form on `/my-record` | machine-written | none | none |
| de-001 | access-log filter form on `/my-record` | machine-written | none | none |
| de-de | access-log filter form on `/my-record` | machine-written | none | none |
| en-001 | source language | source | n/a | n/a |
| es-001 | access-log filter form on `/my-record` | machine-written | none | none |
| fr-001 | access-log filter form on `/my-record` | machine-written | none | none |
| hi-001 | access-log filter form on `/my-record` | machine-written | none | none |
| zh-001 | access-log filter form on `/my-record` | machine-written | none | none |
| zh-cn | access-log filter form on `/my-record` | machine-written | none | none |
