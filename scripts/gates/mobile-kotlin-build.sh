#!/bin/sh
# Gate mobile-kotlin-build (MP-2, MP-4): the Android app's debug APK builds, Kotlin warnings as errors.
# Needs a JDK 17 or later and the Android SDK.
set -eu
root="$(cd "$(dirname "$0")/../.." && pwd)"
sh "$root/scripts/fetch-lily.sh" >/dev/null
if [ -n "${JAVA_HOME:-}" ] && [ ! -x "$JAVA_HOME/bin/java" ]; then unset JAVA_HOME; fi
if [ -z "${ANDROID_HOME:-}" ] && [ -d "$HOME/Library/Android/sdk" ]; then ANDROID_HOME="$HOME/Library/Android/sdk"; export ANDROID_HOME; fi
cd "$root/electronic-patient-record-mobile-app-with-kotlin"
./gradlew --no-configuration-cache :app:assembleDebug
