# Models

- **`Patient`** (matcher crate): the thin identity record: names, birth date (possibly partial), administrative sex,
  identifiers, addresses (postcode only for matching), telecom. It is the API identity DTO and is persisted as JSONB.
- **Clinical resources** (planned) are FHIR R4 JSON stored append-only in `resource_version`, one new row per change,
  with a Provenance. "Delete" is `entered-in-error`. See entity spec §5 and `plan.md` §6.2.
- **Absent is not zero.** Use `Option`/`null` with a reason; FHIR `dataAbsentReason` on the wire.
- Times are UTC with the recorded zone kept; render in the user's zone at the edge.
- People and organisations are references (`Practitioner`, `Organization`), not copied strings.
