package org.electronicpatientrecord.core

/** The national health number system (UI `NATIONAL_NUMBER_SYSTEM`). */
public object NationalNumber {
    /** The identifier system URI. */
    public const val SYSTEM: String = "https://fhir.nhs.uk/Id/nhs-number"

    /** Ten digits as `3 3 4` groups; anything else as stored (UI `nationalNumberDisplay`). */
    public fun display(value: String): String =
        if (value.length == 10 && value.all { it in '0'..'9' }) {
            "${value.substring(0, 3)} ${value.substring(3, 6)} ${value.substring(6)}"
        } else {
            value
        }
}

/**
 * The patient banner's words for one stored patient (UI `PatientBanner.svelte`; spec/mobile-apps/index.md §3).
 *
 * Every line is whole: nothing here shortens a name or an identifier, and the screen must wrap, never truncate (hazard
 * H-061). An absent part says so in words; it is never an empty line. Not here yet: the age (it needs the site's time
 * zone), sex, photo, allergies, death and needs lines.
 */
public data class BannerText(
    /** Lines that must be read before the identity, in order: drill, then temporary identity. */
    val alerts: List<String>,
    /** `FAMILY, Given Given`, or "not available". */
    val name: String,
    /** `Born: …`. */
    val born: String,
    /** `National health number: …`. */
    val nationalNumber: String,
    /** `Other identifier: …`, the first identifier of another system. */
    val otherIdentifier: String,
) {
    /** Every line in reading order, as a screen reader should hear the banner. */
    val lines: List<String> get() = alerts + listOf(name, born, nationalNumber, otherIdentifier)

    public companion object {
        /** The banner for [stored], in [strings]. */
        public fun of(stored: StoredPatient, strings: Strings = Strings.ENGLISH): BannerText {
            val p = stored.patient
            val alerts = buildList {
                if (stored.drill) add(strings.bannerDrill)
                if (p.temporaryIdentity) add(strings.bannerTemporaryIdentity)
            }
            val first = p.names.firstOrNull()
            val parts = listOfNotNull(first?.family?.uppercase(), first?.given?.joinToString(" ")).filter { it.isNotEmpty() }
            val name = if (parts.isEmpty()) strings.notAvailable else parts.joinToString(", ")
            val birth = p.birthDate?.takeIf { it.isNotEmpty() } ?: strings.bannerNoBirthDate
            val national = p.identifiers.firstOrNull { it.system == NationalNumber.SYSTEM }?.value
            val other = p.identifiers.firstOrNull { it.system != NationalNumber.SYSTEM }
            return BannerText(
                alerts = alerts,
                name = name,
                born = "${strings.bannerBorn}: $birth",
                nationalNumber = "${strings.bannerNationalNumber}: ${national?.let(NationalNumber::display) ?: strings.bannerNone}",
                otherIdentifier = "${strings.bannerOtherId}: ${other?.let { "${it.system} ${it.value}" } ?: strings.bannerNone}",
            )
        }
    }
}
