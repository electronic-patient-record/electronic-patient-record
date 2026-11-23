// swift-tools-version: 6.0
// Electronic Patient Record for iOS and iPadOS (spec/mobile-apps, MP-2). NOT FOR CLINICAL USE.
//
// EPRCore has no UI and no networking: the rules the app follows, each with a test that runs on macOS (`swift test`).
// EPRAppUI is the SwiftUI screens, built from Lily Design System; the app target in App/ is a thin shell around it.
// Lily is not published yet, so it comes from the pinned checkout that `sh ../scripts/fetch-lily.sh` makes (MP-3).
import PackageDescription

let package = Package(
    name: "ElectronicPatientRecordMobile",
    defaultLocalization: "en",
    platforms: [.iOS(.v18), .macOS(.v15)],
    products: [
        .library(name: "EPRCore", targets: ["EPRCore"]),
        .library(name: "EPRAppUI", targets: ["EPRAppUI"]),
    ],
    dependencies: [
        .package(path: "../vendor/lily-design-system/lily-design-system-swift"),
    ],
    targets: [
        .target(name: "EPRCore"),
        .target(
            name: "EPRAppUI",
            dependencies: [
                "EPRCore",
                .product(name: "LilyUI", package: "lily-design-system-swift"),
            ]
        ),
        .testTarget(name: "EPRCoreTests", dependencies: ["EPRCore"]),
        .testTarget(name: "EPRAppUITests", dependencies: ["EPRAppUI", "EPRCore"]),
    ],
    swiftLanguageModes: [.v6]
)
