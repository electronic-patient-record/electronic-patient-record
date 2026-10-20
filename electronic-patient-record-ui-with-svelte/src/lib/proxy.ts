// The BFF proxy's path rules, pure so they can be unit-tested. The browser
// reaches the API only through `/api/proxy/<path>`; the server decides what
// that path may be.

/** Returns the upstream URL for a proxied path, or null if the path is refused. */
export function proxyTarget(base: string, path: string, search: string): string | null {
  const segments = path.split("/");
  if (segments[0] !== "api") return null; // only the REST namespace, never /fhir or /_health
  if (segments.some((s) => s === "" || s === "." || s === ".." || s.includes("\\") || s.includes("%"))) {
    return null;
  }
  return `${base.replace(/\/+$/, "")}/${segments.join("/")}${search}`;
}
