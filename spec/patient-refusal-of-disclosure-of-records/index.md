# A patient's refusal of disclosure of records

Researched 2026-11-25 from public sources and from this repository. **A proposal: nothing on this page is built, and no lawyer, clinician, Clinical Safety Officer, data protection officer or patient has reviewed it. It is not legal advice.** What is built already is said where it is. The other refusal, of treatment, is [its own page](../patient-refusal-of-consent/index.md): the two are kept apart in the data, the words and the screens.

Related: [masking and labels](../access-control/masking.md), [life-or-limb emergencies](../access-control/index.md#life-or-limb-emergencies), [protected information](../access-control/protected-information.md), [patient portal](../patient-portal/index.md), [custodial health](../custodial-health/index.md), [privacy analytics](../privacy-analytics.md), [audit](../audit/index.md). Marks: **P** the official text was read, **P-copy** a copy or an older version, **S** a summary or search extract, **NE** not established.

## The short answer

1. **There is no one rule.** Some places bind the holder to a refusal, some only ask that it be considered, some have no refusal for the main record at all.
2. **Whether a clinician may know that something is hidden is the sharpest difference, and it is law in some places.** EU law for shared records: a restriction "shall not be visible to healthcare providers". Sweden: it must be. Alberta shows a lock.
3. **An emergency does not mean the same everywhere.** In Dubai, Switzerland and France a patient can shut emergency access too. In Japan's emergency viewing a refusal is named as a case for it. Italy's beats a missing consent and not a hidden document.
4. **So these are the tenant's settings, from a reviewed pack**, and two decisions already made here have to become settings (below).
5. **A refusal never stops everything.** Courts, notifiable disease, child protection: the list is the place's, and the patient is told of it when they refuse.
6. **A refusal that can be inferred is not kept.** A hidden diagnosis with its medicine shown is not hidden.

## Two decisions here that some places would not allow

| Decided 2026-11-25 by the maintainer | Places that differ, as read | What is proposed |
|---|---|---|
| OQ-PI5: the care team is told how many entries are withheld | **EU EHDS art. 8** (P; from 2029 and 2031, for shared access across providers), **Germany** (P, the national record), **Italy** (P: users may not learn that the choice was made or that data exist). Whether these reach a hospital's own record was not established | A setting, `flag`. **The pack decides; a count is only the default where the pack allows it.** In a place that forbids it the answer is `unstated` for everyone, as it is for patients now |
| OQ-PI2: break-glass shows everything | **Dubai** (P: data opted out of "should not be retrieved" by break-glass), **Switzerland** (P-copy: unless the patient excluded it), **France** (P-copy: unless express opposition before), **Italy** (P: not what the patient obscured), **Sweden** (P: ask the blocking provider to lift it) | A setting, `emergency`. Labels set by staff and refusals made by the patient are separate inputs to it |

Nothing is wrong today: no tenant exists, and break-glass lifts no refusal (OQ-PI8).

## What the settings must be able to say, and who does it that way

### Whether a refusal for direct care exists, and how strong

| Model | Places |
|---|---|
| Binds the holder or the system | Ontario (P), Quebec (P, as enacted), British Columbia's banks (P), Australia's national record (P), Germany's (P), Italy (P), Sweden (P), Finland (P), France's masking (P), Switzerland (P), Dubai (P) |
| Must be considered, may be declined with reasons | Alberta (P); US HIPAA: "not required to agree", save payment in full (P); New Zealand: the holder "may refuse" (P, 2020 text) |
| A professional duty or a right to object | England, GMC para 30 (P, by a copy): do not disclose unless public interest or overall benefit without capacity. Scotland's code (P, old). Northern Ireland's code (P) |
| None found for the main record | Northern Ireland's encompass: "you cannot opt out" (P), **against its own code**. Poland for the treating group (P). Local records in several places (NE) |
| Opt-in: nothing is shared without a yes | Netherlands between systems (P), Switzerland (P), Finland (P), South Korea (P), US substance-use records (P), Israel's portability law, not yet in force (P) |
| Passed, not in force | Ireland s.14 (P), EHDS (P), Singapore's Act (S), India's main duties until 2027 (P) |

### What may be refused

A whole record; a kind; one entry; one visit; a period; a named person; a profession; a unit or provider; a purpose; everything from now on. Denmark: a named person, unit or period "depending on what is technically possible" (P). Ontario's operator has offered only the whole since February 2026, though the statute says "in whole or in part" (P). Some things cannot be hidden: name, birth date, gender, number and death (Ontario, Alberta, P); allergies and vaccinations (Singapore, S).

### What the reader is told

| Rule | Places |
|---|---|
| Existence hidden | EHDS (P), Germany (P), Italy (P), Australia (S) |
| Existence shown, not the content | Sweden across providers (P), Spain (P, by a reading tool), Alberta's lock (P), Ontario's notice (P) |
| **Existence shown to safeguard a child's hidden document** | France, documents hidden from parents (P, regional health agency) |
| The category must not be given away by an answer | US substance use, 42 CFR 2.13(c)(2) (P) |
| Not found | England, Scotland, Wales, Northern Ireland, Ireland, Netherlands, Denmark, Switzerland, Poland, most of Asia and Latin America |

### The emergency

| Question | Values, with a place |
|---|---|
| Does it exist? | No clause found: Germany's record, Netherlands (S). Yes elsewhere |
| What must be true? | The person cannot be asked: Ontario, Australia, France, Italy, Switzerland, Northern Ireland and England by overall benefit. **Vital interests alone: EHDS, Ireland's Act.** A threat to life: Poland. "Including where refused": Japan (P) |
| Does it beat an earlier refusal? | No: Dubai, France, Switzerland, Italy's obscured documents. Yes: Spain's emergency mode, Ontario and Australia by their own tests, EHDS if the state did not allow an absolute opt-out |
| Who, how? | Physicians only: Dubai. Signing in again and declaring the emergency: Singapore (S). Two factors, hospitals only: Japan. Reason, warning, per case: Dubai. A maximum: 5 days from advising the operator (Australia) |
| Told afterwards? | Ontario, Switzerland, France, EHDS: yes. Sweden: on request. Not required in the text: US. Commissioner told too: Ontario, risk to others |
| Asked of the receiver? | Not to use or pass on further: US |

### The rest, in one table

| Setting | Range found |
|---|---|
| Form and time | Spoken or written (Denmark); signed notice (Quebec); online at once (Italy); 3 business days (Alberta); 2 to 5 (Dubai); "immediately" (Sweden) |
| What the patient is told first | That care may be affected (EHDS, Ireland, Italy says it is not prejudiced, Alberta, Dubai). **What it will not stop: hardly anywhere** |
| Care not refused for it | Australia s.46, Quebec s.10, Italy art. 8(8), Germany s.335 |
| Author keeps sight of their own entries | Italy, France, Finland, EHDS recital; Spain NE |
| Linked things hidden too | Italy: hiding a prescription hides its dispensing and report (P). Ontario: interaction alerts allowed if they "do not reveal" (P) |
| Secondary use | A separate opt-out: England, EHDS art. 71, Germany, Sweden (registers), Northern Ireland's code. Japan: notice and 30 days. None found: Ontario, New Zealand, Denmark for research |
| Not stopped by a refusal | Courts; notifiable disease; child protection; coroner; gunshot and stab wounds (Ontario, BC, Alberta); female genital mutilation of a girl (England, Wales); police in listed cases. **Tell the patient unless it defeats the purpose or endangers someone**: Scotland's and Northern Ireland's codes (P) |
| Family | Told unless objected (New Zealand, Ontario's status only, US); **told by custom or duty** (Mexico, Saudi Arabia); only with consent (Brazil, Denmark) |
| Employer, insurer, immigration, police | Never for these in some systems (Netherlands, Switzerland, Australia, EHDS); immigration status in Quebec (P) |
| Categories with extra rules | Italy: HIV, abortion, violence, drugs, anonymous birth shown only to the patient by default. Germany: duty to point out the right to object. US: substance use, psychotherapy notes. Dubai: role-restricted. England, Wales and Scotland: gender recognition. Many NE |
| Young people | Refuse a parent: Ontario (capable, under 16), Quebec 14, Denmark 15, Netherlands 12 and 16, Switzerland 12 to 15 object and 16 alone, Sweden (parents may not block; a mature minor may), Finland (capable, per visit), South Africa 12, New York over 12 may object, Australia 14, England from 11 by assessment. **Texas gives a parent "complete and unrestricted access" (P)**. France: only a professional's decision until 18 |
| Adults who cannot decide | Representatives, by order; some may refuse and some may only permit |
| After death | Binds: Netherlands from 12, Spain, Poland (a court may override), France, England (a note), US (known preference), Quebec, Alberta, Northern Ireland's code. 2 years, 10 years, 30, 50, 70: the length differs |
| Log and notice | Kept 3 years (EHDS, Germany), 10 (Alberta, Switzerland). The patient sees it: most. Told each time: EHDS, France, Singapore on request |
| Withdrawal of a permission | Not retroactive (most). Already shared is not called back; a recipient must delete (Israel, not in force) |
| Expiry of a refusal | None found to expire, save that Wales does not carry it on a change of practice |
| Evidence of refusal kept | 7 years then deleted (Israel, not in force). Elsewhere NE |
| Place of data | Texas and the UAE forbid keeping it abroad, as read |

## What is built here today

| | Now | Gap |
|---|---|---|
| Refuse by | Role, purpose, kind of record, label, period (`consent.rs`) | **A named person, an organisation, one entry, one visit** |
| Strength | Binds; a deny beats a permit; unreadable is a deny | No "considered"; no reason kept for declining |
| What is told | The care team a count; others `unstated` (`masking.rs`) | By setting |
| Emergency | Break-glass lifts no refusal; it lifts labels | By setting; who cannot be asked (PI-10) |
| Merge | Refusals follow, permissions do not (OQ-AG14) | |
| Secondary use | National opt-out as a kind of consent | The rest |
| Young people, proxies | Proxy by scope and date | No age rules; no hidden-from list |
| After death | Access closes by the death rules (bereavement) | Refusal after death |
| Log | The patient sees theirs | Notice |

## Cases the record and the screens must tell apart

| Axis | Values |
|---|---|
| Who refuses | the patient; a young person against a parent; an attorney or guardian; a parent for a child; **someone at their side** |
| Can they be asked now | yes, no (a clinician says so), fluctuating, not known. As on the [treatment page](../patient-refusal-of-consent/index.md#cases-the-record-and-the-screens-must-tell-apart), and the same record is used |
| Threat | to life, limb, serious deterioration, to someone else, none, not known |
| From whom | everyone, a person, a role, a unit, an organisation, a place, family, a parent, an employer |
| What | all, a kind, an entry, a visit, a period, the future |
| For what | care here, care elsewhere, emergency care, research, planning, registry, teaching, insurer |
| In an emergency | holds, does not hold, **not said** |
| State | asked for, in force, declined (by whom, why, told when), changed, withdrawn, in force by law only in part |
| Compelled | none; by court, statute or duty, with what was given, to whom, and whether the patient was told, or why not yet |

**Tests a design must pass:** a refusal of one nurse who is also the only pharmacist on duty; a refusal of a relative who works here; a teenager's contraception and a parent's portal; separated parents; a label and a refusal on one entry; a hidden diagnosis with its medicine, its result, its appointment and its letter; a list that is empty because all is hidden; a refusal made on one record and then a merge; a refusal and a referral out; a refusal in a place that binds and a transfer to one that does not; a court order for what is hidden; a patient read over the shoulder; a patient who dies; a refusal "in an emergency too" and an unconscious patient; **a clinician shown nothing who prescribes against what is hidden**; a downtime printout made the day before.

## Settings of the tenant

A file per deployment today (one deployment, one tenant: QA-12, SE-5), proposed `EPR_DISCLOSURE_RULES_FILE`, TOML, read once at start, with the same head as the treatment pack: place, `law_as_of`, lawyer, data protection officer, clinician, `reviewed_on`, `review_due`. Every setting carries its source. With true tenancy it is held per tenant, versioned and append-only, and **a past decision is explained by the version that stood then**.

| Setting | Values | When absent |
|---|---|---|
| `model` | `opt-out`, `opt-in` | `opt-in` for sharing outside; care here is not refused to the team that gives it |
| `strength` | `binds`, `considered`, `duty` | `binds` |
| `scopes` | Which of the scopes above are offered | Those built |
| `always_visible` | Fields that cannot be hidden | Name, birth date, number, death |
| `flag` | `none`, `existence`, `count`; for whom | `count` to the care team, `unstated` to others (OQ-PI5) **unless the pack's place forbids it, which the pack must say** |
| `emergency.exists`, `.needs`, `.beats_refusal`, `.beats_label`, `.roles`, `.again_sign_in`, `.hours` | as the tables | Exists; needs "cannot be asked"; **does not beat a refusal**; beats a label (OQ-PI2); treating roles; no; 4 |
| `emergency.notice` | `at-once`, `on-request`, `none` | `on-request`, in the access log |
| `patient_may_close_emergency` | yes, no | yes |
| `author_sees_own` | yes, no | yes, named as such |
| `linked` | What is hidden with what | The chain of an entry: result, order, medicine, letter, appointment |
| `safety_alerts` | `run-without-revealing`, `off` | `off` until a clinical owner agrees |
| `decline` | Grounds, who, time, how told | Not offered |
| `compelled` | List of duties: what, to whom, tell the patient or why not | Nothing is automatic; a person decides each |
| `secondary` | Separate choices and defaults | All off |
| `family`, `employer`, `insurer`, `police`, `immigration` | `never`, `with-consent`, `unless-objected`, `by-law-only` | `with-consent`; employer, insurer and immigration `never` |
| `young` | Ages and what each may refuse, and of whom; portal defaults | No parent is refused and no minor's entry is shown to a new proxy: a person decides |
| `death` | Who may refuse, for how long | Refusals stand |
| `log_years`, `patient_log` | | For ever; shown |
| `carry` | Whether a refusal goes with a referral or transfer | It goes |
| `forms` | Spoken, written, online; time to act | Spoken is enough; at once |
| `categories` | Labels that start hidden, and to whom | None (OQ-PI7) |
| `residency` | Where data may be kept | Not set: the deployment's |

**Not settings, for any tenant:** every read, refusal and override is audited; nothing is deleted; an unreadable refusal hides; an empty list that may be partial never says "none"; a refusal is never shown to the person it is against; a declined refusal says who and why; the patient can see what stands; a clinician who cannot see does not see it through an export, a letter, a printout, a search, a count or a suggestion. **A pack that contradicts itself does not load:** for example `flag = "none"` with `notice` text that names the hidden thing, or `beats_refusal` with `patient_may_close_emergency`.

## Schemas, proposed

An addition to today's consent provision, all optional, all closed lists:

```json
{
  "effect": "deny",
  "actors": { "roles": [], "people": ["subject"], "units": [], "organisations": [], "relations": ["parent", "family"] },
  "purposes": [], "resources": [], "labels": [], "period": null,
  "items": ["Condition/…", "visit:…"],
  "future": true,
  "in_emergency": "holds | does-not-hold | not-said",
  "asked_by": { "who": "patient | young-person | attorney | guardian | parent", "in_private": "yes|no|not-known" },
  "told": ["care-may-be-affected", "what-it-does-not-stop"],
  "decision": { "state": "in-force | declined", "by": "…", "why": "…", "told_on": "…" }
}
```

`people` is the staff token subject and is as protected as the entry: the person refused is not told. Compelled disclosures are their own append-only record (`what`, `to`, `basis`, `told`, `not_told_because`, `review_on`). **Decision explained:** an access decision keeps the ids of the provisions and the version of the rules it used, and not their words.

## Screens, proposed

1. **For the patient:** "Who can see what", in their words, with what it will not stop, before they choose. A private way in and a quick way out. Nothing that a person beside them could read as a refusal of them.
2. **For staff recording one:** who asks, in private or not, what, from whom, in an emergency too or not or not asked. A warning when what is chosen can be inferred from what is left.
3. **For the reader:** by `flag`. If a count, beside the list and not in a corner. If nothing, **nothing at all**: no gap, no different wording, no slower answer that tells.
4. **In an emergency:** can they be asked; who says so; the threat; sign in again if set; what will be shown and what will not; that the patient will be told. The pack's rule with its date.
5. **For a declined refusal:** who, why, and how to complain.
6. **For the privacy officer:** what stands, what was overridden, what was compelled, what is overdue.
7. In each interface language by a speaker, at 400%, with a screen reader; words tested with people who were watched when they used a portal.

## Candidate hazards

For the Clinical Safety Officer and the data protection officer; none is added to the log, since nothing is built. H-006, H-007, H-024, H-072 and H-073 already stand.

1. A clinician is told nothing, by law, and prescribes against what is hidden.
2. The flag, the gap or the delay tells the person who is the danger.
3. A refusal is inferred from a medicine, a clinic, a count or a letter.
4. A refusal of a colleague is seen by the colleague.
5. An override by someone who could have asked.
6. A compelled disclosure is made and the patient is told by the wrong route.
7. A parent sees a young person's entry on the day a default changes: a birthday, a merge, a new proxy.
8. A pack is out of date, or of the wrong place, and a right is denied.
9. A refusal does not travel with the record, or travels and is lost.
10. A refusal is taken for a refusal of treatment.

## Order of work, proposed

Tasks DR-1 to DR-12 in [tasks](../../tasks.md). Nothing is started.

## Open questions

Summarised as OQ-DR1 to OQ-DR8 in [open questions](../16-open-questions.md).

1. Do OQ-PI5 and OQ-PI2 become pack settings, with today's answers as the defaults where the place allows? Recommendation: yes. (OQ-DR1)
2. Which places are first? (OQ-DR2)
3. Where the pack is missing or past its review, does the service start? Recommendation: start only in a sandbox without one; past its date, warn on each screen that uses a rule. (OQ-DR3)
4. May a patient close emergency access too, where the law does not say? Recommendation: yes, after being told what it can cost. (OQ-DR4)
5. May a patient refuse a named member of staff, and who finds another? (OQ-DR5)
6. Do safety alerts run on what the clinician may not see? Recommendation: ask clinicians and patients first; an alert with no reason is either ignored or a disclosure. (OQ-DR6)
7. Is the hospital's own record inside the EU rule on visibility, or only access across providers? A lawyer's question. (OQ-DR7)
8. Who is asked: people living with HIV, survivors, young people, people in custody, staff who are patients here? (OQ-DR8)

## Not done, not verified

- Three automated readers working apart; their reports agree where they overlap, and were not checked again at the source. Many national sites refused automated reading and were left alone: among them England's and Scotland's, the regulator's, France's and Estonia's statutes, Singapore's, Quebec's site and parts of the Gulf's.
- **Not established almost anywhere:** police, school, prison and immigration defaults; separated parents; looked-after children; what a clinician is told; whether the patient must be told what a refusal will not stop.
- Denmark, Finland, Sweden and others were read in versions whose later amendments were not checked. Laws passed and not in force are marked.
- Portal rules outside those named, and every country not named, were not looked at.
- The wording on a screen was not tested with anyone.

## Sources

Read 2026-11-25; the main ones. Quotations are short and are from these.

- EHDS, Regulation (EU) 2025/327, arts. 8 to 11, 71, 105: https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=OJ:L_202500327
- England: GMC Confidentiality, on a copy: https://bsw.icb.nhs.uk/wp-content/uploads/sites/6/2022/06/Confidentiality-good-practice-in-handling-patient-information.pdf ; HSCA 2012 s.251B: https://www.legislation.gov.uk/ukpga/2012/7/section/251B
- Northern Ireland, encompass: https://belfasttrust.hscni.net/about/encompass/encompass-faqs ; Wales, opt-out form: https://dhcw.nhs.wales
- Ireland, Health Information Act 2026: https://www.irishstatutebook.ie/2026/en/act/pub/0010/print.html
- Ontario, PHIPA: https://www.ontario.ca/laws/statute/04p03 ; Alberta, masking: https://www.albertanetcare.ca/learningcentre/Netcare-Masking.htm ; British Columbia, E-Health Act: https://www.bclaws.gov.bc.ca
- Australia, My Health Records Act 2012: https://www.legislation.gov.au/C2012A00063
- United States: 45 CFR 164.522: https://www.law.cornell.edu/cfr/text/45/164.522 ; 42 CFR Part 2: https://www.ecfr.gov/current/title-42/chapter-I/subchapter-A/part-2
- Texas, SB 1188: https://capitol.texas.gov
- Germany, SGB V s.353: https://www.gesetze-im-internet.de/sgb_5/__353.html ; what is hidden: https://gesund.bund.de/wie-verwalte-ich-meine-epa
- France: https://www.normandie.ars.sante.fr/media/125961/download ; L1111-17 in the Constitutional Council's file 2024-1101 QPC
- Italy, decree of 7 September 2023: https://www.gazzettaufficiale.it/eli/gu/2023/10/24/249/sg/pdf
- Sweden, Lag 2022:913: https://www.riksdagen.se/sv/dokument-och-lagar/dokument/svensk-forfattningssamling/lag-2022913-om-sammanhallen-vard-och_sfs-2022-913/
- Spain, the ministry's questions: https://www.sanidad.gob.es/areas/saludDigital/historiaClinicaSNS/faq/faqProfesionales.htm
- Netherlands, Wabvpz BWBR0023864; Denmark, Sundhedsloven: https://www.retsinformation.dk/eli/lta/2024/247/dan/pdf ; Finland, Act 703/2023; Switzerland, SR 816.1; Poland, Dz.U. 2024/581
- Brazil, Lei 15.378/2026: https://www2.camara.leg.br/legin/fed/lei/2026/lei-15378-6-abril-2026-798918-publicacaooriginal-178753-pl.html ; Mexico, LFPDPPP: https://www.diputados.gob.mx/LeyesBiblio ; South Africa, National Health Act: https://www.gov.za/sites/default/files/gcis_document/201409/a61-03.pdf
- Japan: https://www.mhlw.go.jp/content/10200000/001252407.pdf ; South Korea: https://elaw.klri.re.kr/eng_service/lawView.do?hseq=64153&lang=ENG ; China, PIPL: http://en.npc.gov.cn
- Dubai, DHA standard ST-09 (2025): https://www.dha.gov.ae ; Saudi Arabia, Law of Practising Healthcare Professions: https://www.moh.gov.sa ; Israel, Patient's Rights Law: https://www.btl.gov.il/Laws1/00_0037_000000.pdf
