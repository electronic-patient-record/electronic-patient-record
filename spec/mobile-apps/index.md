# Native mobile apps (iOS and Android)

**A direction from the author's instruction of 2026-11-23 (plan.md §39). Only a scaffold is built, and only on branch `mobile-apps`.**
Two new subprojects:

| Subproject | Platform | Stack | Guide |
| --- | --- | --- | --- |
| `electronic-patient-record-mobile-app-with-swift` | iOS and iPadOS | Swift 6, SwiftUI, Lily Design System for Swift (`LilyUI`, `LilyCore`) | [AGENTS.md](../../electronic-patient-record-mobile-app-with-swift/AGENTS.md) |
| `electronic-patient-record-mobile-app-with-kotlin` | Android | Kotlin, Jetpack Compose, Lily Design System for Kotlin (`lily-compose`, `lily-core`) | [AGENTS.md](../../electronic-patient-record-mobile-app-with-kotlin/AGENTS.md) |

This page is the contract the two apps share. Each app's own `spec/index.md` says how it meets the contract and what it
has built. Where an app's spec and this page disagree, this page wins; where this page and the entity spec
([`../index.md`](../index.md)) disagree, the entity spec wins.

## 1. Why native apps, and what they do not replace

[Mobile viewability](../mobile-viewability.md) (XP-1 to XP-4) stays the first answer: the web UI must work on a phone, and
it is the only client with sign-in, the patient record and the access log today. The native apps exist for what a browser
does badly on a phone: a device the trust issues and can wipe, unlock by the platform's biometrics, the camera (a wound
photo) without an upload page, a system screen reader and text size honoured by the platform's own controls, and, only
once OQ-5 is answered, reading while out of signal. They are **clients of the same service**: no clinical rule lives in
an app that is not also in the service, and nothing an app shows may differ in meaning from what the web UI shows for the
same record.

## 2. Architecture

```
iOS app (SwiftUI)  ─┐
                    ├──HTTPS──►  Service /api (Axum, axa)  ──►  PostgreSQL 18
Android app (Compose)┘
```

- **Calls the service directly.** A native app is an OAuth 2.0 public client and needs no BFF. It calls the
  unversioned `/api` path and sends the API version in the `Accepts-version` header, never as a path segment
  ([`09-api-surface.md`](../09-api-surface.md)). A path that does not start with `/api/`, or has an empty, `.`, `..`,
  `%` or backslash segment, is refused before any request is made: the same rule as the UI's proxy.
- **Base URL.** `https://` only; plain `http://` only for a loopback host (`127.0.0.1`, `::1`, `localhost`) in a debug build.
  No user-entered base URL in a release build.
- **Sign-in (MP-5).** Authorization code flow with PKCE (S256), `state` and `nonce`, through the system browser
  (`ASWebAuthenticationSession`; Android Custom Tabs), never an embedded web view, as RFC 8252 (OAuth 2.0 for native
  apps) recommends. New Keycloak clients `epr-ios` and `epr-android` (public, PKCE required). The redirect is a claimed
  `https` link (Universal Link, verified App Link) where the deployment can host the association file, otherwise a
  private-use scheme `org.electronicpatientrecord.app:/oauth2redirect`; which one is OQ-MP2.
- **Tokens.** The access token lives in memory only. The refresh token, if the realm issues one to these clients, is
  kept in the Keychain (`kSecAttrAccessibleWhenUnlockedThisDeviceOnly`, no iCloud sync) or encrypted with an Android
  Keystore key that cannot leave the device. A token never goes to `UserDefaults`, `SharedPreferences`, a file, a log
  line, a crash report or an analytics event, except the allow-listed navigation events of §12 rule 1 once a DPIA covers them.
- **Layers.** Each app has a **core** with no UI and no platform networking (Swift package target `EPRCore`, Kotlin JVM
  module `:core`), unit-tested on the build machine, and an **app** layer (SwiftUI views; Compose screens) that renders
  what the core decides. A rule that can be computed (request building, display text for an absent value, idle limits)
  lives in the core with a test.

## 3. Rules shared with the web UI

