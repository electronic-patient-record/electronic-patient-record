# AGENTS.md — Android app (Kotlin)

Kotlin, Jetpack Compose, Lily Design System for Kotlin. Android 9 (API 28) or later. Read the shared contract
[`../spec/mobile-apps/index.md`](../spec/mobile-apps/index.md), then [`spec/index.md`](spec/index.md).

- **Status**: scaffold (MP-2). One screen with the not-for-clinical-use notice and a sample banner on a synthetic
  patient. Not built: sign-in, any network call, any record screen, the idle lock, `FLAG_SECURE`, translations, a
  release build, signing, distribution.
- **Layout.** `:core` is a Kotlin JVM module with no Android and no UI: rules with tests (`./gradlew :core:test`). `:app` is
  the Compose screens and the one activity. Put a rule in `:core` with a test, never in a composable.
- **Lily** comes from the pinned checkout as an included build: run `sh ../scripts/fetch-lily.sh` once (or after the pin
  moves). Use Lily components where Lily's catalogue says `lily`; no Material library (Lily is unstyled, foundation only).
  Lily's `TextStyle`s leave the font size unspecified: give an explicit `sp` size, never multiply theirs.
- **Safety rules** (contract §3): absent is "not available", never `0` or empty; the banner wraps and is never truncated
  (no `maxLines`, no `TextOverflow.Ellipsis`; `HomeScreenTest` checks this at 320 dp and twice the font size); only a 422
  `errors` list is shown from the API; nothing from the record is written to disk; `allowBackup` stays off; synthetic data
  only, and a sample patient is a drill.
- **Requests** go through `ApiRequest`: `/api/...` only, the version in `Accepts-version`, never `/api/v1`.
- **Gates**: `./gradlew :core:test :app:testDebugUnitTest` and `./gradlew :app:assembleDebug`, as
  `sh ../scripts/gates/mobile-kotlin-test.sh` and `mobile-kotlin-build.sh`, run by `ci-local.sh` (project `kotlin`) and
  by CI. Kotlin warnings are errors. A `JAVA_HOME` that does not name a JDK breaks the wrapper: the gate scripts unset it.
