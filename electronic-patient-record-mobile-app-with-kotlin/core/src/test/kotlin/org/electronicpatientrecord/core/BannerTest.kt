package org.electronicpatientrecord.core

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

private fun stored(patient: Patient, drill: Boolean = false) = StoredPatient("p1", 1, "u1", patient, drill)

class BannerTest {
    @Test fun everyPartPresent() {
        val b = BannerText.of(
            stored(
                Patient(
                    names = listOf(HumanName("Smith", listOf("Jo", "Ann"))),
                    birthDate = "1980-03-14",
                    identifiers = listOf(
                        Identifier("urn:example:mrn", "MRN-1"),
                        Identifier(NationalNumber.SYSTEM, "9990000018"),
                    ),
                ),
            ),
        )
        assertTrue(b.alerts.isEmpty())
        assertEquals("SMITH, Jo Ann", b.name)
        assertEquals("Born: 1980-03-14", b.born)
        assertEquals("National health number: 999 000 0018", b.nationalNumber)
        assertEquals("Other identifier: urn:example:mrn MRN-1", b.otherIdentifier)
    }

    @Test fun everyPartAbsentSaysSoInWords() {
        val b = BannerText.of(stored(Patient()))
        assertEquals("not available", b.name)
        assertEquals("Born: Birth date not recorded", b.born)
        assertEquals("National health number: none recorded", b.nationalNumber)
        assertEquals("Other identifier: none recorded", b.otherIdentifier)
        assertTrue(b.lines.all { it.isNotEmpty() })
    }

    @Test fun anEmptyBirthDateIsNotRecorded() {
        assertEquals("Born: Birth date not recorded", BannerText.of(stored(Patient(birthDate = ""))).born)
    }

    @Test fun onlyAFamilyNameOrOnlyGivenNames() {
        assertEquals("SMITH", BannerText.of(stored(Patient(names = listOf(HumanName("Smith"))))).name)
        assertEquals("Jo", BannerText.of(stored(Patient(names = listOf(HumanName(given = listOf("Jo")))))).name)
        assertEquals("not available", BannerText.of(stored(Patient(names = listOf(HumanName("", emptyList()))))).name)
    }

    @Test fun aLongNameAndIdentifierAreKeptWhole() {
        val family = "Featherstonehaugh-".repeat(8) + "Montgomery"
        val value = "X".repeat(120)
        val b = BannerText.of(
            stored(Patient(names = listOf(HumanName(family, listOf("Alexandra"))), identifiers = listOf(Identifier("urn:example:mrn", value)))),
        )
        assertEquals("${family.uppercase()}, Alexandra", b.name)
        assertTrue(b.otherIdentifier.endsWith(value))
        assertFalse(b.lines.any { "…" in it })
    }

    @Test fun alertsComeFirstDrillThenTemporaryIdentity() {
        val b = BannerText.of(stored(Patient(temporaryIdentity = true), drill = true))
        assertEquals(listOf(Strings.ENGLISH.bannerDrill, Strings.ENGLISH.bannerTemporaryIdentity), b.alerts)
        assertEquals(b.alerts, b.lines.take(2))
    }

    @Test fun nationalNumberDisplay() {
        val cases = listOf(
            "9990000018" to "999 000 0018", "999000001" to "999000001", "99900000189" to "99900000189",
            "999-000-0018" to "999-000-0018", "٩٩٩٠٠٠٠٠١٨" to "٩٩٩٠٠٠٠٠١٨",
        )
        for ((value, shown) in cases) assertEquals(shown, NationalNumber.display(value))
    }
}
