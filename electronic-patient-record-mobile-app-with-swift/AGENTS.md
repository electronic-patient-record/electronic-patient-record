# AGENTS.md — iOS app (Swift)

Swift 6, SwiftUI, Lily Design System for Swift. iOS and iPadOS 18 or later. Read the shared contract
[`../spec/mobile-apps/index.md`](../spec/mobile-apps/index.md), then [`spec/index.md`](spec/index.md).

- **Status**: scaffold (MP-2). One screen with the not-for-clinical-use notice and a sample banner on a synthetic
  patient. Not built: sign-in, any network call, any record screen, the idle lock, the snapshot cover, translations,
  signing, distribution.
- **Layout.** `Package.swift` has `EPRCore` (no UI, no networking: rules with tests) and `EPRAppUI` (SwiftUI views from
  Lily). `App/ElectronicPatientRecord.xcodeproj` is the app target, a thin shell; its sources folder is a synchronised
  group, so a new file there needs no project edit. Put a rule in `EPRCore` with a Swift Testing test, never in a view.
- **Lily** comes from the pinned checkout: run `sh ../scripts/fetch-lily.sh` once (or after the pin moves). Use a Lily
  component where Lily's catalogue says `lily`; pass every word as a `String` from `Strings`.
- **Safety rules** (contract §3): absent is "not available", never `0` or empty; the banner wraps and is never
  truncated (no `lineLimit`, no `truncationMode`); only a 422 `errors` list is shown from the API; nothing from the
  record is written to disk; synthetic data only, and a sample patient is a drill.
- **Requests** go through `ApiRequest`: `/api/...` only, the version in `Accepts-version`, never `/api/v1`.
- **Gates**: `swift test` (here), and `xcodebuild -project App/ElectronicPatientRecord.xcodeproj -scheme
  ElectronicPatientRecord -destination 'generic/platform=iOS Simulator' CODE_SIGNING_ALLOWED=NO build`. Warnings are
  errors in the app target. CI does not run them yet (MP-4).
