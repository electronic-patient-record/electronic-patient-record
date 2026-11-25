# What registration can find: candidate retrieval, measured (AG-8)

Measured 2026-11-25 by [`tests/candidate_recall.rs`](../../electronic-patient-record-service-with-rust/tests/candidate_recall.rs) on a scratch database, with the matcher at 0.8.1. **Synthetic people, invented by the author.** It shows which *kinds* of difference are found. It does **not** say how often each kind happens, it is not a recall figure for any site, and no registration lead adjudicated it. The test pins the table, misses included.

## How a duplicate is looked for

The database retrieves at most 200 current patients that share an identifier (a national number without its spaces and hyphens), a birth date, or a family name **without case, accents, spaces, hyphens and apostrophes** (or one part, of four letters or more, of a double one); or whose names are the other way round; or whose birth date has its day and month the other way round. Only those reach the matcher. **A true duplicate outside that set is missed whatever the matcher would have said.**

## Two registrations of one person

| Difference | Retrieved | The matcher says |
|---|---|---|
| none | yes | AutoLink |
| family name misspelt | yes | AutoLink |
| family name changed | yes | Review |
| day and month swapped | yes | Review |
| misspelt, and day and month swapped | yes (since 2026-11-25) | Review |
| family name changed, and year one out | **no** | NonMatch |
| misspelt, and only a birth year known | **no** | Review |
| misspelt, and no birth date known | **no** | Review |
| accent lost | yes | AutoLink |
| accent lost, and only a birth year known | yes (since 2026-11-25) | Review |
| another script | yes | NonMatch |
| another script, and day and month swapped | yes (since 2026-11-25) | NonMatch |
| family and given name swapped | yes | Review |
| family and given name swapped, and day and month swapped | yes (since 2026-11-25) | Review |
| half of a double family name | yes | AutoLink |
| half of a double family name, and year one out | yes (since 2026-11-25) | Review |
| hyphen against space, and only a birth year known | yes (since 2026-11-25) | Review |
| same number, all else different | yes | Review |
| same number written with spaces, and all else different | yes (since 2026-11-25) | Review |

**16 of 19 kinds are retrieved; 9 were when this was first measured, earlier the same day.** Of the 3 that are not, the matcher would have sent 2 to a person: a misspelt family name with only a birth year, and with no birth date. Another script is missed even when retrieved, since nothing transliterates it well enough to score.

## A common family name

With 300 people of one family name, a duplicate that shares **only the name** has a place in the list of 200 two times in three, by the chance of its random id. One that shares the name and the birth date always has one. The answer says `candidates_cut`, so a person is told that the list was cut.

## What it costs

On 50,000 synthetic patients, one machine, one run each, warm: a family name of 6 people took 26 ms and now takes 1.4; one of 1,000 people took 25 and now takes 6. The first way tried, with the keys worked out for each row, took 54 ms for the second, so the keys are stored columns that the database makes. Adding them rewrites the patient table once (about 3 seconds for 50,000 rows, holding it meanwhile); a checksum of every recorded value was the same before and after.

**It also costs work.** More candidates reach the matcher, so more registrations and feed messages may be held for a person. How many more was not measured: it depends on a site's names.

## Two people who are not one

| Pair | The matcher says | Score |
|---|---|---|
| twins with unlike names (Hana, Dawit) | Review | 0.862 |
| twins with like names (Jon, Jan) | Review | 0.947 |
| **twins, one letter apart (Kofi, Kofe)** | **AutoLink** | 0.969 |
| parent and child of one name | Review | 0.840 |
| a common name, another birth date | NonMatch | 0.600 |

**The matcher would let twins whose given names differ by one letter be linked automatically**, on a family name, a birth date and one similar given name with no identifier and no address. The matcher's own test of twins uses names that are nothing alike. Nothing in the service links by itself: registration and the HL7 feed refuse a likely duplicate, a merge needs a person and a reason, and a test holds that no other code reads the decision. But a screen that says "Automatic link" invites the merge, and the matcher is a published library. Whether a name and a birth date alone may ever be enough is OQ-AG8.

## What would help, in order

1. ~~Compare identifiers by what they mean and not by how they were typed~~: done (AG-12, migration 45), for national numbers only.
2. ~~Retrieve by a folded family name and by the names swapped, and by a birth date with its day and month swapped~~: done (AG-12, migration 46).
3. Retrieve by a name that sounds alike, and by the year when only a year is known (AG-12): both widen the list much more, so they need a measured cost and a site's own frequencies.
4. Require an identifier or an address before any automatic decision (AG-13, OQ-AG8): the matcher's, and a clinical safety decision.
5. A site's own evaluation, with adjudicated pairs and denominators, before any of this is believed (AG-8, ID-1).
