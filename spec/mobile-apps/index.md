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
  line, a crash report or an analytics event.
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
   `dataExtractionRules` excluding all domains) or a file the user can reach. A personal phone holds other apps, so the
   clipboard and the keyboard's learned words are a risk the app cannot fully close; whether record text may be
   copied is open (recorded in H-061).
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
| OQ-MP1 | Who owns the devices: the trust (managed, MDM) or the clinician (bring your own)? It decides remote wipe, distribution and whether a lost-device threat is controlled by the trust | **Answered 2026-11-23 by the maintainer: both, on phones and tablets** (§6, Device ownership). Still open inside it: whether record text may be copied on a personal device, and which record kinds a personal device may hold |
| OQ-MP2 | Claimed `https` redirect or a private-use scheme for sign-in? | Claimed `https` where the deployment can host `apple-app-site-association` and `assetlinks.json` |
| OQ-MP3 | Distribution: the App Store and Google Play, Apple Business Manager and managed Google Play, or both? | Managed distribution; public stores only after a clinical safety case |
| OQ-MP4 | Offline reading of the downtime snapshot on a phone (links OQ-5, OQ-MV1)? | Not until OQ-5 is answered; then read-only, encrypted, wiped at sign-out |
| OQ-MP5 | Is a native app needed for patients (the patient portal) as well as for staff? | Staff first; the portal stays web until asked |

## 10. Not done, not verified

No clinician, user or Clinical Safety Officer has looked at this. The 44 pt and 48 dp figures and the RFC 8252 advice are
from the author's knowledge of those documents, not re-read for this page. Keycloak has no `epr-ios` or `epr-android`
client yet. Tasks: `MP-1` to `MP-12` in [`tasks.md`](../../tasks.md).
