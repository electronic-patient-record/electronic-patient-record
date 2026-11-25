# What registration can find: candidate retrieval, measured (AG-8)

Measured 2026-11-25 by [`tests/candidate_recall.rs`](../../electronic-patient-record-service-with-rust/tests/candidate_recall.rs) on a scratch database, with the matcher at 0.8.1 and again at 0.9.0. **Synthetic people, invented by the author.** It shows which *kinds* of difference are found. It does **not** say how often each kind happens, it is not a recall figure for any site, and no registration lead adjudicated it. The test pins the table, misses included.

## How a duplicate is looked for

The database retrieves at most 200 current patients that share an identifier (a national number without its spaces and hyphens), a birth date, or a family name **without case, accents, spaces, hyphens and apostrophes** (or one part, of four letters or more, of a double one); or whose names are the other way round; or whose birth date has its day and month the other way round. Only those reach the matcher. **A true duplicate outside that set is missed whatever the matcher would have said.**

## Two registrations of one person

| Difference | Retrieved | The matcher says |
|---|---|---|
| none | yes | Review |
| family name misspelt | yes | Review |
| family name changed | yes | Review |
| day and month swapped | yes | Review |
| misspelt, and day and month swapped | yes (since 2026-11-25) | Review |
| family name changed, and year one out | **no** | NonMatch |
| misspelt, and only a birth year known | **no** | Review |
| misspelt, and no birth date known | **no** | Review |
| accent lost | yes | Review |
| accent lost, and only a birth year known | yes (since 2026-11-25) | Review |
| another script | yes | NonMatch |
| another script, and day and month swapped | yes (since 2026-11-25) | NonMatch |
| family and given name swapped | yes | Review |
| family and given name swapped, and day and month swapped | yes (since 2026-11-25) | Review |
| half of a double family name | yes | Review |
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
| twins, one letter apart (Kofi, Kofe) | Review (AutoLink until 0.9.0) | 0.969 |
| parent and child of one name | Review | 0.840 |
| a common name, another birth date | NonMatch | 0.600 |

**Until matcher 0.9.0 twins whose given names differ by one letter would have been linked automatically**, on a family name, a birth date and one similar given name with no identifier and no address. Since 0.9.0 no pair is without a verified identifier that both records carry (AG-13). Nothing in the service links by itself: registration and the HL7 feed refuse a likely duplicate, a merge needs a person and a reason, and a test holds that no code acts on `AutoLink`.

## What would help, in order

1. ~~Compare identifiers by what they mean and not by how they were typed~~: done (AG-12, migration 45), for national numbers only.
2. ~~Retrieve by a folded family name and by the names swapped, and by a birth date with its day and month swapped~~: done (AG-12, migration 46).
3. Retrieve by a name that sounds alike, and by the year when only a year is known (AG-12): both widen the list much more, so they need a measured cost and a site's own frequencies.
4. Require an **identifier** before any automatic decision (AG-13, OQ-AG8): **done 2026-11-25** (decided: no), in matcher 0.9.0, which the service uses. An earlier version of this page said "an identifier or an address"; an address or a telephone number is not enough, as the next section shows.
5. A site's own evaluation, with adjudicated pairs and denominators, before any of this is believed (AG-8, ID-1).

## A place that many people share (XP-5, 2026-11-25)

A prison, a barracks, a hostel or a care home puts one postcode, and often one telephone number, on everybody's record, and the matcher counts each as evidence about one person. `tests/shared_place.rs` compares invented pairs of **two people** four ways and pins the decision: `n` not a match, `R` to a person, `A` the matcher would link. **The table is of matcher 0.8.1; with 0.9.0 every `A` is `R`**, and nothing else changes.

| Pair, two people each | Nothing shared | Postcode | Telephone | Both |
|---|---|---|---|---|
| One birth date and nothing else (Okafor, Nowak) | n | n | n | **R** |
| One family name and one birth date, given names unlike | n | **R** | **R** | **R** |
| Like names, one birth date: Hassan Reed, Hussain Reid | R | **A** | **A** | **A** |
| Like names, one birth date: Martin Hanson, Marie Harrison | R | **A** | **A** | **A** |
| One name, birth dates apart by a digit, a year, or day and month | R | R | R | R |
| One name, birth dates far apart | n | n | n | n |
| One name and one birth date | A | A | A | A |
| One letter apart in one name, one birth date | A | A | A | A |

- **With 0.8.1 a shared postcode alone turned "ask a person" into "link".** In a search of 63 family and 51 given names, chosen by the author, 14 pairs of people were tipped; each shares a birth date. The two above are pinned.
- **It also makes work.** Two people with one birth date and no name in common reach the list for a person when postcode and telephone are shared. My arithmetic, not a measure: with 1,000 people in one place and birth dates spread evenly over 50 years, about one newcomer in 18 shares a birth date with someone there.
- **An administrative sex was left out of every pair.** Where both records carry one and they differ, the score falls; that was not measured.
- Nothing in the service links by itself. With 0.9.0 what is left is work: pairs that reach a person because they share a place (XP-16).

**What would help:** no automatic decision without an identifier (done, OQ-AG8). Then a postcode or telephone number known to belong to a place (a list the site keeps) counts for nothing: a rule of the matcher, and a site's list, not built. Not a list the system guesses from how many share it: that would take evidence from a large household too.
