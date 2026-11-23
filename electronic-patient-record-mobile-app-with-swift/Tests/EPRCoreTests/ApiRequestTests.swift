import Foundation
import Testing

@testable import EPRCore

private let base = URL(string: "https://epr.example.org")!

@Suite struct ApiRequestTests {
    @Test func aGetUnderApiCarriesTheVersionHeader() throws {
        let r = try ApiRequest.get(base: base, path: "/api/patients", apiVersion: "1")
        #expect(r.url.absoluteString == "https://epr.example.org/api/patients")
        #expect(r.method == "GET")
        #expect(r.headers["Accepts-version"] == "1")
        #expect(r.headers["Accept"] == "application/json")
        #expect(r.headers["Authorization"] == nil)
    }

    @Test func queryItemsAreEncoded() throws {
        let r = try ApiRequest.get(
            base: base, path: "/api/patients",
            query: [URLQueryItem(name: "family", value: "O'Brien Smith")], apiVersion: "1")
        #expect(r.url.absoluteString == "https://epr.example.org/api/patients?family=O'Brien%20Smith")
    }

    @Test(arguments: ["/patients", "/fhir/R4/Patient", "/apix/patients", "api/patients", "/api", ""])
    func aPathOutsideApiIsRefused(path: String) {
        #expect(throws: ApiRequestError.pathOutsideAPI) {
            try ApiRequest.get(base: base, path: path, apiVersion: "1")
        }
    }

    @Test(arguments: [
        "/api/", "/api//patients", "/api/./patients", "/api/../admin", "/api/patients/%2e%2e",
        "/api/patients\\x", "/api/patients?family=x", "/api/patients#top",
    ])
    func aBadSegmentIsRefused(path: String) {
        #expect(throws: ApiRequestError.badPathSegment) {
            try ApiRequest.get(base: base, path: path, apiVersion: "1")
        }
    }

    @Test(arguments: ["/api/v1/patients", "/api/v2", "/api/V10/patients"])
    func aVersionSegmentIsRefused(path: String) {
        #expect(throws: ApiRequestError.versionInPath) {
            try ApiRequest.get(base: base, path: path, apiVersion: "1")
        }
    }

    @Test func aSegmentThatOnlyStartsWithVIsAllowed() throws {
        let r = try ApiRequest.get(base: base, path: "/api/vitals/v1x", apiVersion: "1")
        #expect(r.url.path == "/api/vitals/v1x")
    }

    @Test(arguments: ["", "1 2", "1;x", "ü", String(repeating: "1", count: 33), "1\r\nX-Evil: 1"])
    func aBadVersionIsRefused(version: String) {
        #expect(throws: ApiRequestError.badAPIVersion) {
            try ApiRequest.get(base: base, path: "/api/patients", apiVersion: version)
        }
    }

    @Test func aDottedVersionIsAllowed() throws {
        let r = try ApiRequest.get(base: base, path: "/api/patients", apiVersion: "2026-11.1")
        #expect(r.headers["Accepts-version"] == "2026-11.1")
    }

    @Test(arguments: ["http://epr.example.org", "ftp://epr.example.org", "http://10.0.0.1"])
    func plainHttpToAnotherHostIsRefusedEvenWhenLoopbackIsAllowed(url: String) {
        #expect(throws: ApiRequestError.baseURLNotHTTPS) {
            try ApiRequest.get(base: URL(string: url)!, path: "/api/patients", apiVersion: "1", allowLoopbackHTTP: true)
        }
    }

    @Test(arguments: ["http://127.0.0.1:5150", "http://localhost:5150", "http://[::1]:5150"])
    func plainHttpToLoopbackOnlyWhenAllowed(url: String) throws {
        #expect(throws: ApiRequestError.baseURLNotHTTPS) {
            try ApiRequest.get(base: URL(string: url)!, path: "/api/patients", apiVersion: "1")
        }
        let r = try ApiRequest.get(base: URL(string: url)!, path: "/api/patients", apiVersion: "1", allowLoopbackHTTP: true)
        #expect(r.url.absoluteString == "\(url)/api/patients")
    }

    @Test(arguments: [
        "https://epr.example.org/prefix", "https://epr.example.org/?a=1", "https://epr.example.org/#x",
        "https://user:pass@epr.example.org", "https:///nohost",
    ])
    func aBaseThatIsNotABareOriginIsRefused(url: String) {
        #expect(throws: ApiRequestError.baseURLNotOrigin) {
            try ApiRequest.get(base: URL(string: url)!, path: "/api/patients", apiVersion: "1")
        }
    }

    @Test func aTrailingSlashOnTheBaseIsAllowed() throws {
        let r = try ApiRequest.get(base: URL(string: "https://epr.example.org/")!, path: "/api/patients", apiVersion: "1")
        #expect(r.url.absoluteString == "https://epr.example.org/api/patients")
    }
}
