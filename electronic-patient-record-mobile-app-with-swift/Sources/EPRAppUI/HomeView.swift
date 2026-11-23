import EPRCore
import LilyUI
import SwiftUI

/// The first screen: the product name, the not-for-clinical-use notice and a sample banner on a synthetic patient.
/// Nothing here calls the service; sign-in and search are MP-5 and MP-7.
public struct HomeView: View {
    private let strings: Strings

    /// The home screen in `strings`.
    public init(strings: Strings = .english) {
        self.strings = strings
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text(verbatim: strings.homeTitle)
                    .font(.largeTitle.weight(.bold))
                    .accessibilityAddTraits(.isHeader)
                    .fixedSize(horizontal: false, vertical: true)
                WarningText(prefix: strings.warningPrefix, text: strings.notClinical)
                Card(heading: strings.sampleHeading, headingLevel: .h2) {
                    PatientBannerView(stored: SyntheticPatients.sample, strings: strings)
                }
            }
            .padding()
        }
    }
}

#Preview {
    HomeView()
}
