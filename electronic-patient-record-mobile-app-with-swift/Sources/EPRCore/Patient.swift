import Foundation

// The service's patient identity shapes, as `GET /api/patients` answers them. The field names are the matcher's
// `Patient` (electronic-patient-record-matcher-with-rust/src/patient.rs) and the service's `Stored`
// (src/patients_store.rs). Every list may be omitted and reads as empty; every single value may be omitted and reads as
// absent (`nil`), never as an empty string or zero.

/// A person's name.
public struct HumanName: Codable, Equatable, Sendable {
    /// The family name, if known.
    public var family: String?
    /// The given names, in order.
    public var given: [String]

    /// A name.
    public init(family: String? = nil, given: [String] = []) {
        self.family = family
        self.given = given
    }

    /// Reads a name, with absent `given` as empty.
    public init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        family = try c.decodeIfPresent(String.self, forKey: .family)
        given = try c.decodeIfPresent([String].self, forKey: .given) ?? []
    }
}

/// An identifier such as a national health number.
public struct Identifier: Codable, Equatable, Sendable {
    /// The identifier's system URI.
    public var system: String
    /// The value within that system.
    public var value: String

    /// An identifier.
    public init(system: String, value: String) {
        self.system = system
        self.value = value
    }
}

/// The administrative sex. Absent (`nil`) is not the same as `.unknown`.
public enum AdministrativeSex: String, Codable, Equatable, Sendable {
    /// Male.
    case male
    /// Female.
    case female
    /// Another administrative sex.
    case other
    /// Not known, or no usable answer was given.
    case unknown
}

/// A patient identity record.
public struct Patient: Codable, Equatable, Sendable {
    /// The record's id once stored.
    public var id: String?
    /// The person's names; the first is the one shown.
    public var names: [HumanName]
    /// The birth date as stored: `YYYY`, `YYYY-MM` or `YYYY-MM-DD`.
    public var birthDate: String?
    /// The administrative sex, if recorded.
    public var sex: AdministrativeSex?
    /// Identifiers.
    public var identifiers: [Identifier]
    /// The names and birth date are placeholders for someone not yet identified.
    public var temporaryIdentity: Bool

    enum CodingKeys: String, CodingKey {
        case id, names, sex, identifiers
        case birthDate = "birth_date"
        case temporaryIdentity = "temporary_identity"
    }

    /// A patient.
    public init(
        id: String? = nil,
        names: [HumanName] = [],
        birthDate: String? = nil,
        sex: AdministrativeSex? = nil,
        identifiers: [Identifier] = [],
        temporaryIdentity: Bool = false
    ) {
        self.id = id
        self.names = names
        self.birthDate = birthDate
        self.sex = sex
        self.identifiers = identifiers
        self.temporaryIdentity = temporaryIdentity
    }

    /// Reads a patient, with absent lists as empty and absent values as `nil`.
    public init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decodeIfPresent(String.self, forKey: .id)
        names = try c.decodeIfPresent([HumanName].self, forKey: .names) ?? []
        birthDate = try c.decodeIfPresent(String.self, forKey: .birthDate)
        sex = try c.decodeIfPresent(AdministrativeSex.self, forKey: .sex)
        identifiers = try c.decodeIfPresent([Identifier].self, forKey: .identifiers) ?? []
        temporaryIdentity = try c.decodeIfPresent(Bool.self, forKey: .temporaryIdentity) ?? false
    }
}

/// One stored version of a patient.
public struct StoredPatient: Codable, Equatable, Sendable {
    /// The server's id.
    public var id: String
    /// 1 for the first version.
    public var version: Int
    /// Who wrote this version.
    public var author: String
    /// The identity record.
    public var patient: Patient
    /// A drill or test record, not a real person.
    public var drill: Bool

    /// A stored version.
    public init(id: String, version: Int, author: String, patient: Patient, drill: Bool = false) {
        self.id = id
        self.version = version
        self.author = author
        self.patient = patient
        self.drill = drill
    }

    /// Reads a stored version, with absent `drill` as `false`.
    public init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(String.self, forKey: .id)
        version = try c.decode(Int.self, forKey: .version)
        author = try c.decode(String.self, forKey: .author)
        patient = try c.decode(Patient.self, forKey: .patient)
        drill = try c.decodeIfPresent(Bool.self, forKey: .drill) ?? false
    }
}

/// The answer to `GET /api/patients`.
public struct PatientSearchResults: Codable, Equatable, Sendable {
    /// The matches, up to `limit`.
    public var results: [StoredPatient]
    /// The most results the service returns.
    public var limit: Int

    /// Reads the service's JSON.
    public static func decode(_ data: Data) throws -> PatientSearchResults {
        try JSONDecoder().decode(PatientSearchResults.self, from: data)
    }
}
