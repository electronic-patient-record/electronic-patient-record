package org.electronicpatientrecord.core

import kotlinx.serialization.SerializationException
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class PatientTest {
    @Test fun decodesTheServiceSearchAnswer() {
        val r = PatientSearchResults.decode(
            """
            {"results": [{"id": "p1", "version": 3, "author": "u1", "drill": false,
              "patient": {"id": "p1", "names": [{"family": "Smith", "given": ["Jo", "Ann"]}],
                          "birth_date": "1980-03", "sex": "unknown",
                          "identifiers": [{"system": "https://fhir.nhs.uk/Id/nhs-number", "value": "9990000018"}],
                          "addresses": [{"postal_code": "ZZ99 9ZZ"}], "telecom": []}}],
             "limit": 50}
            """,
        )
        assertEquals(50, r.limit)
        val s = r.results.single()
        assertEquals(3, s.version)
        assertEquals(listOf(HumanName("Smith", listOf("Jo", "Ann"))), s.patient.names)
        assertEquals("1980-03", s.patient.birthDate)
        assertEquals(AdministrativeSex.UNKNOWN, s.patient.sex)
        assertEquals("9990000018", s.patient.identifiers.first().value)
    }

    @Test fun absentFieldsAreAbsentNotEmptyOrZero() {
        val p = PatientSearchResults.decode("""{"results": [{"id": "p2", "version": 1, "author": "u1", "patient": {}}], "limit": 50}""")
            .results.single()
        assertFalse(p.drill)
        assertTrue(p.patient.names.isEmpty())
        assertNull(p.patient.birthDate)
        assertNull(p.patient.sex)
        assertTrue(p.patient.identifiers.isEmpty())
        assertFalse(p.patient.temporaryIdentity)
    }

    @Test fun nullIsAbsent() {
        val p = PatientSearchResults.decode(
            """{"results": [{"id": "p3", "version": 1, "author": "u1", "patient": {"birth_date": null, "sex": null, "names": [{"family": null, "given": null}], "identifiers": null}}], "limit": 50}""",
        ).results.single().patient
        assertNull(p.birthDate)
        assertNull(p.sex)
        assertEquals(listOf(HumanName(null, emptyList())), p.names)
        assertTrue(p.identifiers.isEmpty())
    }

    @Test fun anUnknownSexValueIsAnErrorNotAGuess() {
        assertFailsWith<SerializationException> {
            PatientSearchResults.decode("""{"results": [{"id": "p4", "version": 1, "author": "u1", "patient": {"sex": "M"}}], "limit": 50}""")
        }
    }

    @Test fun aMissingLimitIsAnError() {
        assertFailsWith<SerializationException> { PatientSearchResults.decode("""{"results": []}""") }
    }
}