The same rules as [the UI's spec](../../electronic-patient-record-ui-with-svelte/spec/index.md), restated for a phone:

1. **Absent is not zero.** A missing value is shown as "not available", never `0`, "none" or an empty cell.
2. **The patient banner never truncates identity.** Name, date of birth and the identifier stay whole on the smallest
   supported screen and at the largest text size: they wrap, never end in an ellipsis. A wrong-patient error is H-001
   and a small screen raises it (H-061).
3. **Safety information is not the first thing dropped.** Allergies, alerts and unreviewed results are never behind a
   "more" control without a visible count ([mobile viewability](../mobile-viewability.md) requirement 4).
4. **API error text.** Only a 422 list of problems (`errors`) may be shown; every other status shows the app's own words.
5. **Nothing from the record is kept on the device beyond the session.** No response cache on disk (`URLCache` with
   zero disk capacity on iOS; no HTTP cache on Android), no record data in a notification, a widget, Spotlight or the Android
   recents thumbnail; the app-switcher snapshot is blanked (iOS: a cover view on `scenePhase` inactive; Android:
   `FLAG_SECURE`). Screenshots are blocked on Android by the same flag; iOS cannot block them, which is recorded in H-061.
6. **Idle lock.** After the service's idle limit (900 s by default, as the UI's `EPR_SESSION_IDLE_SECONDS`) with no
   interaction the app locks and asks for the platform's biometrics or passcode; after the absolute limit (28800 s) it
   signs out. A background poll does not count as interaction (hazard H-025).
7. **Words.** The app's words are a typed table whose keys match the UI's `STRINGS` where the meaning is the same; the
   ten `LOCALES` of [locales](../locales.md). Translations are machine-written until a speaker reviews them, and no app
   may say a translation is reviewed.
8. **Synthetic data only** in previews, tests and fixtures.
9. **Not for clinical use** is shown on the first screen of every build until the project says otherwise.

## 4. Accessibility

- Every screen is usable with VoiceOver and TalkBack, Switch Control and Switch Access, and keyboard focus on a tablet.
- Text follows Dynamic Type and Android font scale, up to the largest accessibility size, with no clipped text.
- Touch targets at least 44 by 44 points (Apple Human Interface Guidelines) and 48 by 48 dp (Android accessibility
  guidance); both exceed WCAG 2.2 criterion 2.5.8 (24 CSS px).
- Components come from Lily Design System where Lily's catalogue gives the platform decision `lily`; where it says
  `native`, the platform control is used. Lily carries the accessibility contracts; the app does not re-implement them.
- Tests: every screen has a test that reads its accessibility tree (XCUITest accessibility queries on
  iOS; the Compose semantics tree under Robolectric on Android).

## 5. Lily Design System dependency

Neither Lily package is published yet (checked 2026-11-23: the SwiftPM export repository
`LilyDesignSystem/lily-design-system-swift` answered "Repository not found", and Maven Central has no
`io.github.lilydesignsystem:lily-compose`). Until one is, both apps build against **one pinned commit of the Lily monorepo**
(`LILY_COMMIT` in `scripts/fetch-lily.sh`), cloned into `vendor/lily-design-system/` (ignored by git):

- Swift: `.package(path: "../vendor/lily-design-system/lily-design-system-swift")`.
- Kotlin: `includeBuild("../vendor/lily-design-system/lily-design-system-kotlin")`, with Gradle substituting
  `io.github.lilydesignsystem:lily-compose` and `lily-core` by the included projects.

When Lily publishes, the path is replaced by a versioned dependency (MP-3) and the script goes. Moving the pin is a
commit of its own that says what changed in Lily.

## 6. Platforms

