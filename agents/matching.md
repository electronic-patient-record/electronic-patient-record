# Matching

Read the matcher [spec](../electronic-patient-record-matcher-with-rust/spec/index.md). The rules that must not be loosened
to make a test pass, because a wrong automatic link (an overlay) is a patient safety incident:

1. A missing given name, family name or birth date blocks `AutoLink`.
2. Twins (same family name, birth date, address; different given names) never `AutoLink`.
3. Two different valid NHS numbers are `NonMatch`.
4. A shared NHS number with different full birth dates is `Review`, not `AutoLink`.
5. Merge must be reversible (unmerge), with provenance (task ID-5).

Thresholds are unvalidated until the labelled corpus exists (ID-1).
