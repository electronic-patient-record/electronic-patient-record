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
   average; the note says why.
5. **Missing evidence blocks automatic linking.** `AutoLink` additionally requires given name, family name and
   birth date to be present and above their minimums (`min_given_for_auto` 0.85, `min_family_for_auto` 0.85,
   `min_birth_date_for_auto` 0.99), and at least `min_evidence_weight` (0.5) of the total weight present. Otherwise the
   decision falls to `Review` and `conflicts` lists the reasons.
6. **Diacritics are preserved.** Folding is NFKC plus lowercase; accents are never stripped.

## 3. Scoring

Weighted mean of per-field similarities over the fields present on both records.

| Field | Weight | Similarity |
|---|---|---|
| family name | 0.25 | best Jaro-Winkler over name pairs |
| given name | 0.20 | per token best match, averaged over the shorter list (with lists of equal length, averaged both ways and the lower kept, so order never raises the score); an initial matching a token's first letter scores 0.8 |
| birth date | 0.30 | exact 1.0; day and month swapped 0.8; one component typo 0.6; partial dates by shared precision; else 0.0 |
| sex | 0.05 | equal 1.0, different 0.0; `unknown` counts as absent |
| postcode | 0.10 | any folded postcode in common |
| telecom | 0.10 | any phone (digits) or email (lowercase) in common |

Decision: `auto_link_threshold` 0.92, `review_threshold` 0.75, else `NonMatch`. **These are unvalidated starting
values** (ID-1 asks for a 200-pair labelled corpus before they are trusted).

## 4. Deterministic identifiers

- A **shared valid NHS number** (system `https://fhir.nhs.uk/Id/nhs-number`, modulus-11 check digit) gives score 1.0
  and `AutoLink`, unless both records have full birth dates that differ, which gives `Review` with a conflict.
- **Two different valid NHS numbers** give `NonMatch` with a conflict, however alike the demographics.
- Invalid NHS numbers are ignored for matching.
- Other identifiers (MRN) do not decide a match yet.

## 5. Known gaps

Labelled corpus, fuzz, benches; phonetic comparison; address lines; name-change history (maiden names) beyond
multiple `names`; transliteration; configurable per-site weights from a file.
