import Foundation

/// Why a request was refused before it was made.
public enum ApiRequestError: Error, Equatable, Sendable {
    /// The base URL is not `https`, and is not plain `http` to a loopback host where that is allowed.
    case baseURLNotHTTPS
    /// The base URL carries a path, query, fragment or user information: it must be a bare origin.
    case baseURLNotOrigin
    /// The path does not start with `/api/`.
    case pathOutsideAPI
    /// A path segment is empty, `.` or `..`, or holds `%`, `\`, `?` or `#`.
    case badPathSegment
    /// The path carries a version segment such as `/api/v1`: the version goes in the header.
    case versionInPath
    /// The API version is not a short token of letters, digits, dots and hyphens.
    case badAPIVersion
}

/// A request to the service, checked before it is made (spec/mobile-apps/index.md §2).
///
/// The app calls the unversioned `/api` path directly and names the API version in the `Accepts-version` header,
/// never in the path (spec/09-api-surface.md). The path rule is the web UI's proxy rule.
public struct ApiRequest: Equatable, Sendable {
    /// The header that carries the API version.
    public static let versionHeader = "Accepts-version"
    /// The longest API version token accepted.
    public static let maxVersionLength = 32

    /// The full URL.
    public let url: URL
    /// The HTTP method.
    public let method: String
    /// The headers to send. Never a token: the caller adds `Authorization` at the moment of sending.
    public let headers: [String: String]

    /// A `GET` for `path` under `base`.
    ///
    /// - Parameters:
    ///   - base: the service's origin, such as `https://epr.example.org`.
    ///   - path: an absolute path that starts with `/api/`, with no query.
    ///   - query: query items, encoded by `URLComponents`.
    ///   - apiVersion: the value of `Accepts-version`.
    ///   - allowLoopbackHTTP: accept plain `http` to `127.0.0.1`, `::1` or `localhost`. Only a debug build passes `true`.
    public static func get(
        base: URL,
        path: String,
        query: [URLQueryItem] = [],
        apiVersion: String,
        allowLoopbackHTTP: Bool = false
    ) throws(ApiRequestError) -> ApiRequest {
        try checkBase(base, allowLoopbackHTTP: allowLoopbackHTTP)
        try checkPath(path)
        try checkVersion(apiVersion)
        guard var parts = URLComponents(url: base, resolvingAgainstBaseURL: false) else {
            throw .baseURLNotOrigin
        }
        parts.percentEncodedPath = path
        parts.queryItems = query.isEmpty ? nil : query
        guard let url = parts.url else { throw .badPathSegment }
        return ApiRequest(
            url: url,
            method: "GET",
            headers: ["Accept": "application/json", versionHeader: apiVersion]
        )
    }

    /// Whether `host` is a loopback name or address.
    public static func isLoopback(_ host: String) -> Bool {
        ["127.0.0.1", "::1", "[::1]", "localhost"].contains(host.lowercased())
    }

    private static func checkBase(_ base: URL, allowLoopbackHTTP: Bool) throws(ApiRequestError) {
        guard let parts = URLComponents(url: base, resolvingAgainstBaseURL: false),
              let host = parts.host, !host.isEmpty
        else { throw .baseURLNotOrigin }
        switch parts.scheme?.lowercased() {
        case "https":
            break
        case "http" where allowLoopbackHTTP && isLoopback(host):
            break
        default:
            throw .baseURLNotHTTPS
        }
        let pathIsEmpty = parts.percentEncodedPath.isEmpty || parts.percentEncodedPath == "/"
        guard pathIsEmpty, parts.percentEncodedQuery == nil, parts.percentEncodedFragment == nil,
              parts.user == nil, parts.password == nil
        else { throw .baseURLNotOrigin }
    }

    private static func checkPath(_ path: String) throws(ApiRequestError) {
        guard path.hasPrefix("/api/") else { throw .pathOutsideAPI }
        let segments = path.dropFirst().split(separator: "/", omittingEmptySubsequences: false)
        for segment in segments {
            if segment.isEmpty || segment == "." || segment == ".." { throw .badPathSegment }
            if segment.contains(where: { "%\\?#".contains($0) }) { throw .badPathSegment }
        }
        if segments.count > 1, isVersionSegment(segments[1]) { throw .versionInPath }
    }

    private static func isVersionSegment(_ segment: Substring) -> Bool {
        guard let first = segment.first, first == "v" || first == "V", segment.count > 1 else { return false }
        return segment.dropFirst().allSatisfy { $0.isASCII && $0.isNumber }
    }

    private static func checkVersion(_ version: String) throws(ApiRequestError) {
        guard !version.isEmpty, version.count <= maxVersionLength,
              version.allSatisfy({ $0.isASCII && ($0.isLetter || $0.isNumber || $0 == "." || $0 == "-") })
        else { throw .badAPIVersion }
    }
}
