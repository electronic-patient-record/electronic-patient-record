import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

// The development realm (`operations/keycloak/epr-realm.json`) is imported by Keycloak at start-up. Keycloak
// stores a role's or client's description in a 255-character column, so a longer one makes a fresh import
// fail and Keycloak not start (found 2026-11-10: a 272-character role description). An existing database
// imported before the text grew hides it.
const realm: unknown = JSON.parse(
  readFileSync(
    resolve(process.cwd(), "../operations/keycloak/epr-realm.json"),
    "utf8",
  ),
);

function descriptions(o: unknown, path = ""): [string, string][] {
  if (Array.isArray(o))
    return o.flatMap((x, i) => descriptions(x, `${path}[${i}]`));
  if (o && typeof o === "object")
    return Object.entries(o).flatMap(([k, v]) =>
      k === "description" && typeof v === "string"
        ? [[path, v] as [string, string]]
        : descriptions(v, `${path}/${k}`),
    );
  return [];
}

describe("development Keycloak realm", () => {
  it("has descriptions, and every one fits Keycloak's 255-character column", () => {
    const all = descriptions(realm);
    expect(all.length).toBeGreaterThan(0);
    const long = all
      .filter(([, d]) => d.length > 255)
      .map(([p, d]) => `${p}: ${d.length}`);
    expect(long).toEqual([]);
  });
});
