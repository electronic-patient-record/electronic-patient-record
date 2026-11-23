// Renders each screen at 320 points wide, the narrowest supported, so every body runs. What VoiceOver hears is checked
// through the accessibility tree in UI tests on a simulator, which are not built yet (MP-10).
import EPRCore
import SwiftUI
import Testing

@testable import EPRAppUI

@MainActor private func renders(_ view: some View) -> Bool {
    ImageRenderer(content: view.frame(width: 320)).cgImage != nil
}

@Suite @MainActor struct RenderTests {
    @Test func homeRenders() {
        #expect(renders(HomeView()))
    }

    @Test func aBannerWithEveryPartAbsentRenders() {
        #expect(renders(PatientBannerView(stored: StoredPatient(id: "p", version: 1, author: "a", patient: Patient()))))
    }

    @Test func theSampleIsADrillSoItCannotBeMistakenForAPerson() {
        #expect(SyntheticPatients.sample.drill)
        #expect(BannerText(stored: SyntheticPatients.sample).alerts.first == Strings.english.bannerDrill)
    }
}
