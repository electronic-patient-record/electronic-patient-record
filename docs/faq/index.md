# Frequently asked questions

**Use only invented patient data.** Electronic Patient Record is a pre-release prototype for evaluating how a record works. For your own results, appointments or corrections, use the service provided by your care team. [Project status](../../spec/14-implementation-status.md)

## Trying the project

### Can our practice use this with patients?

No. The project has working features for identity, allergies, observations and referrals, but it has no clinical validation or approval for use in care. The production checklist describes the evidence a named site would need, including clinical safety review, a demonstrated backup restore and a support team responsible for incidents. [Implementation status](../../spec/14-implementation-status.md) · [Path to production](../../governance/production-readiness.md)

### How can I try it, and which patient should I look up first?

Start the local development service and set up a test account using the Keycloak guide. Load the supplied sandbox dataset into that development instance, then search for the family name `SandboxAlder` on the **Patients** page. The dataset contains 24 invented people with sample observations and allergies. With sign-in enabled, the loader needs a clinician's development token. [Setup guide](../../README.md#run-the-api-from-a-fresh-clone) · [Development sign-in](../../operations/keycloak/README.md) · [Sandbox instructions](../../operations/sandbox/README.md)

### Can I use it on a phone or when the connection drops?

Use the web interface for a signed-in session on a phone. The native iOS and Android apps currently show a sample patient banner. On the web patient page and **My record**, a failed refresh leaves the last successful read on screen and warns that it may be out of date. Offline record access and synchronization are still planned. [Mobile apps](../../spec/mobile-apps/index.md) · [Web interface](../../electronic-patient-record-ui-with-svelte/spec/index.md)

### Which languages and accessibility features can I try?

English is the source language. Welsh covers the main interface text. Other language choices fall back to English for most screens. Translations are machine-written and await speaker review. The web interface includes labelled fields, skip links and alerts, with automated accessibility checks. Hands-on keyboard and screen-reader verification is still a production-readiness item. [Languages](../../spec/locales.md) · [Accessibility evidence](../../governance/production-readiness.md)

## Patients, families and carers

### I signed in. Why is “My record” empty?

**My record** lists only the test records linked to the signed-in account. If it shows no link, ask the person running the sandbox to check which patient or proxy relationship they recorded for that account. Signing in alone grants no access to a record. [My record and care relationships](../../electronic-patient-record-ui-with-svelte/spec/index.md)

### Can a parent or carer see someone else's record?

Yes, within the permissions recorded for that proxy in the prototype. An authorized user records the basis for access, what the proxy may see and when access ends. Identity details, allergies, the photo and the access log have separate permissions. A proxy cannot grant themselves access. Ending the relationship revokes it. [Proxy authority](../../electronic-patient-record-service-with-rust/spec/index.md)

### Can I see who looked at my record or download the history?

On **My record**, open **Who has accessed this record**. The log shows the time, purpose and outcome of recorded access. It identifies staff by their recorded role and organization. You can filter by date, person category, outcome or purpose. The CSV download includes up to the 1,000 most recent entries matching the applied filters. If the page says it cut the list short, narrow the date range. [Access log and CSV download](../../electronic-patient-record-ui-with-svelte/spec/index.md)

### What if an allergy or a measurement is wrong?

On your own test record, open **My record** and find **Ask for a record to be corrected**. Select the allergy or vital sign, describe the error and submit the request. You can follow its progress there. A clinician records a decision and a reason. An applied correction creates a new version and preserves the original. If the request is refused, you can add a statement of disagreement. [Patient correction form](../../electronic-patient-record-ui-with-svelte/src/lib/PatientAmendments.svelte) · [Amendment requests](../../electronic-patient-record-service-with-rust/spec/index.md)

### Where are my test results, messages and appointments?

The prototype lets authorized staff record results and sign off a specific version. Patients and proxies cannot view results because release rules have yet to be implemented. The patient view also has no messaging, appointment booking or bill payment. The design assigns booking to a separate patient administration system, and billing is outside the scope of version 1.0. [Patient portal scope and current features](../../spec/patient-portal/index.md)

## Registration and clinical staff

### How do I check whether a test patient already has a record?

On **Patients**, search by the start of the family name, date of birth or national health number. If registration finds a possible duplicate, it shows the candidate alongside the details you entered and explains the match. Review those details before deciding. Registering a separate person despite that warning requires a written reason. [Patient search and registration](../../electronic-patient-record-ui-with-svelte/spec/index.md)

### Why can a comparison score 1.0 and still ask for human review?

