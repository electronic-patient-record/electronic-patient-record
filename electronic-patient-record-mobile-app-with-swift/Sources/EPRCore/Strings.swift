/// The app's words (spec/mobile-apps/index.md §3 item 7). Where a key has the web UI's meaning it has the web UI's name
/// and English (electronic-patient-record-ui-with-svelte/src/lib/strings.ts). English only so far: no other locale is
/// built (MP-9).
public struct Strings: Equatable, Sendable {
    /// The product name (UI `homeTitle`).
    public var homeTitle: String
    /// The not-for-clinical-use notice (UI `notClinical`).
    public var notClinical: String
    /// The prefix of a warning, read before it (app only).
    public var warningPrefix: String
    /// A missing value (UI `notAvailable`).
    public var notAvailable: String
    /// The banner's accessibility label (UI `bannerLabel`).
    public var bannerLabel: String
    /// UI `bannerBorn`.
    public var bannerBorn: String
    /// UI `bannerNoBirthDate`.
    public var bannerNoBirthDate: String
    /// UI `bannerNationalNumber`.
    public var bannerNationalNumber: String
    /// UI `bannerOtherId`.
    public var bannerOtherId: String
    /// UI `bannerNone`.
    public var bannerNone: String
    /// UI `bannerDrill`.
    public var bannerDrill: String
    /// UI `bannerTemporaryIdentity`.
    public var bannerTemporaryIdentity: String
    /// The heading over the sample banner on the home screen (app only).
    public var sampleHeading: String

    /// English, the source.
    public static let english = Strings(
        homeTitle: "Electronic Patient Record",
        notClinical: "Not for clinical use. Do not enter real patient data.",
        warningPrefix: "Important",
        notAvailable: "not available",
        bannerLabel: "Patient banner",
        bannerBorn: "Born",
        bannerNoBirthDate: "Birth date not recorded",
        bannerNationalNumber: "National health number",
        bannerOtherId: "Other identifier",
        bannerNone: "none recorded",
        bannerDrill: "DRILL — NOT A REAL PATIENT. Nothing recorded here belongs to a person.",
        bannerTemporaryIdentity:
            "TEMPORARY IDENTITY — NOT YET IDENTIFIED. The name and birth date are placeholders, not this person's own.",
        sampleHeading: "Sample patient (synthetic)"
    )
}
