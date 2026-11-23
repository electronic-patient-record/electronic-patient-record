import EPRCore
import SwiftUI

/// The patient banner (spec/mobile-apps/index.md §3 item 2; hazard H-061).
///
/// Every line wraps and none is ever shortened: no `lineLimit`, and `fixedSize` lets each line grow downward at any
/// Dynamic Type size. VoiceOver reads it as one group named by `Strings.bannerLabel`, alerts first.
public struct PatientBannerView: View {
    private let banner: BannerText
    private let label: String

    /// The banner for `stored`.
    public init(stored: StoredPatient, strings: Strings = .english) {
        banner = BannerText(stored: stored, strings: strings)
        label = strings.bannerLabel
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            ForEach(Array(banner.alerts.enumerated()), id: \.offset) { _, alert in
                whole(alert).font(.headline)
            }
            whole(banner.name)
                .font(.title2.weight(.bold))
                .accessibilityAddTraits(.isHeader)
            whole(banner.born)
            whole(banner.nationalNumber)
            whole(banner.otherIdentifier)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Text(verbatim: label))
    }

    private func whole(_ line: String) -> some View {
        Text(verbatim: line)
            .lineLimit(nil)
            .fixedSize(horizontal: false, vertical: true)
    }
}
