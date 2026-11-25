# What registration can find: candidate retrieval, measured (AG-8)

Measured 2026-11-25 by [`tests/candidate_recall.rs`](../../electronic-patient-record-service-with-rust/tests/candidate_recall.rs) on a scratch database, with the matcher at 0.8.1. **Synthetic people, invented by the author.** It shows which *kinds* of difference are found. It does **not** say how often each kind happens, it is not a recall figure for any site, and no registration lead adjudicated it. The test pins the table, misses included.

## How a duplicate is looked for

The database retrieves at most 200 current patients that share an identifier, a birth date or a family name with the new registration, exactly (a national number without its spaces and hyphens). Only those reach the matcher. **A true duplicate outside that set is missed whatever the matcher would have said.**

## Two registrations of one person

| Difference | Retrieved | The matcher says |
|---|---|---|
| none | yes | AutoLink |
| family name misspelt | yes | AutoLink |
| family name changed | yes | Review |
| day and month swapped | yes | Review |
| misspelt, and day and month swapped | **no** | Review |
| family name changed, and year one out | **no** | NonMatch |
| misspelt, and only a birth year known | **no** | Review |
| misspelt, and no birth date known | **no** | Review |
| accent lost | yes | AutoLink |
| accent lost, and only a birth year known | **no** | Review |
| another script | yes | NonMatch |
| another script, and day and month swapped | **no** | NonMatch |
| family and given name swapped | yes | Review |
| family and given name swapped, and day and month swapped | **no** | Review |
| half of a double family name | yes | AutoLink |
| half of a double family name, and year one out | **no** | Review |
| hyphen against space, and only a birth year known | **no** | Review |
| same number, all else different | yes | Review |
| same number written with spaces, and all else different | yes (since 2026-11-25) | Review |

**10 of 19 kinds are retrieved. Of the 9 that are not, the matcher would have sent 7 to a person.** One difference is usually survived; two at once usually are not. The last row was a defect and is fixed (AG-12): registration, search and the HL7 feed compared identifiers as text, so one number typed two ways was two. Another script is missed even when retrieved, since nothing transliterates it well enough to score.

## A common family name

With 300 people of one family name, a duplicate that shares **only the name** has a place in the list of 200 two times in three, by the chance of its random id. One that shares the name and the birth date always has one. The answer says `candidates_cut`, so a person is told that the list was cut.

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
2. Retrieve by a folded family name (accents, hyphens, spaces) and by the given name in the family's place (AG-12).
3. Retrieve by a birth date with its day and month swapped, and by the year when only a year is known (AG-12), and measure what it costs.
4. Require an identifier or an address before any automatic decision (AG-13, OQ-AG8): the matcher's, and a clinical safety decision.
5. A site's own evaluation, with adjudicated pairs and denominators, before any of this is believed (AG-8, ID-1).
