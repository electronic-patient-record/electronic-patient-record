/// The national health number system (UI `NATIONAL_NUMBER_SYSTEM`).
public enum NationalNumber {
    /// The identifier system URI.
    public static let system = "https://fhir.nhs.uk/Id/nhs-number"

    /// Ten digits as `3 3 4` groups; anything else as stored (UI `nationalNumberDisplay`).
    public static func display(_ value: String) -> String {
        guard value.count == 10, value.allSatisfy({ $0.isASCII && $0.isNumber }) else { return value }
        let digits = Array(value)
        return "\(String(digits[0..<3])) \(String(digits[3..<6])) \(String(digits[6...]))"
    }
}

/// The patient banner's words for one stored patient (UI `PatientBanner.svelte`; spec/mobile-apps/index.md §3).
///
/// Every line is whole: nothing here shortens a name or an identifier, and the view must wrap, never truncate
/// (hazard H-061). An absent part says so in words; it is never an empty line.
/// Not here yet: the age (it needs the site's time zone), sex, photo, allergies, death and needs lines.
public struct BannerText: Equatable, Sendable {
    /// Lines that must be read before the identity, in order: drill, then temporary identity.
    public let alerts: [String]
    /// `FAMILY, Given Given`, or "not available".
    public let name: String
    /// `Born: …`.
    public let born: String
    /// `National health number: …`.
    public let nationalNumber: String
    /// `Other identifier: …`, the first identifier of another system.
    public let otherIdentifier: String

    /// The banner for `stored`, in `strings`.
    public init(stored: StoredPatient, strings: Strings = .english) {
        let p = stored.patient
        var alerts: [String] = []
        if stored.drill { alerts.append(strings.bannerDrill) }
        if p.temporaryIdentity { alerts.append(strings.bannerTemporaryIdentity) }
        self.alerts = alerts

        let first = p.names.first
        let parts = [first?.family?.uppercased(), first?.given.joined(separator: " ")]
            .compactMap { $0 }
            .filter { !$0.isEmpty }
        name = parts.isEmpty ? strings.notAvailable : parts.joined(separator: ", ")

        let birth = p.birthDate.flatMap { $0.isEmpty ? nil : $0 } ?? strings.bannerNoBirthDate
        born = "\(strings.bannerBorn): \(birth)"

        let national = p.identifiers.first { $0.system == NationalNumber.system }?.value
        nationalNumber = "\(strings.bannerNationalNumber): \(national.map(NationalNumber.display) ?? strings.bannerNone)"

        let other = p.identifiers.first { $0.system != NationalNumber.system }
        otherIdentifier = "\(strings.bannerOtherId): \(other.map { "\($0.system) \($0.value)" } ?? strings.bannerNone)"
    }

    /// Every line in reading order, as a screen reader should hear the banner.
    public var lines: [String] { alerts + [name, born, nationalNumber, otherIdentifier] }
}
