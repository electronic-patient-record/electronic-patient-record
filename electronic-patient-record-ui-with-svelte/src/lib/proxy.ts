// The BFF proxy's path rules, pure so they can be unit-tested. The browser
// reaches the API only through `/api/proxy/<path>`; the server decides what
// that path may be.

/** Returns the upstream URL for a proxied path, or null if the path is refused. */
export function proxyTarget(
  base: string,
  path: string,
  search: string,
): string | null {
  const segments = path.split("/");
  if (segments[0] !== "api") return null; // only the REST namespace, never /fhir or /_health
  if (
    segments.some(
      (s) =>
        s === "" ||
        s === "." ||
        s === ".." ||
        s.includes("\\") ||
        s.includes("%"),
    )
  ) {
    return null;
  }
  return `${base.replace(/\/+$/, "")}/${segments.join("/")}${search}`;
}

/**
 * The header a page sets on a background look (a poll nobody asked for). The BFF does not count such a
 * request as activity, and does not forward the header to the API.
 */
export const BACKGROUND_HEADER = "x-epr-background";

export function isBackground(value: string | null | undefined): boolean {
  return value === "1";
}

/** The purposes of use the API accepts in `X-Purpose-Of-Use` (service `Purpose::parse`). */
export const PURPOSES = [
  "treatment",
  "emergency",
  "operations",
  "payment",
  "research",
  "patient-access",
  "audit",
] as const;

/**
 * The purpose of use to forward, or null to send none (the API then assumes `operations`). A value
 * that is not on the list is dropped here rather than passed on: the browser does not get to put
 * arbitrary text in a header the API records in the audit trail.
 */
export function purposeHeader(value: string | null | undefined): string | null {
  return value && (PURPOSES as readonly string[]).includes(value)
    ? value
    : null;
}

/**
 * The `Accepts-version` value to forward, or null. The API is versioned by this header, never by the
 * path; only a short token of letters, digits, dots and hyphens passes, so nothing odd reaches the API.
 */
export function versionHeader(value: string | null | undefined): string | null {
  return value && /^[A-Za-z0-9.-]{1,32}$/.test(value) ? value : null;
}
