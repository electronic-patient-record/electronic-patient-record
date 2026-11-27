// A list's next page, from the API's `Link: <...>; rel="next"` (patient search, `axa-cursor-pagination`). Only a
// same-origin path under `/api/` is a next page: anything else (a host, a scheme, a `//`, a backslash) is dropped, so
// neither the proxy nor the browser can be sent somewhere else by a header.

const NEXT = /^<(\/api\/[^<>\s]*)>\s*;\s*rel="next"$/;

/** The path of the next page in a `Link` header, or null when there is none or it is not a path under /api/. */
export function nextPath(link: string | null | undefined): string | null {
  const m = link?.trim().match(NEXT);
  if (!m) return null;
  const path = m[1]!;
  if (path.includes("//") || path.includes("\\") || path.includes(".."))
    return null;
  return path;
}
