import EPRAppUI
import SwiftUI

/// The iOS app: a thin shell around EPRAppUI (spec/mobile-apps/index.md §2). NOT FOR CLINICAL USE.
@main
struct ElectronicPatientRecordApp: App {
    var body: some Scene {
        WindowGroup {
            HomeView()
        }
    }
}
