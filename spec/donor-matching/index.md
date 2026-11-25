# Information for matching donors: blood, kidney, bone marrow and others (DM-1)

Asked for on 2026-11-24 ("add information for matching blood donors, kidney donors, bone marrow donors, etc."). **Built 2026-11-25, service only: keeping what laboratories report. Nothing matches anyone.** No screen, no FHIR, no laboratory feed, and no transfusion scientist, histocompatibility scientist, transplant clinician, donor or recipient has read it. NOT FOR CLINICAL USE. Hazard H-069. Sources below were read **as search summaries on 2026-11-24, not in full**, and some are old or not the guideline itself; check each before relying on it.

## The decision: keep reports, never match

Matching belongs to laboratories, registries and transplant teams, with their own samples, crossmatches and rules that change. A record that said "compatible" would be trusted at the moment it is most dangerous: a wrong blood group can kill. So the record keeps **reports**, says whether separately collected samples **agree**, and stops.

- It never says what a person's group is, and never that two people match.
- It holds no donor, recipient, crossmatch, score, waiting list or registry entry, and no search across patients. `tests/http_typing.rs` fails on a route or schema named for one.
- It is not a source for issuing blood. The laboratory tests its own sample.

## What bears on compatibility, and what is kept

| For | What matters, from the sources | Kept (DM-1) | Not kept |
|---|---|---|---|
| **Blood** | ABO and RhD; red cell antibodies. UK practice asks for **two samples, collected and identified separately**, before group-matched red cells are issued where no secure electronic identification is in place [NBT] [BSH] [ROTH] | `abo`, `rhd`, `red-cell-antibody` | Other red cell groups as a set, transfusion history, reactions, special requirements such as irradiated blood |
| **Bone marrow and stem cells** | High-resolution HLA-A, -B, -C, -DRB1, -DQB1 (a 10/10 match), and -DPB1 (12/12); the United States counts 8/8 without DQB1 [FUNG] [BSHI]; cytomegalovirus status to choose between equal donors [BSHI]. DRB3 and DRB4 are named as relevant too | `hla` (11 loci, one or two alleles, the database release), `cmv-igg` | Permissive DPB1 groups, donor age, sex and ABO as a choice, donor-specific antibodies as a comparison |
| **Kidney** | Blood group; HLA mismatch; HLA antibodies, as unacceptable antigens and a calculated reaction frequency; a virtual and then a real crossmatch; waiting time, age and location in the UK scheme [BTS] [ODT]. KDIGO advises typing of HLA class I and II for donor and recipient [KDIGO] | `abo`, `hla`, `hla-antibody` | Reaction frequency, unacceptable antigens as a list for a scheme, crossmatch, kidney function, size, anything about a scheme |
| **Others** (liver, heart, lung, cornea, tissue) | Not researched | Only what the above already hold | Everything else |

## Rules

1. **Every report names its laboratory, its sample and when the sample was taken.** The two-sample rule is about collection, so two tubes with different numbers taken at one time count as **one collection**.
2. The summary of ABO and of RhD is `not-reported`, `one-collection`, `agree` or `disagree`. **Any difference is `disagree`**, however many agree, until one is marked entered in error. Agreement is not permission, and a group can change after a stem cell transplant.
3. After a merge the reports of both records are in one list, so two people merged by mistake may show as `disagree`. This is a help and not a check.
4. An HLA allele is its fields (`02:01`), without the gene and without a list of possibilities, with the **release** of the database its name is from [HLA]. One allele is *one seen*, never two the same. Nothing is looked up, so a name that does not exist is accepted.
5. *Not detected* is of one sample on one day. An antibody found once and not found later is still a fact.
6. A report has no note and no place for another person.
7. Versions, reasons, entered in error and reads [as at a time](../as-at/index.md) are as for the [problem list](../problem-list/index.md).

## Secrets: an HLA type can show who is not related

