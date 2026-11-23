#!/bin/sh
# Gate mobile-swift-test (MP-2, MP-4): the iOS app's core and views, `swift test` on macOS. Needs Xcode.
set -eu
root="$(cd "$(dirname "$0")/../.." && pwd)"
sh "$root/scripts/fetch-lily.sh" >/dev/null
cd "$root/electronic-patient-record-mobile-app-with-swift"
swift test
