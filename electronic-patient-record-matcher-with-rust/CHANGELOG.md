# Changelog

## [Unreleased]

- **Changed:** a family name of several parts is compared part by part; `Smith-Jones` no longer scores as a close
  match to `Smithson` (CQ-19).
- **Changed:** a given name on the longer list that nothing on the shorter list matches blocks `AutoLink` (conflict
  "one record has a given name the other lacks"); the score is unchanged, so such a pair goes to Review (CQ-19).

- **Changed:** names are also compared transliterated (`Müller`/`Mueller`, `José`/`Jose`); the better score is kept, so a
  similarity can rise but never fall. `normalize::transliterate` is public (CQ-19).
- **Changed:** given and family names that match each other's boxes (not as written) score at 0.9 of that similarity and
  block `AutoLink` with the conflict "given and family names appear swapped" (CQ-19).
- **Fixed:** `fold_name` keeps combining marks that follow a letter; a virama or similar mark used to turn into a space and
  split an Indic name into fragments (CQ-19).
- **Changed:** the postcode field, and the telecom field unless an email address is shared, count at half weight as household
  evidence; `FieldScore.weight` shows the weight used. Identical records still score 1.0 (CQ-19).
- **Changed:** phone numbers are compared in E.164 form, and a number under 7 digits is absent (CQ-19). `MatchConfig` gains
  `default_calling_code` (44; `serde` default, so older JSON still loads) and `ConfigError::InvalidCallingCode`;
  `normalize::normalize_phone` is public.
- **Added:** `MatchConfig::validate()`, `ConfigError` and `MatchingEngine::try_new` (CQ-19). `new` is unchanged.

## [0.2.0] - 2026-10-21

Patient-safety fixes to matching. Behaviour changes, so pairs scored by 0.1.0 may score differently. See the
[repository changelog](../CHANGELOG.md) entries CQ-1 to CQ-5 for tests and what is not done.

- **Changed:** `compare` is symmetric when both given-name lists have the same length; it averages both ways and keeps
  the lower (CQ-1).
- **Changed:** `is_valid_nhs_number` rejects repeated-digit placeholders (`0000000000` … `9999999999`) (CQ-2).
- **Added:** `is_placeholder_nhs_number` (CQ-2).
- **Changed:** a shared NHS number goes to Review when both names disagree (CQ-2), or when the full birth dates differ
  at all (CQ-3).
- **Changed:** birth-date typo credit needs a one-digit slip or an adjacent swap (CQ-4).
- **Changed:** impossible calendar dates are absent (CQ-4).
- **Changed:** given-name tokens that fold to nothing are absent (CQ-5).

## [0.1.0] - 2026-10-20

Scaffold. See the [repository changelog](../CHANGELOG.md) for what was and was not done.
