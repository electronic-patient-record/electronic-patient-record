package org.electronicpatientrecord.app

import org.electronicpatientrecord.core.AdministrativeSex
import org.electronicpatientrecord.core.HumanName
import org.electronicpatientrecord.core.Identifier
import org.electronicpatientrecord.core.NationalNumber
import org.electronicpatientrecord.core.Patient
import org.electronicpatientrecord.core.StoredPatient

/**
 * Synthetic patients for previews and the home screen's sample. Not real people (spec/mobile-apps/index.md §3 item 8).
 * The national health number is from the 999 range that no real person is given.
 */
object SyntheticPatients {
    /** A drill patient with every banner part present and a long name, to show wrapping. */
    val SAMPLE: StoredPatient = StoredPatient(
        id = "00000000-0000-4000-8000-000000000001",
        version = 1,
        author = "synthetic",
        patient = Patient(
            names = listOf(HumanName("Featherstonehaugh-Montgomery", listOf("Alexandra", "Bartholomew"))),
            birthDate = "1980-03-14",
            sex = AdministrativeSex.FEMALE,
            identifiers = listOf(
                Identifier(NationalNumber.SYSTEM, "9990000018"),
                Identifier("urn:example:mrn", "MRN-0001"),
            ),
        ),
        drill = true,
    )
}
