import { describe, expect, it } from "vitest";
import {
  describeConsent,
  describeProvision,
  describeProxy,
  emptyConsentForm,
  formToConsent,
  proxyTerms,
  relationshipRecord,
} from "../../src/lib/access";

describe("access and consent requests", () => {
  it("builds a relationship record with optional dates", () => {
    expect(relationshipRecord(" sub-1 ", "care-team", "", "")).toEqual({
      subject: "sub-1",
      kind: "care-team",
      status: "active",
      period: null,
    });
    expect(relationshipRecord("s", "proxy", "2026-01-01", "")).toMatchObject({
      period: { start: "2026-01-01" },
    });
    expect(relationshipRecord("s", "proxy", "", "2026-12-31").period).toEqual({
      end: "2026-12-31",
    });
  });

  it("builds a deny with the chosen roles and nothing else restricted", () => {
    const f = emptyConsentForm();
    f.actors = ["receptionist"];
    const c = formToConsent(f);
    expect(c.kind).toBe("general");
    expect(c.status).toBe("active");
    expect(c.provisions).toEqual([
      {
        effect: "deny",
        actors: ["receptionist"],
        purposes: [],
        resources: [],
        labels: [],
        period: null,
      },
    ]);
  });

  it("builds the opt-out with no provisions of its own, whatever else is ticked", () => {
    const f = emptyConsentForm();
    f.nationalOptOut = true;
    f.actors = ["clinician"];
    expect(formToConsent(f)).toEqual({
      kind: "national-data-opt-out",
      status: "active",
      provisions: [],
    });
  });

  it("describes a provision in words, with 'any' where nothing is chosen", () => {
    expect(
      describeProvision({
        effect: "deny",
        actors: [],
        purposes: [],
        resources: [],
        labels: [],
      }),
    ).toBe("Deny: any role · for any purpose · any record · any label");
    expect(
      describeProvision({
        effect: "permit",
        actors: ["clinician", "nurse"],
        purposes: ["treatment"],
        resources: ["AllergyIntolerance"],
        labels: ["SUD"],
        period: { start: "2026-01-01" },
      }),
    ).toBe(
      "Permit: clinician, nurse · for treatment · AllergyIntolerance · SUD · 2026-01-01 to …",
    );
    expect(
      describeConsent({
        kind: "national-data-opt-out",
        status: "active",
        provisions: [],
      }),
    ).toContain("National data opt-out");
    expect(
      describeConsent({ kind: "general", status: "active", provisions: [] }),
    ).toBe("No provisions");
  });

  it("builds proxy terms that end at the start of the day after the last day", () => {
    expect(proxyTerms("deputyship", ["identity"], "2026-12-31")).toEqual({
      basis: "deputyship",
      scopes: ["identity"],
      expires_at: Date.UTC(2027, 0, 1) / 1000,
    });
    for (const bad of ["", "tomorrow", "2026-02-30", "2026-13-01", "26-01-01"])
      expect(proxyTerms("deputyship", ["identity"], bad), bad).toBeNull();
  });

  it("sends terms for a proxy only, and never invents them", () => {
    const t = proxyTerms("patient-request", ["photo"], "2026-12-31");
    expect(relationshipRecord("s", "proxy", "", "", t).proxy).toEqual(t);
    expect(relationshipRecord("s", "care-team", "", "", t)).not.toHaveProperty(
      "proxy",
    );
    expect(relationshipRecord("s", "proxy", "", "", null)).not.toHaveProperty(
      "proxy",
    );
  });

  it("describes a proxy by its scopes and last day, and says plainly when it has no terms", () => {
    const t = proxyTerms(
      "patient-request",
      ["identity", "photo"],
      "2026-12-31",
    );
    expect(describeProxy(t)).toBe(
      "Name, birth date and identifiers, Photo · until the end of 2026-12-31",
    );
    expect(describeProxy(null)).toContain("sees nothing");
  });
});
