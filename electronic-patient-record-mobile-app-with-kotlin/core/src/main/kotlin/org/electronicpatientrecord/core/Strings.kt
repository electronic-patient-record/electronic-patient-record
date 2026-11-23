package org.electronicpatientrecord.core

/**
 * The app's words (spec/mobile-apps/index.md §3 item 7). Where a key has the web UI's meaning it has the web UI's name and
 * English (electronic-patient-record-ui-with-svelte/src/lib/strings.ts). English only so far: no other locale is built (MP-9).
 */
public data class Strings(
    /** The product name (UI `homeTitle`). */
    val homeTitle: String,
    /** The not-for-clinical-use notice (UI `notClinical`). */
    val notClinical: String,
    /** The prefix of a warning, read before it (app only). */
    val warningPrefix: String,
    /** A missing value (UI `notAvailable`). */
    val notAvailable: String,
    /** The banner's accessibility label (UI `bannerLabel`). */
    val bannerLabel: String,
    /** UI `bannerBorn`. */
    val bannerBorn: String,
    /** UI `bannerNoBirthDate`. */
    val bannerNoBirthDate: String,
    /** UI `bannerNationalNumber`. */
    val bannerNationalNumber: String,
    /** UI `bannerOtherId`. */
    val bannerOtherId: String,
    /** UI `bannerNone`. */
    val bannerNone: String,
    /** UI `bannerDrill`. */
    val bannerDrill: String,
    /** UI `bannerTemporaryIdentity`. */
    val bannerTemporaryIdentity: String,
    /** The heading over the sample banner on the home screen (app only). */
    val sampleHeading: String,
) {
    public companion object {
        /** English, the source. */
        public val ENGLISH: Strings = Strings(
            homeTitle = "Electronic Patient Record",
            notClinical = "Not for clinical use. Do not enter real patient data.",
            warningPrefix = "Important",
            notAvailable = "not available",
            bannerLabel = "Patient banner",
            bannerBorn = "Born",
            bannerNoBirthDate = "Birth date not recorded",
            bannerNationalNumber = "National health number",
            bannerOtherId = "Other identifier",
            bannerNone = "none recorded",
            bannerDrill = "DRILL — NOT A REAL PATIENT. Nothing recorded here belongs to a person.",
            bannerTemporaryIdentity =
                "TEMPORARY IDENTITY — NOT YET IDENTIFIED. The name and birth date are placeholders, not this person's own.",
            sampleHeading = "Sample patient (synthetic)",
        )
    }
}
