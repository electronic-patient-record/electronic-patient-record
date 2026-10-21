import { describe, expect, it } from "vitest";
import {
  describeConsent,
  describeProvision,
  emptyConsentForm,
  formToConsent,
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
});
