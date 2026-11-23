# electronic-patient-record (iOS app)

SwiftUI app on Lily Design System for Swift. A scaffold. **Not for clinical use.**

```sh
sh ../scripts/fetch-lily.sh      # the pinned Lily checkout, once
swift test                       # the core and the views, on macOS
open App/ElectronicPatientRecord.xcodeproj   # run on a simulator from Xcode
```

The contract both mobile apps share is [`../spec/mobile-apps/index.md`](../spec/mobile-apps/index.md); this app's spec
is [`spec/index.md`](spec/index.md).
