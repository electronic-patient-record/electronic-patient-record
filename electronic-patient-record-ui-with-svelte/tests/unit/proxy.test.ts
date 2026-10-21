import { describe, it, expect } from "vitest";
import { isBackground, proxyTarget, purposeHeader } from "../../src/lib/proxy";

describe("proxyTarget", () => {
  it("forwards API paths", () => {
    expect(proxyTarget("http://x:1/", "api/patients/compare", "?a=1")).toBe(
      "http://x:1/api/patients/compare?a=1",
    );
  });
  it("refuses anything outside /api", () => {
    expect(proxyTarget("http://x", "fhir/R4/Patient", "")).toBeNull();
    expect(proxyTarget("http://x", "_health", "")).toBeNull();
    expect(proxyTarget("http://x", "", "")).toBeNull();
  });
  it("refuses traversal and odd segments", () => {
    for (const p of [
      "api/../_health",
      "api//x",
      "api/./x",
      "api/%2e%2e/x",
      "api\\x",
    ]) {
      expect(proxyTarget("http://x", p, "")).toBeNull();
    }
  });
});

describe("purposeHeader", () => {
  it("passes a known purpose of use and drops anything else", () => {
    expect(purposeHeader("patient-access")).toBe("patient-access");
    expect(purposeHeader("treatment")).toBe("treatment");
    for (const bad of [
      "",
      null,
      undefined,
      "Treatment",
      "treatment, research",
      "x\r\nHost: evil",
      "fun",
    ]) {
      expect(purposeHeader(bad)).toBeNull();
    }
  });
});

describe("isBackground (CQ-8)", () => {
  it("is true only for the exact marker", () => {
    expect(isBackground("1")).toBe(true);
    for (const v of [null, undefined, "", "0", "true", "yes"])
      expect(isBackground(v)).toBe(false);
  });
});
