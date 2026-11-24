# Relative history: inherited and shared-risk history (EH-3)

**Built 2026-11-24, service only**, and reshaped the same day on the maintainer's instruction. No screen, no FHIR, no clinician or clinical geneticist review. NOT FOR CLINICAL USE. The idea's source is [openEHR and `openehr-rust`](../lessons/openehr-rust/index.md): an entry about a relative that does not say so reads as a finding about the patient. Hazard H-067. For CF-4.

## Three questions in place of one word

"Mother" or "brother" mixes facts that matter for different reasons. They are asked apart, and none is guessed from another.

1. **Genetic link** (`genetic`, required): `genetic-parent`, `full-sibling`, `half-sibling`, `identical-twin`, `fraternal-twin`, `genetic-child`, `grandparent`, `aunt-uncle`, `cousin`, `none`, `unknown`. An inherited condition follows it. A `line` (`maternal`, `paternal`, `unknown`) is required where the link has a side, and refused where it has none. `none` is an adoptive, step or foster relative, a partner or a surrogate, and **is never counted towards inherited risk**. A word such as "brother", with no more known, is `unknown`, not a full sibling.
2. **Gestational link** (`gestational`: `yes`, `no`, `unknown`): whether they carried the pregnancy. Infections and exposures in pregnancy follow it. Required for a genetic parent whose line is not paternal, since an egg donor did not; refused for a twin, a child and a paternal genetic parent; open to anyone else, since an aunt or grandmother can be a surrogate.
3. **Shared environment** (`environment`, required: `childhood`, `adulthood`, `both`, `never`, `unknown`): when they lived in the same place, in the patient's life.

**The legal or social role is left out** (adoptive, step, foster, guardian): it does not help to assess a risk and it tells of an adoption.

| Person | genetic | line | gestational | environment |
|---|---|---|---|---|
| Birth mother who raised the patient | genetic-parent | maternal | yes | childhood |
| Egg donor | genetic-parent | maternal | no | never |
| Sperm donor | genetic-parent | paternal | | never |
| Surrogate | none | | yes | never |
| Adoptive father | none | | | childhood |
| Half-brother by the same mother, raised apart | half-sibling | maternal | | never |
| Identical twin | identical-twin | | | childhood |

`sex` (at birth: `female`, `male`, absent if not stated) is optional, because some inheritance depends on it. Two things differ from what was first suggested to the maintainer: a half-sibling's side is in `line` and not in the link's own word, so that it can be unknown; and `sex` was added, since "sister" and "brother" are no longer said.

## Other rules

1. An entry is its own kind of record and never appears on the problem list.
2. **A relative with no genetic link, no pregnancy and no shared environment is refused**: there is nothing to record.
3. **A relative is never named.** There is no place for a name, a birth date, an identifier or a note, and a field the record does not know is refused. A number tells two apart. The relative has not agreed to be in this record (H-065; OQ-CF3).
4. **Present, absent and unknown are three findings.** An empty list is `not-recorded`. The summary counts `inherited`, `not_inherited` and `link_unknown` apart.
5. **An exposure belongs on the patient's own record** ("lived with a smoker, 1990 to 2004"). Copying a relative's health facts for a shared environment is second best; a place for exposures is not built (EH-17).
6. An age at onset is in whole years and only for a condition the relative had.
7. The source is required. **A relative's own health record is not a source** (OQ-EH4).
8. Every change is a new version with a reason; entered in error is final and is a clinician's.
9. A patient and a proxy read none yet (OQ-EH1). A pharmacist reads none.
10. Nothing is worked out from it: no score, no pedigree, no recall.

## It is not a family view, and the word stays refused

[Survivor grief](../survivor-grief/index.md) refuses a family or household view (SG-2). The maintainer decided (2026-11-24, OQ-EH5) that this is kept **and that the guard against the word "family" continues**. So the feature was renamed, `tests/grief_census.rs` is as it was before EH-3 with no exception, and `tests/relative_history_census.rs` extends the refusal to every path, policy resource and schema name. A name does not make a thing safe, so that test also holds what does: an entry sits under one patient, has nowhere to put another person, and nothing lists entries across patients.

## Danger: an entry can reveal a secret

A link of `none`, a half-sibling, or a genetic parent who did not carry the pregnancy can tell of an adoption, a donor, a surrogate or a parentage the patient does not know of. This is why a patient and a proxy read none, and it is a reason for staff to read only what they need. Nothing masks it yet (EH-8).

## Routes

`GET` and `POST /api/patients/{id}/relative-history`, `PUT …/{entry_id}`, `GET …/{entry_id}/versions`, `GET …/as-at?time=`. Policy resource `RelativeHistory`; its FHIR type is `FamilyMemberHistory`.

## Not done

The screen and FHIR (EH-15); genetic test results (RC-12); a relative's consent; consanguinity, mitochondrial donation, pregnancy losses and pedigrees. No code system was looked up: HL7 is believed to have codes for natural, gestational, adoptive, half-sibling and twin relations, from memory. The condition is free text of up to 120 characters and **can hold a name**: nothing can refuse that.