One study put father and child kidney pairs sharing less than one haplotype at about 1 to 3 percent of such pairs, and found donors and clinicians divided on telling them [YOUNG]; KDIGO advises telling candidates beforehand what may be found and what will be done with it, and protecting a donor's privacy [KDIGO]; a children's hospital asks families to say in advance who should be told [GOSH]. So: a clinician keeps the reports, a nurse reads them, **nobody else does**, a patient and proxy read none (OQ-DM2), and nothing here compares two people. It is genetic data. It is linked to [relative history](../relative-history/index.md) by nothing.

## Not decided

| Id | Question |
|---|---|
| OQ-DM1 | What does the customer need: to see a patient's own reports, to send a person to a registry, or a transplant or transfusion workflow? |
| OQ-DM2 | What does a patient see of their own typing, and what are they told beforehand about what it may show? |
| OQ-DM3 | Which laboratory system and registry would reports come from, in what format (HL7 v2, FHIR genomics reporting, HML)? A person typing a blood group is the weakest source there is. |
| OQ-DM4 | Which law governs donor consent, anonymity and living donation where it is used? A pack, not the core. |
| OQ-DM5 | May a relative who is a possible donor be referred for typing, by whom, without this record linking two people? |

## Tasks

DM-1 to DM-5 in [tasks](../../tasks.md).

## Not done, not verified

No guideline was read in full and the BSH wording was not seen. The current UK kidney scheme's weights, haploidentical donor choice and donor-specific antibodies were not found. No LOINC or SNOMED code is used: the ABO and RhD codes were not checked. The Human Tissue Authority's guidance was not found. No one who does this work was asked.

## Sources (search summaries, 2026-11-24)

- [NBT] North Bristol NHS Trust, two sample rule: https://www.nbt.nhs.uk/severn-pathology/pathology-services/haematology/blood-transfusion/blood-transfusion-two-sample-rule
- [BSH] British Society for Haematology, pre-transfusion compatibility procedures (2012, with a later addendum not read): https://b-s-h.org.uk/guidelines/guidelines/pre-transfusion-compatibility-procedures-in-blood-transfusion-laboratories ; NHS National Services Scotland: https://www.nss.nhs.scot/blood-tissues-and-cells/transfusion-laboratories/pre-transfusion-tests/
- [ROTH] The Rotherham NHS Foundation Trust, blood bank handbook: https://therotherhamft.nhs.uk/sites/default/files/2025-02/Blood%20Bank%20Laboratory%20User%20handbook.pdf
- [BSHI] BSHI guideline, HLA matching and donor selection: https://discovery-pp.ucl.ac.uk/1508614/1/Little_BSHI%20Guideline%20AAM.pdf
- [FUNG] Reviews of HLA matching, several papers read as one summary: https://pmc.ncbi.nlm.nih.gov/articles/PMC6913495 ; https://pmc.ncbi.nlm.nih.gov/articles/PMC8525224
- [BTS] Papers on the UK kidney offering scheme: https://pmc.ncbi.nlm.nih.gov/articles/PMC9292213 ; https://www.frontierspartnerships.org/articles/10.1111/tri.13776/pdf
- [ODT] NHS Blood and Transplant, organ offering algorithms: https://www.gov.uk/algorithmic-transparency-records/nhs-blood-and-transplant-organ-offering-scheme-algorithms
- [KDIGO] KDIGO 2017 living kidney donor guideline: https://pmc.ncbi.nlm.nih.gov/articles/PMC5540357/
- [YOUNG] Young et al., Transplantation 2009;87(10):1429: https://pubmed.ncbi.nlm.nih.gov/19461476/
- [GOSH] Great Ormond Street Hospital, tissue typing for kidney donation: https://www.gosh.nhs.uk/conditions-and-treatments/procedures-and-treatments/tissue-typing-kidney-donation/
- [HLA] HLA nomenclature: https://hla.alleles.org/pages/nomenclature/naming_alleles/
- Not used, for OQ-DM3: FHIR genomics reporting, histocompatibility: https://hl7.org/fhir/uv/genomics-reporting/histocompatibility.html ; NMDP HLA reporting: https://fhir.nmdp.org/ig/hla-reporting/