- iOS and iPadOS 18 or later (Lily's minimum). Swift 6 language mode with strict concurrency.
- Android 9 (API 28) or later: `BiometricPrompt` and hardware-backed Keystore keys are there from 28. Target and compile
  SDK follow Lily's (37).
- Phones and tablets, portrait and landscape. No watch, TV or desktop target.

### Device ownership (decided by the maintainer, 2026-11-23; OQ-MP1)

Both apps MUST support **managed devices** (issued and administered by the trust, under MDM) **and bring-your-own
devices** (owned by the clinician), each on **phones and tablets** (iPhone and iPad; Android phones and tablets). One
build serves all four cases; an app never refuses to run because a device is unmanaged.

1. **The app carries its own controls.** Nothing in the contract may rely on an MDM being present. What the app must do
   by itself, on every device, is in §2 and §3 and in rules 2 to 5 here. A managed device adds to these; it never
   replaces one.
2. **A device lock is required.** The app does not start a session unless the device has a passcode or biometric lock
   (iOS: `LAContext` policy `deviceOwnerAuthentication` can be evaluated; Android: `KeyguardManager.isDeviceSecure`). If
   the lock is removed while the app holds a refresh token, the next launch signs out and wipes.
3. **An app-level wipe.** Signing out, an idle sign-out (§3 rule 6) and a refused refresh (the realm answers
   `invalid_grant`, or a 401 follows a refresh) each delete the refresh token and the Keystore key that wraps it, and
   drop every in-memory record. This is the control for a lost personal phone: the trust revokes the user's sessions
   in Keycloak, and the app wipes itself the next time it reaches the service. It does not help a phone that never
   connects again, which is why the idle limit and the realm's refresh-token lifetime stay short.
4. **Nothing leaves the app's sandbox.** No record data in the share sheet, a notification, a widget, Spotlight, the
   Android recents thumbnail, a backup (iOS: excluded from iCloud and device backup; Android: `allowBackup="false"` and
   `dataExtractionRules` excluding all domains) or a file the user can reach.
   **Record text is never copied onto a personal device** (decided by the maintainer, 2026-11-23). The app cannot tell a
   personal device from a managed one with certainty (rule 5), so the rule holds on every device and every build:
   - no copy, cut, "look up", "translate", "share" or drag of record text: record text is drawn so the platform's text
     selection is off (iOS: not selectable, no `UIPasteboard` write, `NSItemProvider` and drag interactions off; Android:
     no `SelectionContainer`, no `ClipboardManager` write, `View.setLongClickable(false)` on record views);
   - no export, print or save of record text or a record photograph to Files, Photos, the gallery, mail or another app;
   - the app's own text fields (patient search) are not a way in: autocorrect, predictive text and learning are off for them
     (iOS: `autocorrectionDisabled`, `textContentType` none, `keyboardType` `.asciiCapable` or `.default` with no
     suggestions; Android: `textNoSuggestions`, `IME_FLAG_NO_PERSONALIZED_LEARNING`), and the field is cleared when
     the app locks;
   - a pasted value is accepted only into a search field, and never written back out.
   Not preventable by the app, and stated so: a **screenshot or screen recording on iOS**, a **photograph of the
   screen**, and a clinician **retyping** what they read. The rule closes what the app does; H-061 keeps the
   rest as residual. Android's `FLAG_SECURE` blocks screenshots and recordings there (§3 rule 5).
5. **Jailbreak and root detection is advice, not a control.** An app cannot reliably detect a compromised device. It may
   warn on the signs it can see; it never counts the absence of a sign as proof the device is sound.
6. **Managed configuration is optional and can only tighten.** On a managed device the app reads the platform's managed
   app configuration (iOS `com.apple.configuration.managed`; Android Enterprise managed configurations) for the service
   base URL and a shorter idle limit. A managed value may shorten the idle limit and the absolute limit, never lengthen
   them past §3 rule 6, and a managed base URL still has to be `https://` (§2).
7. **Tablets.** Layouts adapt to the width; the banner rules (§3 rule 2) and the touch-target sizes (§4) hold at every
   size and at split-screen and slide-over widths. A tablet is a personal device of one user in the same way as a
   phone: a tablet shared between clinicians is not supported (one user per install, no fast user switch).
8. **No real record data on a bring-your-own device until rules 2 to 4 are built and tested on a real device.** Until
   then both apps show synthetic data only, on every device (§3 rule 8). Which kinds of device may hold which kind of
   record (for example, no photographs on a personal phone) is open and goes with the clinical safety case.
9. **Distribution (OQ-MP3) is separate.** Managed devices can take managed distribution; a bring-your-own device usually
   takes the public stores, which is where OQ-MP3 and the fake-app risk are decided.

## 7. Built (scaffold, M0)

Each app, the same in both:

- **Core:** `ApiRequest` builds a request for a path under `/api` with the `Accepts-version` header and refuses any other
  path or a non-HTTPS non-loopback base URL; `PatientSearchResults` and `Patient` decode the service's
  `GET /api/patients` answer (fields as the matcher's `Patient`: `names`, `birth_date`, `identifiers`, …, every field
  optional or defaulted, as the service may omit them); `Banner` gives the patient banner's display text, with
  "not available" for each absent part; `Strings` holds the English words the scaffold uses.
- **App:** one screen with the product name, the not-for-clinical-use notice and a sample banner on a synthetic patient,
  built from Lily components.

Not built: sign-in, any network call, any record screen, the idle lock, the snapshot cover, translations, a release
build, signing, distribution. Nothing has run on a real device.

## 8. Tests and gates

| Gate | Swift | Kotlin |
| --- | --- | --- |
| `mobile-swift-test` (`--fast`) | `swift test` in the subproject (the core and the views, on macOS) | |
| `mobile-swift-build` (`--full`) | `xcodebuild build` for the iOS Simulator, warnings as errors | |
| `mobile-kotlin-test` (`--fast`) | | `./gradlew :core:test :app:testDebugUnitTest` (core, and Compose semantics under Robolectric) |
| `mobile-kotlin-build` (`--full`) | | `./gradlew :app:assembleDebug`, Kotlin warnings as errors |

Each is `sh scripts/gates/<gate>.sh`, which fetches the pinned Lily checkout first, run by `scripts/ci-local.sh` and by
GitHub CI (MP-4). The Swift gates need macOS and Xcode: CI runs them on `macos-latest`, only when the iOS app, its
gates or the Lily pin change. The Kotlin gates need a JDK 17 or later and the Android SDK: CI runs them on
`ubuntu-latest` with Temurin 25, only when the Android app, its gates or the Lily pin change. Dependabot does not watch
the Gradle build: its versions follow Lily's at the pinned commit, and a bump on its own would split the two builds.

## 9. Open questions

| ID | Question | Recommendation |
| --- | --- | --- |
| OQ-MP1 | Who owns the devices: the trust (managed, MDM) or the clinician (bring your own)? It decides remote wipe, distribution and whether a lost-device threat is controlled by the trust | **Answered 2026-11-23 by the maintainer: both, on phones and tablets** (§6, Device ownership). Decided with it, 2026-11-23: record text is never copied onto a personal device (§6 rule 4). Still open: which record kinds a personal device may hold |
| OQ-MP2 | Claimed `https` redirect or a private-use scheme for sign-in? | Claimed `https` where the deployment can host `apple-app-site-association` and `assetlinks.json` |
| OQ-MP3 | Distribution: the App Store and Google Play, Apple Business Manager and managed Google Play, or both? | Managed distribution; public stores only after a clinical safety case |
| OQ-MP4 | Offline reading of the downtime snapshot on a phone (links OQ-5, OQ-MV1)? | Not until OQ-5 is answered; then read-only, encrypted, wiped at sign-out |
| OQ-MP5 | Is a native app needed for patients (the patient portal) as well as for staff? | **Answered 2026-11-23 by the maintainer's instruction: yes, with the four feature groups of §11.** Open inside it: one app with two audiences or two apps (OQ-MP6) |
| OQ-MP6 | One app that serves staff and patients (a role decides the screens), or a separate patient app? | Separate: a patient's sign-in, data, words and risks differ from a clinician's, and a staff build must not carry the patient features or the reverse |
| OQ-MP8 | Is a third-party PROMs provider (Promptly Health or another) allowed to receive health data, and under what contract and hosting? | Not until a DPIA and a processor contract exist; the app talks only to the service, never to the provider directly |
| OQ-MP9 | What identity assurance does authorised access need, and who verifies a requester (the practice, a national service, both)? | The practice or the national identity service verifies, never the app; trial only with a named group |
| OQ-MP10 | What exactly does the WCP desktop show as warnings and adverse reactions, and which of it must the phone show? | A list from the deployment's clinical lead, kinds and fields; the phone shows no fewer kinds than the desktop for safety information |
| OQ-MP11 | Who may a note or task be shared with, and may a phone share beyond the care team? | The care team and named roles the service's policy allows; no sharing outside the organisation |
| OQ-MP12 | Which result kinds may be signed off on a phone? | The same as the web at first; the Clinical Safety Officer may exclude kinds (for example critical or imaging reports) |
| OQ-MP7 | Which national services does the deployment link to or write to (NHS 111, an organ donation register, a blood service)? | A deployment pack names them; the app carries none by default (equity rule, [scope](../02-scope.md)) |

## 11. Patient features (the maintainer's instruction of 2026-11-23)

The native apps also serve **patients** (and proxies acting for them). The feature list is the one a national patient app
offers; each group below says what the service has today, what it has not, and the rules that apply. Nothing in this
section is built, and the names of national services are one country's examples, not the product's (a deployment pack
names its own, OQ-MP7).

| Group | What the patient does | The service today | Owner of the data |
| --- | --- | --- | --- |
| **Prescriptions** | Order and manage **repeat** prescriptions | **Nothing**: no medicines, medication request or prescription route exists | The prescribing system (not built here) |
| **Appointments and referrals** | Book, view or cancel a GP appointment; check a waiting-list referral or a hospital appointment | Referrals are built (`/api/patients/{id}/referrals`, the worklist); **no appointment route exists**; waiting lists are display-only (PA-15) | A PAS or the booking system (PA-1); the referral is this record's |
| **Health records** | View parts of the GP record: **test results, allergies, medicines**; receive **messages** from GP practices and hospitals | Results and allergies are built (staff routes); **no medicines list, no message route** | This record, under the portal's release rules ([patient portal](../patient-portal/index.md) R-RES) |
| **Services and preferences** | NHS 111 guidance; record an **organ donation decision**; register to give blood | **Nothing** | National services (OQ-MP7) |

### Rules

1. **A patient is not a clinician.** A patient signs in to a patient client (`epr-patient-ios`, `epr-patient-android`, not
   the staff clients), reads under the purpose `patient-access`, and sees only the records linked to the signed-in person,
   or a proxy's in-scope ones (patient portal PA-5). Every view writes an audit row as an access by the patient or proxy
   (R-RES-8). The staff routes are not called by a patient build, and a staff build does not carry these screens.
2. **Release rules decide what appears.** A result shows only when the deployment's release rule has released it
   (R-RES-1 to R-RES-8); a result no one has reviewed says "A clinician has not reviewed this result yet", and no screen
   reads an unflagged result as normal (R-RES-4). A missing reference range says so; an absent value is "not available",
   never `0` (§3 rule 1).
3. **Medicines and allergies are not complete unless the record says so.** A medicines list shows its date and its source,
   and an empty list says "No medicines are recorded", never "You take no medicines"; the allergy screen shows the
   allergy statement (`/allergy-statement`) and never reads silence as "no allergies" (§3 rule 1, H-002 family).
4. **Repeat prescriptions are a request, not an order.** The app **asks**; a prescriber or the practice approves. The
   screen lists only medicines the record marks as repeat, shows what was last issued and when the next request is
   allowed, and says plainly that the request is not yet approved and how long it usually takes **[deployer to
   decide]**. It cannot request a medicine the patient types in, a dose change, or a controlled drug. A request that fails or times out is
   never shown as sent. Duplicate taps do not send two requests (an idempotency key, as the service's writes use).
5. **Booking and cancelling go through the booking system.** The portal owns clinical content; the PAS owns booking (PA-1).
   Cancelling an appointment needs a confirmation that names the date, time, place and clinician, and an appointment
   that cannot be cancelled online (too close, or a kind the pack excludes) says so and gives the practice's way. A
   waiting-list screen shows a referral's state and dates as the record has them; it never estimates a wait the record
   does not hold, and "no date yet" is not zero days.
6. **Messages.** A message from a practice or hospital shows its sender, its date and whether it was read, and the app
   shows no message text in a notification (§3 rule 5): the notification says only that there is something new. A
   message that asks for urgent action is shown with the service's own words on what to do if worried, and the app
   never claims a message was seen by the practice when it was not.
7. **Organ donation and blood donation are not the record's.** The app **links to or writes to** the national register or
   service the deployment names (OQ-MP7); it never keeps the decision as the authoritative copy, never shows a stale one
   as current, and says when the register could not be reached. A decision that could not be sent is not shown as
   recorded. Guidance such as NHS 111 is a link to the national service with the app's own words around it; the app does
   not give clinical advice of its own.
8. **Emergencies.** Every patient screen that shows symptoms, results or a message carries a fixed line on what to do in
   an emergency (the deployment's number), outside the record's text and never behind a menu.
9. **The never-copy rule holds for patients too.** Record text, including a patient's own results, is never copied onto the
   device (§6 rule 4). A patient who wants a copy asks the web portal's record download (patient portal), which
   has its own audit and release rules. This is stricter than a national patient app, and the maintainer decided it
   for every device; it is noted here because it will surprise a patient.
10. **A patient's phone is a personal device.** Device ownership rules 2 to 4 and 8 of §6 apply, so no real patient
    reaches these screens until they are built and tested; proxies and children (adolescent-confidential items, R-PRX)
    follow the portal's proxy rules, not an app-only rule.
11. **Words and access.** The patient apps need the ten locales (§3 rule 7) and the plain-language standard of the web
    portal; accessibility (§4) matters more here: a patient may be elderly, visually impaired or unwell.

### Order

Read before write, and the least risky first: **health records, read-only** (results, allergies; medicines and messages
once their routes exist) → **referral and appointment viewing** → **services and preferences** (links) → **cancel an
appointment** → **repeat prescription requests** (a write with clinical effect, last). Each needs a service route that
does not exist today (the table above) and its spec in the patient portal first; the apps do not invent a route.

## 12. Further patient features (feedback of 2026-11-23)

Twelve more items for the patient apps, from the maintainer's feedback. Each is assessed here for what it needs and what
could go wrong; none is built, and several name services this repository does not have. Items the feedback calls
"exploring" or "feasibility" are tasks to find out, not to build. Third-party and national names (Promptly Health, a
prescription barcode scheme, a pharmacy directory, a vaccination register) are one deployment's examples (OQ-MP7).

| # | Item | Assessment | Task |
| --- | --- | --- | --- |
| 1 | **PROMs** (patient-reported outcome measures) through an integration with Promptly Health | A **third-party processor** receives and returns health data: it needs a DPIA, a processor contract and a data-flow entry before any code. The service already stores completed assessments (WL-3, `assessments_store`), the natural home for a returned score. Who is **eligible** is decided by the service (a clinician or a rule), never by the app. A score is not a diagnosis | MP-17 |
| 2 | **Monitoring** of how users navigate the app | Conflicts with §2 ("no analytics event"): that rule changes to "no event except the allow-listed navigation events of §12 rule 1". Highest privacy risk of the twelve if done carelessly; see §12 rule 1 | MP-18 |
| 3 | **Onboarding** that says what the app can do | Low risk, high value. It must describe only what is switched on for this deployment and this user (a clinician build, a patient with no proxy), or it promises a feature that is not there | MP-19 |
| 4 | **Notifications** for hospital appointments and waiting lists | Apple and Google carry every push: the payload and its metadata are seen by them. Content-free by §3 rule 5 and §11 rule 6; needs the communications preferences (item 12) and a PAS source (PA-1) | MP-20 |
| 5 | **Vaccination appointments**: show, and notify | Needs a source (an immunisation register or the PAS) that this repository has not got; **explore first**. An appointment is not a vaccination record: the app shows what was booked, never "you are protected" | MP-21 |
| 6 | **A clearer test results screen** | Safety-critical presentation: the layout may not make a result look more normal, or more alarming, than the record says. Refines MP-13 and R-RES-4; test it with patients and a clinician | MP-22 |
| 7 | **More results: hospital-requested (secondary care)** | A hospital's result has a different requester and reviewer from the GP's; "a clinician has not reviewed this" is **whose** review? The results store already merges by provenance; the patient view must say who requested it and which organisation, and the release rule is per source | MP-23 |
| 8 | **Authorised access**: request to manage someone else's care, with an improved authorisation process (a trial) | The most abusable item: a person who should not have access (coercive control, a family dispute) is the threat. Needs verified identity of both people, the subject's own consent or a lawful basis for those who cannot give it, a scope, an end date, adolescent-confidential items, and a way for the subject to see and end it. The service has proxy terms (`relationships`); the request flow is new. A trial needs its own safety case | MP-24 |
| 9 | **Nominate or change a preferred pharmacy** | A write with clinical effect: a prescription goes to the nominated pharmacy, so a wrong or stale choice sends medicine to the wrong place. Needs a pharmacy directory (national service) and a route that tells the prescribing system | MP-25 |
| 10 | **A prescription barcode** to collect from a pharmacy | A barcode is a **bearer credential**: anyone who scans it can collect. It is shown on demand after unlock, drawn in memory, never stored, shared or copied (§6 rule 4), hidden after a short time, and Android keeps `FLAG_SECURE` on it. The iOS screenshot gap (H-061) matters more here. Needs MP-16 first | MP-26 |
| 11 | **Track a prescription's status** | A status comes from other systems and goes stale. Unknown is "status not available", never "being prepared"; every status shows when it was last updated; no estimate of when it will be ready. Needs MP-16 first | MP-27 |
| 12 | **Personal details shared and updated across systems; communications preferences** | Both **explore** items. A patient-entered change to name, address or phone is a **claim** until verified, and a wrong address or name breaks matching (the matcher's identity rules, H-001 family) and spreads to every system that receives it; changes are new versions, never edits. Communications preferences decide whether a notification may be sent at all | MP-28, MP-29 |

### Rules

1. **Monitoring (item 2) is privacy-preserving or it is off.** First-party only: no third-party analytics or advertising SDK,
   no session replay, no device fingerprint. An event is an **allow-listed screen or feature name** and a timestamp, with
   a random install identifier that is reset on sign-out; never a patient or user identifier, a record value, a search
   term, free text, a URL with an identifier, or a location. It is **off until a DPIA covers it** and the patient is told,
   in plain words, with a switch that works (and a staff build follows the trust's policy). A count that could identify
   a person (a screen only one patient used) is not reported. Crash reports follow the same rule and carry no record data
   (§2). The monitoring is for the project's improvement; it never changes what a patient sees.
2. **Onboarding (item 3)** is a short, skippable, translated sequence built from the list of features **enabled for this
   deployment and user**, so it cannot describe a screen that is not there. It makes no clinical claim (for example "check
   your results" says results appear when released, not that they are normal), and it repeats the emergency line (§11 rule 8).
3. **Notifications (items 4, 5)** carry no record text, no name, no service or organisation name, and no result or
   appointment detail: "You have something new in the app" (§11 rule 6), sent only to a patient whose preference allows it
   (item 12). A push can be late, lost or dropped, so **the app never promises one**: the app says that notifications may
   be delayed and that appointments and results are in the app. A cancelled or changed appointment is shown in the app
   at the next open and is not left as a stale notification.
4. **Results (items 6, 7)**: the layout shows, for each value, the number, the unit, the reference range or "No reference
   range was sent", the lab's flag **in words and not by colour alone**, the date taken and released, who requested it
   and which organisation, and whether a clinician has reviewed this version, in this order and at every text size. It
   never shows a trend, a "normal" label, a traffic light or a plain-language interpretation the record does not hold.
   A result from secondary care says who requested it and who reviews it **[deployer to decide the wording]**. No
   screen ranks results by how worrying they look.
5. **Authorised access (item 8)** is requested in the app and **granted outside it**: the app sends a request; the practice
   or the service verifies both people to the deployment's assurance level (national identity proofing where there is
   one) and decides; the subject is told (by a route the requester does not control) and can see and end it; the scope
   and end date are shown to both; adolescent-confidential and sensitive items stay hidden (R-PRX). A request that is
   refused says "not granted" and no more. The trial is limited (a named group, an end date, a review) and each
   outcome is reported to the Clinical Safety Officer.
6. **Pharmacy (item 9)**: the screen shows the current nominated pharmacy and **when it was set**, a change names the
   pharmacy's name and address and asks for a confirmation, and says plainly that a prescription already sent goes
   where it was sent. A change that did not reach the prescribing system is never shown as made.
7. **Barcode and status (items 10, 11)** as in the table; the barcode screen also raises the screen brightness only while
   it is shown, and is blanked in the switcher snapshot (§3 rule 5).
8. **Personal details (item 12)**: until verified, a patient's change is held as a **request** shown to a member of staff
   (the work queue), not applied; the app says "your change has been sent" and never "updated" until the record says so;
   a change of name, date of birth or sex is **never** made from the app alone. Sharing across systems goes through the
   service's outbound choke point (PA-13), which has no real recipient yet (the outbox's only sink is a log line).
9. **Communications preferences (item 12)** are per channel (push, SMS, e-mail, letter) and per kind (appointments,
   results, messages, reminders); a patient can always turn a kind off, except a message the law or clinical safety
   requires, which the app says it cannot turn off and why. The default for a new patient is **off** for push until they
   choose. A preference is a record in the service, not a phone setting, so it follows the patient to a new device.
10. **A third-party processor (items 1, 2, 4)** appears in the DPIA, the record of processing and the data-flow table before
    it is called, with the data it receives, where it is hosted and how it is deleted; a patient's data is never sent
    to one by default.

### Order and what comes first

Cheap and safe first: **onboarding** (3) with the **results layout** (6) and **secondary-care results** (7), because
they are the reading screens of MP-13. **Monitoring** (2) early, so later features are learned from, but only after its
DPIA. Then **communications preferences** (12) and **notifications** (4), which need each other. **Vaccination**
(5), **pharmacy** (9), **prescription status** (11) and **barcode** (10) follow the prescribing route of MP-16 and the
pharmacy directory. **PROMs** (1) after the third-party review. **Authorised access** (8) and **personal details**
(12) last: they need verified identity and the most review, and the feedback marks personal details as exploration.

## 13. Provider features (feedback of 2026-11-23)

Three features for **staff** (clinicians looking after patients), from the maintainer's feedback. They are the staff build's
core, and they are **clinical acts on a small screen**: the risk is a wrong patient, a wrong version, or a signature
given without reading. "WCP" is the existing desktop clinical portal the feedback names; this repository has no copy of
it, so its content is a target to be listed, not a fact (OQ-MP10).

| # | Feature | The service today | Gap |
| --- | --- | --- | --- |
| 1 | **View and sign off test results** | Built: `GET /api/patients/{id}/results`, `…/results/{result_id}`, `POST …/results/{result_id}/sign-off` (a sign-off is of **one version** of a result) | None for the route; the app's screens and the step-up (rule 2) are new |
| 2 | **View patient warnings and adverse reactions** that the desktop shows | Allergies and the allergy statement are built (`/allergies`, `/allergy-statement`); **no route for alerts or warnings** as a kind | An alerts or warnings route, and the list of what the desktop shows (OQ-MP10) |
| 3 | **Add and edit notes and tasks** for the patients you look after, and **share** them with other app users | Tasks: rule-made tasks, a team list and `POST /api/tasks/{id}/close` are built; the **watch list** (`/api/watch-list`) is the nearest to "my patients"; amendments are corrections, not notes. **No route to write a note or to create, edit or share a task** | A notes route, manual task create and edit, a sharing model, and a definition of "patients you are looking after" |

### Rules

1. **The same facts as the desktop and the web UI.** A result, warning or adverse reaction on the phone says what the web UI
   says for the same record, checked by a shared fixture (MP-8). Counts are never hidden (§3 rule 3). "No warnings
   recorded" is not "none exist": a screen says whether the list was checked and when (§3 rule 1).
2. **Signing off is a deliberate act.** The sign-off screen shows the **whole patient banner** (§3 rule 2), the test name, the
   value, unit, range and the lab's flag in words (§12 rule 4), when it was taken and the **version** being signed. Then:
   - it needs a **step-up**: the device's biometrics or passcode again, within a minute before the act, even when the
     app is unlocked (MFA plan, "re-authentication for sensitive actions");
   - the button is **not a swipe, not a long press and not a default**; it names the act ("Sign off this result for
     Jane Doe") and sits away from the edge;
   - **one result at a time**: no "sign all", no batch, no sign-off from a list row or a notification;
   - it sends the version it showed. If the result changed since (a corrected version), the service refuses and the app
     says "This result has changed. Review the new version." and signs nothing;
   - a failed or timed-out request is **never shown as signed**; the app asks the service before it says so, and shows
     who signed and when from the service's answer;
   - **never offline and never queued**: no signature is stored on the device to send later;
   - the app shows the button only to a role the service allows; the service decides, and a refusal is the app's own words
     (§3 rule 4);
   - results are listed oldest unreviewed first, with the count; the app **never ranks by how worrying a result
     looks** and never marks one "normal".
3. **Notes and tasks are record writes, append-only.** An edit is a **new version** with author and time; the earlier version
   stays; nothing is deleted. Before a note or task is saved the **patient is confirmed** (the banner, whole) so it
   cannot be written to the wrong person's record. A saved note or task appears only after the service answers.
   - **Drafts** are held in memory only and discarded at sign-out, with a warning when the app locks that an unsaved draft
     will be lost after the idle limit; an unsent draft is not written to the device (§3 rule 5, §6 rule 4). This costs a
     clinician a half-written note, and that cost is accepted over a clinical text on a lost phone **[maintainer to
     confirm]**.
   - Note fields have autocorrect, prediction and learning off (§6 rule 4); dictation is allowed only if the platform's
     dictation does not send text to a third party without the trust's agreement (not checked) **[open]**.
   - A task has a team that is a role (OQ-TL3), a due date and a status; closing needs the service's answer; an overdue
     task is overdue by the service's clock and not the phone's.
4. **Sharing** is to a **named user or a team that is a role**, chosen from the service's directory, and only people with a
   legitimate relationship to the patient (the care team, the watch list, or a role that may see the record), as the
   service's policy decides. The app shows **who can see** a note or task before it is shared and afterwards, and the
   service writes an audit row for the share and for each read. A user with no access to the patient cannot be chosen,
   and an unshared note is visible to its author only.
5. **Patients you look after** means a list the service gives (the watch list, a team's patients), never a list kept on the
   phone. Looking at a patient outside that list is the web's **break-glass**, which is **not** on the mobile apps
   in this version **[maintainer to confirm]**.
6. **Notifications for staff** (a new result to review, a task assigned) are content-free (§12 rule 3): "You have something
   new" and nothing about the patient or the result.
7. **Audit.** Every view of a result, a warning or a note, every sign-off, note, task and share is audited by the service, the
   same as from the web; the app sends its own name and version so an audit row can say which client acted.
8. **The staff build carries no patient features and the patient build carries none of these** (OQ-MP6).

### Order

Read-only first: **results view** and **warnings and adverse reactions** (with MP-7 and MP-8), then **sign-off** (a write
with clinical effect, after MP-5 sign-in and MP-6 the lock and the step-up), then **tasks** (close, then create and
share), then **notes**. Each write waits for its route and for the entity spec to say who may do it.

## 10. Not done, not verified

No clinician, user or Clinical Safety Officer has looked at this. The 44 pt and 48 dp figures and the RFC 8252 advice are
from the author's knowledge of those documents, not re-read for this page. Keycloak has no `epr-ios` or `epr-android`
client yet, and no patient client (`epr-patient-ios`, `epr-patient-android`). Tasks: `MP-1` to `MP-33` in [`tasks.md`](../../tasks.md).
