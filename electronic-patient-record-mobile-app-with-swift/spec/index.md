# iOS app — Specification

How the iOS app meets the shared contract, [`../../spec/mobile-apps/index.md`](../../spec/mobile-apps/index.md), which
wins where the two disagree.

## 1. Built (scaffold, MP-2)

- **`EPRCore`** (no UI, no networking):
  - `ApiRequest.get(base:path:query:apiVersion:allowLoopbackHTTP:)` builds a `GET` and refuses, before any request: a
    base URL that is not `https` (plain `http` only to `127.0.0.1`, `::1` or `localhost`, and only when the caller allows
    it), a base URL with a path, query, fragment or user information, a path not under `/api/`, an empty, `.` or `..`
    segment or one holding `%`, `\`, `?` or `#`, a version segment straight after `/api` (`/api/v1`), and an API version
    that is not 1 to 32 letters, digits, dots and hyphens. Headers: `Accept: application/json` and `Accepts-version`. It
    never adds `Authorization`.
  - `PatientSearchResults`, `StoredPatient`, `Patient`, `HumanName`, `Identifier`, `AdministrativeSex` decode the service's
    `GET /api/patients` answer. An omitted or `null` list is empty and an omitted or `null` value is `nil`; an unknown
    `sex` value is a decoding error, not a guess. Fields the scaffold does not show (addresses, telecom, death date) are
    ignored.
  - `BannerText` gives the banner's lines with the web UI's words: drill and temporary-identity alerts first, then
    `FAMILY, Given` (or "not available"), `Born: …` (or "Birth date not recorded", also for an empty string), the national
    health number in `3 3 4` groups (or "none recorded") and the first other identifier. Nothing is shortened.
  - `Strings.english`: the words above, named as the web UI's keys where the meaning is the same.
- **`EPRAppUI`**: `HomeView` (title, Lily `WarningText` with the notice, a Lily `Card` holding the sample banner) and
  `PatientBannerView` (every line wraps; one accessibility group named "Patient banner").
  `SyntheticPatients.sample` is a drill record with a 999-range number.
- **App**: `App/ElectronicPatientRecord.xcodeproj`, bundle id `org.electronicpatientrecord.app`, display name
  "EPR (not for clinical use)", iPhone and iPad, warnings as errors.

## 2. Tests

`swift test`: `ApiRequestTests` (every refusal above, query encoding, loopback only when allowed), `PatientTests`
(the search answer, absent and `null` fields, an unknown sex, a missing limit), `BannerTests` (every part present and
absent, a long name and identifier kept whole, alert order, the number grouping) and `RenderTests` (each view renders at
320 points). The simulator build compiles the app for iOS.

## 3. Not built, not verified

No UI test reads the accessibility tree yet, and none runs on a simulator in a gate (MP-10). The gates
`mobile-swift-test` and `mobile-swift-build` run these in `ci-local.sh` and CI (MP-4). The banner was looked at by
hand once on an iPhone 16e simulator (iOS 27.2) at the default and the largest accessibility text size: every line
wrapped. Not tried on a device, with VoiceOver, on an iPad, in landscape, or in dark mode. The age, sex, photo,
allergy, death and needs lines of the web banner are not here. Lily's plain styles are used, so the screen is unstyled.
