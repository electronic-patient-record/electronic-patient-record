# Matcher — Specification

Source of truth for the matcher crate's internals. The entity spec ([`../../spec/index.md`](../../spec/index.md))
wins on the cross-subproject contract.

## 1. Purpose

Given two `Patient` records, return a score, a decision and a per-field explanation, so a registration clerk or a
reviewer can see why two records were or were not linked.

## 2. Invariants

1. **Total and pure.** No IO, clocks, randomness or panics. Any pair of values gives a result.
2. **Symmetric.** `compare(a, b) == compare(b, a)` (tested over a grid of 122,500 ordered pairs of name, date and sex
   variants).
3. **Deterministic.** Same input, same output (tested).
4. **Absent is not zero.** A field missing on either side has `similarity: None` and is excluded from the weighted
   average; the note says why. A name token that folds to nothing (`-`, spaces) is missing, not a token that disagrees.
5. **Missing evidence blocks automatic linking.** `AutoLink` additionally requires given name, family name and
   birth date to be present and above their minimums (`min_given_for_auto` 0.85, `min_family_for_auto` 0.85,
   `min_birth_date_for_auto` 0.99), and at least `min_evidence_weight` (0.5) of the total weight present. Otherwise the
   decision falls to `Review` and `conflicts` lists the reasons.
6. **Diacritics are preserved.** Folding is NFKC plus lowercase; accents are never stripped.

## 3. Scoring

Weighted mean of per-field similarities over the fields present on both records.

