# Android app — Specification

How the Android app meets the shared contract, [`../../spec/mobile-apps/index.md`](../../spec/mobile-apps/index.md),
which wins where the two disagree. The core is the same as the iOS app's, rule for rule, so a behaviour change is made in
both or the difference is written here.

## 1. Built (scaffold, MP-2)

- **`:core`** (Kotlin JVM, no Android, explicit API mode):
  - `ApiRequest.get(base, path, query, apiVersion, allowLoopbackHttp)` builds a `GET` and throws `ApiRequestException`
    with an `ApiRequestError` for the same refusals as the iOS app: a base URL that is not `https` (plain `http` only to
    `127.0.0.1`, `::1` or `localhost`, and only when allowed), a base URL that is not a bare origin or does not parse, a
    path not under `/api/`, an empty, `.` or `..` segment or one holding `%`, `\`, `?` or `#`, a version segment straight
    after `/api`, and an API version that is not 1 to 32 letters, digits, dots and hyphens. Query names and values are
    percent-encoded keeping only RFC 3986 unreserved characters (so `'` is `%27`, where the iOS app's `URLComponents`
    leaves it). Headers: `Accept: application/json` and `Accepts-version`; never `Authorization`.
  - `PatientSearchResults.decode`, `StoredPatient`, `Patient`, `HumanName`, `Identifier`, `AdministrativeSex`
    (kotlinx.serialization): an omitted or `null` list is empty and an omitted or `null` value is `null`; unknown keys are
    ignored; an unknown `sex` value is an error (its serializer is a string, so `coerceInputValues` does not turn it
    into `null`).
  - `BannerText.of` and `NationalNumber.display`: the same lines and words as the iOS app and the web UI.
  - `Strings.ENGLISH`.
- **`:app`**: `MainActivity` shows `HomeScreen` (a heading, Lily `WarningText` with the notice, a Lily `LilyCard` holding
  the sample banner) in a vertical scroll. `PatientBanner` is a traversal group described as "Patient banner"; every line
  is a `BasicText` with no line limit. `SyntheticPatients.SAMPLE` is a drill record with a 999-range number.
  `applicationId` `org.electronicpatientrecord.app`, label "EPR (not for clinical use)", `minSdk` 28, `targetSdk` 37,
  `allowBackup` off and every data-extraction domain excluded, no permissions.

## 2. Tests

`./gradlew :core:test`: `ApiRequestTest`, `PatientTest`, `BannerTest`, the same cases as the iOS app's.
`./gradlew :app:testDebugUnitTest` (Robolectric, SDK 36, native graphics): the notice is spoken with its prefix, the title
and the patient's name are headings, the banner is named as a group, an absent patient says so in words, the lines read
top to bottom alerts first, and, at 320 dp and twice the font size, every banner line is laid out whole with no ellipsis
and no line wider than the banner while the long name wraps. That test was checked by adding `maxLines = 1` with an
ellipsis to the name: it failed, as it should.

## 3. Not built, not verified

No instrumented test runs on an emulator in a gate (MP-10). Not tried on a device, with TalkBack, Switch Access, a
tablet, landscape or dark mode. The age, sex, photo, allergy, death and needs lines of the web banner are not here.
Lily is unstyled, so the screen is too.
