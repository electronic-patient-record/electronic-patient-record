package org.electronicpatientrecord.core

import java.net.URI
import java.net.URISyntaxException

/** Why a request was refused before it was made. */
public enum class ApiRequestError {
    /** The base URL is not `https`, and is not plain `http` to a loopback host where that is allowed. */
    BASE_URL_NOT_HTTPS,

    /** The base URL carries a path, query, fragment or user information: it must be a bare origin. */
    BASE_URL_NOT_ORIGIN,

    /** The path does not start with `/api/`. */
    PATH_OUTSIDE_API,

    /** A path segment is empty, `.` or `..`, or holds `%`, `\`, `?` or `#`. */
    BAD_PATH_SEGMENT,

    /** The path carries a version segment such as `/api/v1`: the version goes in the header. */
    VERSION_IN_PATH,

    /** The API version is not a short token of letters, digits, dots and hyphens. */
    BAD_API_VERSION,
}

/** A request refused before it was made, and why. */
public class ApiRequestException(public val error: ApiRequestError) : IllegalArgumentException(error.name)

/**
 * A request to the service, checked before it is made (spec/mobile-apps/index.md §2).
 *
 * The app calls the unversioned `/api` path directly and names the API version in the `Accepts-version` header, never in
 * the path (spec/09-api-surface.md). The path rule is the web UI's proxy rule.
 */
public data class ApiRequest(
    /** The full URL. */
    val url: String,
    /** The HTTP method. */
    val method: String,
    /** The headers to send. Never a token: the caller adds `Authorization` at the moment of sending. */
    val headers: Map<String, String>,
) {
    public companion object {
        /** The header that carries the API version. */
        public const val VERSION_HEADER: String = "Accepts-version"

        /** The longest API version token accepted. */
        public const val MAX_VERSION_LENGTH: Int = 32

        /**
         * A `GET` for [path] under [base].
         *
         * @param base the service's origin, such as `https://epr.example.org`.
         * @param path an absolute path that starts with `/api/`, with no query.
         * @param query query items, each name and value percent-encoded (RFC 3986 unreserved characters kept).
         * @param apiVersion the value of `Accepts-version`.
         * @param allowLoopbackHttp accept plain `http` to `127.0.0.1`, `::1` or `localhost`. Only a debug build passes `true`.
         * @throws ApiRequestException when any rule is broken.
         */
        public fun get(
            base: String,
            path: String,
            query: List<Pair<String, String>> = emptyList(),
            apiVersion: String,
            allowLoopbackHttp: Boolean = false,
        ): ApiRequest {
            val origin = checkBase(base, allowLoopbackHttp)
            checkPath(path)
            checkVersion(apiVersion)
            val encoded = query.joinToString("&") { (name, value) -> "${encode(name)}=${encode(value)}" }
            val url = origin + path + if (encoded.isEmpty()) "" else "?$encoded"
            return ApiRequest(
                url = url,
                method = "GET",
                headers = mapOf("Accept" to "application/json", VERSION_HEADER to apiVersion),
            )
        }

        /** Whether [host] is a loopback name or address. */
        public fun isLoopback(host: String): Boolean =
            host.lowercase() in setOf("127.0.0.1", "::1", "[::1]", "localhost")

        private fun refuse(error: ApiRequestError): Nothing = throw ApiRequestException(error)

        /** The origin, `scheme://authority`, with no trailing slash. */
        private fun checkBase(base: String, allowLoopbackHttp: Boolean): String {
            val uri = try {
                URI(base)
            } catch (_: URISyntaxException) {
                refuse(ApiRequestError.BASE_URL_NOT_ORIGIN)
            }
            val host = uri.host
            if (host.isNullOrEmpty()) refuse(ApiRequestError.BASE_URL_NOT_ORIGIN)
            when (uri.scheme?.lowercase()) {
                "https" -> Unit
                "http" -> if (!(allowLoopbackHttp && isLoopback(host))) refuse(ApiRequestError.BASE_URL_NOT_HTTPS)
                else -> refuse(ApiRequestError.BASE_URL_NOT_HTTPS)
            }
            val path = uri.rawPath.orEmpty()
            if ((path.isNotEmpty() && path != "/") || uri.rawQuery != null || uri.rawFragment != null ||
                uri.rawUserInfo != null
            ) {
                refuse(ApiRequestError.BASE_URL_NOT_ORIGIN)
            }
            return "${uri.scheme.lowercase()}://${uri.rawAuthority}"
        }

        private fun checkPath(path: String) {
            if (!path.startsWith("/api/")) refuse(ApiRequestError.PATH_OUTSIDE_API)
            val segments = path.substring(1).split("/")
            for (segment in segments) {
                if (segment.isEmpty() || segment == "." || segment == "..") refuse(ApiRequestError.BAD_PATH_SEGMENT)
                if (segment.any { it in "%\\?#" }) refuse(ApiRequestError.BAD_PATH_SEGMENT)
            }
            if (segments.size > 1 && isVersionSegment(segments[1])) refuse(ApiRequestError.VERSION_IN_PATH)
        }

        private fun isVersionSegment(segment: String): Boolean =
            segment.length > 1 && (segment[0] == 'v' || segment[0] == 'V') &&
                segment.drop(1).all { it in '0'..'9' }

        private fun checkVersion(version: String) {
            val ok = version.isNotEmpty() && version.length <= MAX_VERSION_LENGTH &&
                version.all { it in 'a'..'z' || it in 'A'..'Z' || it in '0'..'9' || it == '.' || it == '-' }
            if (!ok) refuse(ApiRequestError.BAD_API_VERSION)
        }

        private fun encode(text: String): String = buildString {
            for (byte in text.encodeToByteArray()) {
                val c = byte.toInt() and 0xff
                val ch = c.toChar()
                if (ch in 'a'..'z' || ch in 'A'..'Z' || ch in '0'..'9' || ch in "-._~") {
                    append(ch)
                } else {
                    append('%').append("0123456789ABCDEF"[c shr 4]).append("0123456789ABCDEF"[c and 0xf])
                }
            }
        }
    }
}