| Field | Weight | Similarity |
|---|---|---|
| family name | 0.25 | best Jaro-Winkler over name pairs; a name of several parts (`Smith-Jones`, `Jones Smith`) is compared part by part (each part of the shorter takes its best match, scaled by the ratio of the two parts' lengths, mean kept; with equal numbers of parts both ways and the lower kept), so `Smith-Jones` does not match `Smithson` and `Smith` still matches `Smith-Jones` |
| transliteration | | applied to both name fields: the score is the better of the name as written and both names transliterated (`ä ö ü` to `ae oe ue`, `ß` to `ss`, a Latin letter with a mark to its base letter), so `Müller` and `Mueller` match fully; other scripts are untouched |
| placeholder names | | A name part that folds to `unknown`, `unidentified`, `anonymous`, `unk` or `unkn` (digits fold away, so `UNK-001` is `unk`) is a placeholder: it says nothing about who the person is, so it is treated as an absent name everywhere (family, given, swap, phonetic). Absent evidence blocks AutoLink, so two unidentified patients never match each other and an unidentified patient never auto-links to a real record. Whole-part match only (`Unkind` and `Doe` count as names); English and short by design (EC-2) |
| phonetic signal | none | **Not a similarity and never in the score.** `MatchResult.phonetic_match` is true when one name on each record has the same American Soundex for the family name and for the first given name (after transliteration; a script with no Latin letters makes no claim) and the two are not spelled identically. When the decision would be `NonMatch` and the birth-date similarity reaches `min_birth_date_for_auto`, the pair goes to `Review` with the conflict "names sound alike but are spelled differently". It never reaches `AutoLink` |
| nicknames | | two given-name tokens in one group of a built-in list of English nicknames (`Bill`, `Will`, `William`; 38 groups in `normalize::NICKNAME_GROUPS`) score at least 0.8, under `min_given_for_auto`, so the pair can reach Review and never auto-links on the name alone. A short form in two groups (`Alex`) joins each, but the two long forms do not match each other |
| name folding | | NFKC, lowercase; letters and the combining marks (`\p{M}`) that follow a letter are kept, anything else (hyphen, apostrophe, digits) separates words; a mark with no letter before it is dropped. A virama or vowel sign therefore does not split `कृष्ण` |
| swapped names | | when neither the family nor the given similarity reaches 0.85 as written, but the family name matches the other record's first given name and the first given name matches the other's family name (the lower of the two at least 0.85), both fields take that similarity times 0.9 and `AutoLink` is blocked with the conflict "given and family names appear swapped"; symmetric |
| given name | 0.20 | per token best match, averaged over the shorter list (with lists of equal length, averaged both ways and the lower kept, so order never raises the score; when the longer list has a name nothing in the shorter matches at 0.85 or more, `AutoLink` is blocked with the conflict "one record has a given name the other lacks", so `John` and `John Paul` go to Review); an initial matching a token's first letter scores 0.8 |
| birth date | 0.30 | exact 1.0; day and month swapped 0.8; one component differing by a keying slip (one digit, or two adjacent digits swapped, written as `DD`, `MM` or `YYYY`) 0.6; partial dates by shared precision; else 0.0. A date the calendar does not have (`2020-02-31`, `1900-02-29`) is absent |
| sex | 0.05 | equal 1.0, different 0.0; `unknown` counts as absent |
| postcode | 0.10, **half (0.05) as household evidence** | any folded postcode in common |
| telecom | 0.10 | any phone or email in common. A phone is normalised towards E.164: `+` or `00` marks an international number, a leading `0` takes `default_calling_code` (44), a `0` written after the country code (`+44 (0)7700`) is dropped, a number with no prefix is kept as written, and fewer than 7 digits is absent. Email is lowercase. A shared postcode or phone is household evidence (relatives, flatmates and a care home share them): the field's weight is halved, whatever the similarity, unless an email address is shared, which keeps the full 0.10. `FieldScore.weight` reports the weight used |

Configuration: `MatchConfig::validate()` refuses a negative or non-finite weight, all weights zero, a threshold outside 0 to 1
(or NaN), and a `review_threshold` above `auto_link_threshold`, each with a `ConfigError` variant; `MatchingEngine::try_new`
calls it, while `MatchingEngine::new` does not. `Default` is valid.

Conflicts: `MatchResult.conflicts` keeps its English text, and `MatchResult.conflict_codes` carries the same reasons as
`ConflictCode` values in the same order, serialised with a snake_case `code` tag and their numbers (`field_unavailable`
and `field_below_minimum` name a `GuardField`; `field_below_minimum` gives the similarity and the minimum;
`evidence_thin` gives the percentage present). A client translates the code, not the text. The field names in
`FieldScore.field` are already stable snake_case identifiers and stay strings.

Decision: `auto_link_threshold` 0.92, `review_threshold` 0.75, else `NonMatch`. **These are unvalidated starting
values** (ID-1 asks for a 200-pair labelled corpus before they are trusted).

## 4. Deterministic identifiers

- A **valid NHS number** has a correct modulus-11 check digit and is not ten copies of one digit. Every
  repeated-digit number (`0000000000` … `9999999999`) passes modulus 11, none is issued, and systems use them as
  placeholders, so they are invalid (`is_placeholder_nhs_number` says which).
- A **shared valid NHS number** (system `https://fhir.nhs.uk/Id/nhs-number`) gives score 1.0 and `AutoLink`, unless
  either of these holds, which gives `Review` with a conflict for each:
  - both records have full birth dates that differ;
  - both the given and the family name are below their auto-link minimums (a family name alone may change, as on
    marriage; both disagreeing suggests a mistyped or reused number);
  - the given name, the family name or the birth date is absent on either record (`FieldUnavailable`, one per
    missing field): missing evidence is not agreement, as for every other pair (CQ-37, decided 2026-10-25). A
    number alone, or a number with a date dropped, is for a person to confirm. The score is still 1.0 and the
    stated basis still the shared number.
- **Two different valid NHS numbers** give `NonMatch` with a conflict, however alike the demographics.
- Invalid NHS numbers are ignored for matching.
- Other identifiers (MRN) do not decide a match yet.

## 5. Known gaps

Labelled corpus, fuzz, benches; phonetic comparison; address lines; name-change history (maiden names) beyond
multiple `names`; transliteration; configurable per-site weights from a file.
