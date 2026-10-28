# Record of processing activities (DRAFT, UK GDPR Article 30)

The controller, its representative and the data protection officer are **[controller to decide]**. This record is a starting point from the code. It has not been reviewed.

| Activity | Purpose | Data subjects | Categories of data | Recipients | Retention | Security |
| --- | --- | --- | --- | --- | --- | --- |
| Registering and identifying a patient | Know who the record is about | Patients | Names, birth date, sex, identifiers, addresses, telecom, photo | Staff with a care relationship; the patient | `record`: not set | Policy table, consent, audit |
| Recording allergies and vital signs | Care | Patients | Health data | Staff with a care relationship and consent; the patient | `record`: not set | As above; versions are append-only |
| Consent and relationships | Say who may see and act | Patients; proxies | Consent decisions; who stands in what relation | Staff; the patient | `record`: not set | As above |
| Requests to correct a record | Rights of rectification | Patients | The patient's words; the decision and its ground | Clinicians deciding; the patient | `record`: not set | Requester never decides |
| Matching and merging patients | Avoid duplicates and wrong-patient errors | Patients | Identity fields; match scores | Staff | `record`: not set | Merges are versions, not deletions |
| Emergency access | Care when consent or relationship is missing | Patients; staff | Who, whom, why, how long | Reviewers | `audit`: not set | Reason required; every grant reviewed |
| Audit | Accountability | Patients; staff | Who did what to whose record, when, from which address | Reviewers; the patient (own log) | `audit`: not set | Append-only, hash-chained |
| Receiving HL7 messages | Keep identity in step with a feeder system | Patients | The raw message when refused | Staff resolving refusals | `feed`: not set | MLLP listener has no authentication or encryption; off by default |
| FHIR facade | Exchange with other systems | Patients | Patient, Observation and AllergyIntolerance resources | Authorised clients | Not applicable | Same policy gate as the REST API |

Processors and sub-processors (hosting, identity provider, any carrier): **[controller to decide]**. International transfers: none are made by the software itself; hosting is the deployer's choice.
