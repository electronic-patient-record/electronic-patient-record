# Equity audit: country assumptions in the core (IO-19)

Under the [equity rule](02-scope.md), this lists where the code today treats one country's way as the default. Found by
searching the three projects for identifiers, names, postcodes, phone codes, time zones, units and locale handling, then
reading the hits (2026-10-29). **A finding is a gap to name, not a defect fixed.** Nothing here has been changed.

| # | Finding | Where | Who it excludes or disadvantages | Proposed pack boundary | Clinical? |
|---|---|---|---|---|---|
| 1 | The only **deterministic identifier** is the NHS number: a shared valid one decides a link, two different valid ones forbid it. No other country's national id (Medicare, IHI, CURP, Aadhaar-style ids, national insurance ids) can do this | matcher `engine.rs` (`valid_nhs_numbers`, `NHS_NUMBER_SYSTEM`), `nhs.rs` | Every deployment outside England, Wales and the Isle of Man: identity matching falls back to scoring alone | An **identifier pack**: system URI, check-digit rule, placeholder rule, and whether a shared value decides or only supports | **Yes**: wrong-patient risk both ways |
| 2 | Name type is `family` (one string) plus ordered `given`. No second family name (Spanish, Portuguese), patronymic, mononym, or name order by culture | matcher `patient.rs` `HumanName` | Spanish and Portuguese double surnames, Icelandic patronymics, Indonesian and other mononyms, family-first cultures | Keep FHIR `HumanName` text and parts; a **name pack** states comparison rules per locale | **Yes** |
| 3 | **Soundex** is English-oriented; **nickname groups** are English and a few Arabic transliterations; `transliterate` handles Latin-script marks and German digraphs only | matcher `normalize.rs` | Names in other scripts get no phonetic signal (the code says so); other languages' diminutives are unlisted | A **name pack** with phonetic and nickname data per language; absence stays "no claim", never "different" | **Yes** |
| 4 | **Phone default country code is 44** (a leading `0` becomes `44`) | matcher `engine.rs` (default), `normalize.rs` | Any deployment outside the UK, where a leading `0` means a different country code | Deployment setting from the site's country, not a constant | **Yes** (a wrong code can fake a match) |
| 5 | `Address` has only `postal_code`, folded to alphanumerics lowercase; address lines, region and district take no part in matching | matcher `patient.rs`, `normalize.rs` | Countries without postcodes or with coarse ones (many have none); address forms that matter (building, village, district) | An **address pack** per country: which parts exist and which carry evidence | **Yes** |
| 6 | **Identifier systems and the amendment/consent model** assume UK Core and US Core as the only references; `metadata` names only those | service `controllers/fhir.rs`, spec `interoperability` | Every other national FHIR guide | National profile packs (IO-18) | **Yes** |
| 7 | **Site time zone** defaults to `UTC` and takes an IANA name; the examples use `Europe/London`. Works worldwide | service `site.rs` | Nobody; the example text only | Keep; change the example to show a second zone | No |
| 8 | **Access-log times** are written as ISO-style `YYYY-MM-DD HH:MM` in 24-hour form, which is neutral; `Intl.DateTimeFormat("en-US")` only supplies the parts, but the **short zone name** (`timeZoneName: "short"`) is English-locale text and varies by locale. Other date displays were not read | UI `lib/access-log.ts` (`whenText`) | Low: a reader outside the US sees an English zone label such as `GMT+1` | Format the zone label with the active locale; read the other date displays | No (checked: output is unambiguous) |
| 9 | **Units shown** map UCUM to `mmHg`, `°C` and so on in one table; there is no per-site unit preference (for example kPa for pressure, °F, lb/ft) | UI `lib/vitals.ts`, service `observations.rs` | Sites that read pressure in kPa, weight in stones and pounds, or temperature in °F | Store UCUM; display units by deployment and user preference, with the stored unit always visible | **Yes** (a unit mix-up is a dose or severity risk) |
| 10 | **Locale set** has ten entries (`ar-001`, `cy-001`, `de-001`, `de-de`, `en-001`, `es-001`, `fr-001`, `hi-001`, `zh-001`, `zh-cn`) and **only English strings exist**; right-to-left is handled for `ar-001` only | UI `lib/locales.ts`, `spec/locales.md` | Speakers of Portuguese, Bengali, Russian, Swahili, Japanese, Indonesian and the many others absent; Hebrew, Persian and Urdu would need the RTL list extended by hand | Locale data as a pack; derive RTL from the script, not a list; add non-Gregorian calendar support | No (labels), **Yes** for clinical terms in a translation |
| 11 | **Regulatory text and roles** name UK and US law and bodies (NHS DSPT/DTAC, DCB0129/0160, HIPAA, UK MDR, ONC) as the compliance frame; other regimes (GDPR member-state law, PIPEDA, Australia's My Health Records Act, India's DPDP Act, Brazil's LGPD, and so on) are not mapped | `spec/12-compliance.md`, `spec/hazard` | Deployers elsewhere find no mapping or starting point | A **regulatory pack** per jurisdiction: what the product supports with evidence, never a claim of compliance | No |
| 12 | **Terminology** assumes SNOMED CT and UK drug dictionary ideas in places; the SNOMED check is by identifier shape only | service `interop/terminology.rs` | Countries using other code systems or no SNOMED licence | **Terminology pack** declaring code systems per country | **Yes** |
| 13 | **Sex** field is FHIR administrative gender only (no separate recorded sex, gender identity or sex-for-clinical-use) | matcher `patient.rs` `AdministrativeSex` | Jurisdictions and patients needing more than the four values; clinical rules that depend on sex at birth | Not a pack: a data-model question for the maintainer | **Yes** |

The proposed mechanism that would absorb these rows is in [`interoperability/index.md`](interoperability/index.md) "Pack mechanism (design, OS-10)", with a table of which file replaces which row.

## What is already neutral

- IDs are opaque and the record is FHIR-shaped, so it can carry any `Identifier.system`.
- `Patient.names` is a list, `birth_date` accepts partial dates, and the time zone is configurable.
- The UI sets `lang` and `dir` from the locale, and URLs carry the locale prefix.

## Not done / not verified

Read from a keyword search, not a census: a finding that did not match a keyword is missing here. Rows 1 to 6, 8 and 9 were read in code (row 8 was first mis-stated as a US date format and corrected after reading `whenText`); rows 7, 10 and 13 are from the files cited; rows 11 and 12 are from the specs and not re-read. The countries named as examples are from memory and unchecked, and their laws are not summarised. No change was made to any code, so every "clinical" row still needs a hazard row (AS-1) before it is fixed.

## Verified 2026-10-30

A second pass checked the from-memory claims against sources and re-read two rows in code. Corrections to the table:

- **Row 4 (phone code).** The 44 is not a constant: the matcher's `MatchConfig.default_calling_code` is a field with a
  serde default of 44 (`engine.rs`). The gap is that the service never sets it (no `calling_code` in the service source
  or config), so every deployment gets 44. The fix is a site setting, not a code change in the matcher.
- **Row 12 (terminology).** `interop/terminology.rs` checks only the shape of a SNOMED CT identifier and returns `Ok` for
  any other system ("nothing here knows its rules"). That is neutral. dm+d appears only in `spec/terminology/index.md`,
  not in code. The row stands as a spec finding, not a code one.
- **Row 11 (regulation).** Re-read: `spec/12-compliance.md` names UK and US regimes plus GDPR Article 9. Confirmed.

Facts for the **identifier pack** (row 1), each with its source; "unverified" means no primary source was found:

| Country | Identifier | Shape and check | Source |
|---|---|---|---|
| Australia | IHI | 16 digits, prefix `800360`, Luhn check, no separators (AU Base invariants `inv-ihi-value-0..2`) | [HL7 AU Base `au-ihi`](https://hl7.org.au/fhir/4.1.0/StructureDefinition-au-ihi.html) |
| New Zealand | NHI | Old form three letters and four digits with a modulus-11 check; new form `AAANNAX` issued from 1 July 2026 with a modulus-23 check character | [Te Whatu Ora, NHI format changes](https://www.tewhatuora.govt.nz/health-services-and-programmes/health-identity/national-health-index/upcoming-changes-nhi) |
| Scotland | CHI | Ten digits: `DDMMYY`, two digits, a sex digit (odd male, even female), then a modulus-11 check; new numbers from August 2026 may use modulus 10 (Luhn) instead | [Public Health Scotland `chi_check`](https://code.publichealthscotland.scot/phsmethods/reference/chi_check.html) |
| United States | SSN, Medicare, MBI | No national patient identifier; the HL7 registry lists `us-ssn`, `us-medicare` and `us-mbi` with dashes removed | [HL7 FHIR identifier registry](https://hl7.org/fhir/R4/identifier-registry.html) |
| Brazil | CNS | 15 digits, weighted sum (15 down to 1) modulo 11; two branches by first digit (1–2 definitive, 7–9 provisional); community test vector `187507943600018` valid | [ANS algorithm file (official, Java)](https://www.gov.br/ans/pt-br/arquivos/assuntos/espaco-da-operadora-de-plano-de-saude/aplicativos-ans/sib/sib-algoritmos-xml10-17012019.pdf); the provisional branch is **unverified** |
| Ireland | IHI | Health Identifiers Act 2014; one secondary source gives a GS1 form with two check digits and conflicts with itself on length (16 or 18) | [HSE IHI page](https://www.hse.ie/eng/about/who/national-services/individual-health-identifier); format **unverified** |
| Germany | KVNR | One letter and nine digits with a check digit; the algorithm was not found in a primary source | **unverified** |
| India | ABHA | 14 digits, shown 2-4-4-4; no official page found by search | **unverified** |

The existing NHS-number rule (ten digits, modulus 11, repeated digits are placeholders) already shows the pack's shape:
system URI, length, check rule, placeholder rule, and whether a shared value decides or only supports a link.

Facts for the **name pack** (rows 2 and 3):

- FHIR R4 defines `humanname-fathers-family` and `humanname-mothers-family` as extensions on `HumanName.family`, used
  by the Czech Core and the HL7 Europe laboratory profiles for Spanish-style double surnames
  ([HumanName extensions](https://www.hl7.org/fhir/R4/humanname-extensions.html)). The matcher's single `family`
  string can hold both names; a pack would say how to compare the parts. No source gives matching rules.
- The W3C guidance lists family-name-first orders (China, Japan, Korea, Hungary), patronymics with no family name
  (Iceland), `bin`/`binti` (Malaysia, Indonesia), mononyms, two and more family names (Spanish-speaking countries,
  Brazil), and advises not to require a family name and not to label fields "first" and "last"
  ([W3C, Personal names around the world](https://www.w3.org/International/questions/qa-personal-names)).

Facts for **phone** (row 4) and **address** (row 5):

- The national (trunk) prefix is usually, not always, `0`; Italy keeps its leading `0` as part of the number, so
  stripping it is wrong there ([libphonenumber FAQ](https://sources.debian.org/src/libphonenumber/8.13.51%2Bds-5/FAQ.md)).
  A pack entry needs the calling code and the trunk-prefix rule, not the code alone.
- Around forty countries use no postcode and the United Arab Emirates is one; the figure is a secondary reading of the
  Universal Postal Union list ([GeoPostcodes](https://www.geopostcodes.com/blog/countries-without-zip-codes/)), **unverified** against the UPU itself.

Facts for the **regulatory pack** (row 11), names only, nothing summarised as a rule:

- Brazil LGPD treats health data as sensitive; Canada has PIPEDA and provincial health acts (Ontario PHIPA 2004); India's
  DPDP Act 2023 has no sensitive-data category; Australia's Privacy Act 1988 has health-specific principles (secondary
  sources only, [one overview](https://caseiq.com/resources/a-practical-guide-to-data-privacy-laws-by-country)).
- The European Health Data Space, Regulation (EU) 2025/327, entered into force on 26 March 2025, applies from 26 March
  2027, and the first exchange wave (patient summary, ePrescription) starts in March 2029 on an IPS base over FHIR R4
  ([European Commission](https://health.ec.europa.eu/ehealth-digital-health-and-care/european-health-data-space-regulation-ehds_en)); a secondary post gives 2027 for the same wave, so the date is **[uncertain]**.

**Not done / not verified:** no code changed; the rows marked unverified need a primary source before a pack is written;
the ANS file was found by search and not read line by line; nothing here is a legal summary.
