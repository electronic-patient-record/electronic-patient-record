# electronic-patient-record (Android app)

Jetpack Compose app on Lily Design System for Kotlin. A scaffold. **Not for clinical use.**

```sh
sh ../scripts/fetch-lily.sh                        # the pinned Lily checkout, once
./gradlew :core:test :app:testDebugUnitTest        # the core, and the screens under Robolectric
./gradlew :app:installDebug                        # on a running emulator or device
```

Needs a JDK 17 or later and the Android SDK (`ANDROID_HOME`, or `local.properties`). The contract both mobile apps share
is [`../spec/mobile-apps/index.md`](../spec/mobile-apps/index.md); this app's spec is [`spec/index.md`](spec/index.md).
