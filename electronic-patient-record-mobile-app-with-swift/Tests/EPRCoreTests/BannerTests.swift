import Testing

@testable import EPRCore

private func stored(_ patient: Patient, drill: Bool = false) -> StoredPatient {
    StoredPatient(id: "p1", version: 1, author: "u1", patient: patient, drill: drill)
}

@Suite struct BannerTests {
    @Test func everyPartPresent() {
        let b = BannerText(
            stored: stored(
                Patient(
                    names: [HumanName(family: "Smith", given: ["Jo", "Ann"])],
                    birthDate: "1980-03-14",
                    identifiers: [
                        Identifier(system: "urn:example:mrn", value: "MRN-1"),
                        Identifier(system: NationalNumber.system, value: "9990000018"),
                    ])))
        #expect(b.alerts.isEmpty)
        #expect(b.name == "SMITH, Jo Ann")
        #expect(b.born == "Born: 1980-03-14")
        #expect(b.nationalNumber == "National health number: 999 000 0018")
        #expect(b.otherIdentifier == "Other identifier: urn:example:mrn MRN-1")
    }

    @Test func everyPartAbsentSaysSoInWords() {
        let b = BannerText(stored: stored(Patient()))
        #expect(b.name == "not available")
        #expect(b.born == "Born: Birth date not recorded")
        #expect(b.nationalNumber == "National health number: none recorded")
        #expect(b.otherIdentifier == "Other identifier: none recorded")
        #expect(b.lines.allSatisfy { !$0.isEmpty })
    }

    @Test func anEmptyBirthDateIsNotRecorded() {
        #expect(BannerText(stored: stored(Patient(birthDate: ""))).born == "Born: Birth date not recorded")
    }

    @Test func onlyAFamilyNameOrOnlyGivenNames() {
        #expect(BannerText(stored: stored(Patient(names: [HumanName(family: "Smith")]))).name == "SMITH")
        #expect(BannerText(stored: stored(Patient(names: [HumanName(given: ["Jo"])]))).name == "Jo")
        #expect(BannerText(stored: stored(Patient(names: [HumanName(family: "", given: [])]))).name == "not available")
    }

    @Test func aLongNameAndIdentifierAreKeptWhole() {
        let family = String(repeating: "Featherstonehaugh-", count: 8) + "Montgomery"
        let value = String(repeating: "X", count: 120)
        let b = BannerText(
            stored: stored(
                Patient(
                    names: [HumanName(family: family, given: ["Alexandra"])],
                    identifiers: [Identifier(system: "urn:example:mrn", value: value)])))
        #expect(b.name == "\(family.uppercased()), Alexandra")
        #expect(b.otherIdentifier.hasSuffix(value))
        #expect(!b.lines.contains { $0.contains("…") })
    }

    @Test func alertsComeFirstDrillThenTemporaryIdentity() {
        let b = BannerText(stored: stored(Patient(temporaryIdentity: true), drill: true))
        #expect(b.alerts == [Strings.english.bannerDrill, Strings.english.bannerTemporaryIdentity])
        #expect(Array(b.lines.prefix(2)) == b.alerts)
    }

    @Test(arguments: [
        ("9990000018", "999 000 0018"), ("999000001", "999000001"), ("99900000189", "99900000189"),
        ("999-000-0018", "999-000-0018"), ("٩٩٩٠٠٠٠٠١٨", "٩٩٩٠٠٠٠٠١٨"),
    ])
    func nationalNumberDisplay(value: String, shown: String) {
        #expect(NationalNumber.display(value) == shown)
    }
}
