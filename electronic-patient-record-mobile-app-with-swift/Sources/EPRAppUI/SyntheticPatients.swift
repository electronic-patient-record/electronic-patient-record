import EPRCore

/// Synthetic patients for previews and the home screen's sample. Not real people (spec/mobile-apps/index.md §3 item 8).
/// The national health number is from the 999 range that no real person is given.
public enum SyntheticPatients {
    /// A patient with every banner part present and a long name, to show wrapping.
    public static let sample = StoredPatient(
        id: "00000000-0000-4000-8000-000000000001",
        version: 1,
        author: "synthetic",
        patient: Patient(
            names: [HumanName(family: "Featherstonehaugh-Montgomery", given: ["Alexandra", "Bartholomew"])],
            birthDate: "1980-03-14",
            sex: .female,
            identifiers: [
                Identifier(system: NationalNumber.system, value: "9990000018"),
                Identifier(system: "urn:example:mrn", value: "MRN-0001"),
            ]
        ),
        drill: true
    )
}
