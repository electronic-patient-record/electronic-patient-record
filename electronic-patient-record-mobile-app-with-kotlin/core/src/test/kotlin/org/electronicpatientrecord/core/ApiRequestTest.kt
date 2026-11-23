package org.electronicpatientrecord.core

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

private const val BASE = "https://epr.example.org"

class ApiRequestTest {
    private fun refused(error: ApiRequestError, block: () -> Unit) =
        assertEquals(error, assertFailsWith<ApiRequestException> { block() }.error)

    @Test fun aGetUnderApiCarriesTheVersionHeader() {
        val r = ApiRequest.get(BASE, "/api/patients", apiVersion = "1")
        assertEquals("https://epr.example.org/api/patients", r.url)
        assertEquals("GET", r.method)
        assertEquals("1", r.headers["Accepts-version"])
        assertEquals("application/json", r.headers["Accept"])
        assertNull(r.headers["Authorization"])
    }

    @Test fun queryItemsAreEncoded() {
        val r = ApiRequest.get(BASE, "/api/patients", listOf("family" to "O'Brien Smith", "x" to "a+b&c=ü"), apiVersion = "1")
        assertEquals("https://epr.example.org/api/patients?family=O%27Brien%20Smith&x=a%2Bb%26c%3D%C3%BC", r.url)
    }

    @Test fun aPathOutsideApiIsRefused() {
        for (path in listOf("/patients", "/fhir/R4/Patient", "/apix/patients", "api/patients", "/api", "")) {
            refused(ApiRequestError.PATH_OUTSIDE_API) { ApiRequest.get(BASE, path, apiVersion = "1") }
        }
    }

    @Test fun aBadSegmentIsRefused() {
        val paths = listOf(
            "/api/", "/api//patients", "/api/./patients", "/api/../admin", "/api/patients/%2e%2e",
            "/api/patients\\x", "/api/patients?family=x", "/api/patients#top",
        )
        for (path in paths) refused(ApiRequestError.BAD_PATH_SEGMENT) { ApiRequest.get(BASE, path, apiVersion = "1") }
    }

    @Test fun aVersionSegmentIsRefused() {
        for (path in listOf("/api/v1/patients", "/api/v2", "/api/V10/patients")) {
            refused(ApiRequestError.VERSION_IN_PATH) { ApiRequest.get(BASE, path, apiVersion = "1") }
        }
    }

    @Test fun aSegmentThatOnlyStartsWithVIsAllowed() {
        assertEquals("$BASE/api/vitals/v1x", ApiRequest.get(BASE, "/api/vitals/v1x", apiVersion = "1").url)
    }

    @Test fun aBadVersionIsRefused() {
        for (v in listOf("", "1 2", "1;x", "ü", "1".repeat(33), "1\r\nX-Evil: 1")) {
            refused(ApiRequestError.BAD_API_VERSION) { ApiRequest.get(BASE, "/api/patients", apiVersion = v) }
        }
    }

    @Test fun aDottedVersionIsAllowed() {
        assertEquals("2026-11.1", ApiRequest.get(BASE, "/api/patients", apiVersion = "2026-11.1").headers["Accepts-version"])
    }

    @Test fun plainHttpToAnotherHostIsRefusedEvenWhenLoopbackIsAllowed() {
        for (url in listOf("http://epr.example.org", "ftp://epr.example.org", "http://10.0.0.1")) {
            refused(ApiRequestError.BASE_URL_NOT_HTTPS) {
                ApiRequest.get(url, "/api/patients", apiVersion = "1", allowLoopbackHttp = true)
            }
        }
    }

    @Test fun plainHttpToLoopbackOnlyWhenAllowed() {
        for (url in listOf("http://127.0.0.1:5150", "http://localhost:5150", "http://[::1]:5150")) {
            refused(ApiRequestError.BASE_URL_NOT_HTTPS) { ApiRequest.get(url, "/api/patients", apiVersion = "1") }
            assertEquals(
                "$url/api/patients",
                ApiRequest.get(url, "/api/patients", apiVersion = "1", allowLoopbackHttp = true).url,
            )
        }
    }

    @Test fun aBaseThatIsNotABareOriginIsRefused() {
        val bases = listOf(
            "https://epr.example.org/prefix", "https://epr.example.org/?a=1", "https://epr.example.org/#x",
            "https://user:pass@epr.example.org", "https:///nohost", "not a url",
        )
        for (base in bases) refused(ApiRequestError.BASE_URL_NOT_ORIGIN) { ApiRequest.get(base, "/api/patients", apiVersion = "1") }
    }

    @Test fun aTrailingSlashOnTheBaseIsAllowed() {
        assertEquals("$BASE/api/patients", ApiRequest.get("$BASE/", "/api/patients", apiVersion = "1").url)
    }
}
