#!/bin/sh
# Gate mobile-swift-build (MP-2, MP-4): the iOS app builds for the iOS Simulator, warnings as errors. Needs Xcode.
set -eu
root="$(cd "$(dirname "$0")/../.." && pwd)"
sh "$root/scripts/fetch-lily.sh" >/dev/null
cd "$root/electronic-patient-record-mobile-app-with-swift"
xcodebuild -project App/ElectronicPatientRecord.xcodeproj -scheme ElectronicPatientRecord \
  -destination 'generic/platform=iOS Simulator' -derivedDataPath "$root/target/mobile-swift/DerivedData" \
  CODE_SIGNING_ALLOWED=NO -quiet build
