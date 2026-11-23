import Foundation
import Testing

@testable import EPRCore

@Suite struct PatientTests {
    @Test func decodesTheServiceSearchAnswer() throws {
        let json = Data(
            """
            {"results": [{"id": "p1", "version": 3, "author": "u1", "drill": false,
              "patient": {"id": "p1", "names": [{"family": "Smith", "given": ["Jo", "Ann"]}],
                          "birth_date": "1980-03", "sex": "unknown",
                          "identifiers": [{"system": "https://fhir.nhs.uk/Id/nhs-number", "value": "9990000018"}],
                          "addresses": [{"postal_code": "ZZ99 9ZZ"}], "telecom": []}}],
             "limit": 50}
            """.utf8)
        let r = try PatientSearchResults.decode(json)
        #expect(r.limit == 50)
        #expect(r.results.count == 1)
        let s = r.results[0]
        #expect(s.version == 3)
        #expect(s.patient.names == [HumanName(family: "Smith", given: ["Jo", "Ann"])])
        #expect(s.patient.birthDate == "1980-03")
        #expect(s.patient.sex == .unknown)
        #expect(s.patient.identifiers.first?.value == "9990000018")
    }

    @Test func absentFieldsAreAbsentNotEmptyOrZero() throws {
        let json = Data(#"{"results": [{"id": "p2", "version": 1, "author": "u1", "patient": {}}], "limit": 50}"#.utf8)
        let p = try PatientSearchResults.decode(json).results[0]
        #expect(p.drill == false)
        #expect(p.patient.names.isEmpty)
        #expect(p.patient.birthDate == nil)
        #expect(p.patient.sex == nil)
        #expect(p.patient.identifiers.isEmpty)
        #expect(p.patient.temporaryIdentity == false)
    }

    @Test func nullIsAbsent() throws {
        let json = Data(
            #"{"results": [{"id": "p3", "version": 1, "author": "u1", "patient": {"birth_date": null, "sex": null, "names": [{"family": null}]}}], "limit": 50}"#
                .utf8)
        let p = try PatientSearchResults.decode(json).results[0].patient
        #expect(p.birthDate == nil)
        #expect(p.sex == nil)
        #expect(p.names == [HumanName(family: nil, given: [])])
    }

    @Test func anUnknownSexValueIsAnErrorNotAGuess() {
        let json = Data(
            #"{"results": [{"id": "p4", "version": 1, "author": "u1", "patient": {"sex": "M"}}], "limit": 50}"#.utf8)
        #expect(throws: DecodingError.self) { try PatientSearchResults.decode(json) }
    }

    @Test func aMissingLimitIsAnError() {
        #expect(throws: DecodingError.self) { try PatientSearchResults.decode(Data(#"{"results": []}"#.utf8)) }
    }
}
