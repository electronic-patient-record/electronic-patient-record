#!/bin/sh
# Gate mobile-kotlin-test (MP-2, MP-4): the Android app's core on the JVM, and its screens under Robolectric.
# Needs a JDK 17 or later and the Android SDK.
set -eu
root="$(cd "$(dirname "$0")/../.." && pwd)"
sh "$root/scripts/fetch-lily.sh" >/dev/null
# A JAVA_HOME that does not name a JDK breaks the Gradle wrapper; fall back to the java on PATH.
if [ -n "${JAVA_HOME:-}" ] && [ ! -x "$JAVA_HOME/bin/java" ]; then unset JAVA_HOME; fi
if [ -z "${ANDROID_HOME:-}" ] && [ -d "$HOME/Library/Android/sdk" ]; then ANDROID_HOME="$HOME/Library/Android/sdk"; export ANDROID_HOME; fi
cd "$root/electronic-patient-record-mobile-app-with-kotlin"
./gradlew --no-configuration-cache :core:test :app:testDebugUnitTest
