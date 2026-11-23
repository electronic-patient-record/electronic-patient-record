# iOS app: index

| Path | Purpose |
|---|---|
| [AGENTS.md](AGENTS.md) | Rules for agents working on the iOS app |
| [spec/index.md](spec/index.md) | The iOS app's specification |
| `Package.swift` | `EPRCore` (rules, no UI) and `EPRAppUI` (SwiftUI from Lily) |
| `Sources/EPRCore/ApiRequest.swift` | `/api` paths, the `Accepts-version` header, HTTPS only |
| `Sources/EPRCore/Banner.swift` | The patient banner's words; nothing shortened |
| `App/ElectronicPatientRecord.xcodeproj` | The app target |
| `Tests/` | Swift Testing |