A shared national health number that passes the format checks can produce a score of 1.0. If either record lacks a given name, family name or birth date, the matcher still asks for review and identifies the missing evidence. For example, two synthetic records with the same valid number and one missing birth date get this result. Names and dates of birth alone also cannot trigger an automatic link. [Matcher rules](../../electronic-patient-record-matcher-with-rust/spec/index.md#4-deterministic-identifiers)

### Can we undo a mistaken merge?

A clinical administrator can open the surviving record, choose **Undo this merge** for the relevant record and give a reason. The service retains both original records and a history of the link. A merged-away record points to the survivor so staff can see where it went. [Merge controls](../../electronic-patient-record-ui-with-svelte/spec/index.md) · [Merge permissions and history](../../electronic-patient-record-service-with-rust/spec/index.md)

### What is the difference between “not recorded” and “no known allergies”?

“Not recorded” means the system has no allergy information to show. “No known allergies” requires an explicit clinician statement and carries its time. If access was refused or the allergy lookup failed, the banner says so. Neither outcome establishes the person's allergy status. [Patient banner and allergy states](../../electronic-patient-record-ui-with-svelte/spec/index.md)

### What happens if someone edits the patient details while I have the page open?

The page checks for a newer version. If you have unsaved edits, it preserves your typing and warns that the record changed. Saving against the old version is refused. Check the newer details before reapplying your edit. Choosing **Reload and discard my changes** discards your draft. [Patient banner refresh and version checks](../../electronic-patient-record-ui-with-svelte/spec/index.md)

### Why can I sign in but still be refused access to a test record?

Signing in identifies you. The service then checks your role, the action, your relationship to the patient, your declared purpose and applicable consent rules. Selecting **Treatment** alone does not grant access. For a development account, check the assigned role and care relationship with the sandbox administrator. [Development role setup](../../operations/keycloak/README.md) · [Access policy](../../electronic-patient-record-service-with-rust/spec/index.md)

## Integrators and contributors

### Can we connect our existing system using FHIR or HL7 v2?

For a sandbox integration, start with `GET /fhir/R4/metadata`. Its CapabilityStatement lists the FHIR R4 resources and operations the service exposes. For example, it supports Patient search, and updating a Patient requires the version you read. There is also a separate HL7 v2 feed for admission, discharge and transfer messages. A connection to a particular hospital system still needs testing with that partner. [API surface](../../spec/09-api-surface.md) · [Service interfaces](../../electronic-patient-record-service-with-rust/spec/index.md) · [Partner testing](../../governance/production-readiness.md)

### Can we self-host and adapt the software?

Yes, for evaluation with synthetic data. The repository supplies a Podman setup for the service, database and cache. You can choose any one of the licences listed by the project, or contact the copyright holder about custom terms. [Local setup](../../README.md#run-the-api-from-a-fresh-clone) · [Licence choices](../../LICENSE.md)

### Where do I report a confusing screen, an accessibility issue or a privacy problem?

For an ordinary bug or confusing screen, open a project issue naming the page, what you expected and what happened. Use invented details. Report a suspected vulnerability or exposure of personal information privately through the security policy's email address or private GitHub reporting link. Keep personal data out of reports and attachments. [Contributing](../../CONTRIBUTING.md) · [Private security reporting](../../SECURITY.md)

## Where the questions came from

These are anticipated questions. The examples below informed the choice of topics. The answers above describe this project's own implementation.

| Example | Question it helped identify |
| --- | --- |
| [KenyaEMR navigation guide, Kenya (PDF)](https://kenyahmis.org/wp-content/uploads/2024/04/SOP-KenyaEMR-3.x-Navigation-July-2023-Final-1.pdf) | How does a registration clerk find a returning patient before creating another record? |
| [Bahmni implementation FAQ, India](https://www.bahmni.org/faq) | What does a hospital need to try, host and support the software itself? |
| [MyKanta record information](https://www.kanta.fi/en/what-information-does-mykanta-show) and [guardian access, Finland](https://www.kanta.fi/en/as-a-guardian-of-a-minor) | Where do I take a correction, and what can I see when acting for a child? |
| [My Health Record access history, Australia](https://www.digitalhealth.gov.au/initiatives-and-programs/my-health-record/manage-your-record/privacy-and-access/access-history) | Can a patient tell when someone accessed the record and in what capacity? |
| [My HealtheVet secure messaging, United States](https://www.va.gov/health-care/send-receive-messages/) | Can I contact my care team about a result or appointment through the record? |
