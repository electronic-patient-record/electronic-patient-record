import { render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import PatientBanner from "../../src/lib/PatientBanner.svelte";
import type { DeathLoad } from "../../src/lib/death";
import type { Stored } from "../../src/lib/patient";
import { STRINGS } from "../../src/lib/strings";

vi.mock("../../src/lib/api.js", async (real) => ({
  ...(await real<typeof import("../../src/lib/api")>()),
  fetchBlob: vi.fn(async () => ({ status: 500, body: null })),
}));

const stored = (over: Partial<Stored> = {}): Stored =>
  ({
    id: "p1",
    version: 1,
    author: "a",
    site_time_zone: "UTC",
    patient: {
      names: [{ family: "Rivers", given: ["Morgan"] }],
      birth_date: "1980-02-14",
      identifiers: [],
    },
    ...over,
  }) as unknown as Stored;

const today = new Date("2026-11-05T12:00:00Z");
const show = (props: Record<string, unknown> = {}) =>
  render(PatientBanner, { stored: stored(), today, ...props });

const asserted = (confirmed: boolean): DeathLoad => ({
  status: 200,
  answer: {
    answer: "asserted",
    status: confirmed ? "verified" : "reported",
    confirmed,
    occurred: "2026-10-12",
    dates_differ: false,
    holds_contact: true,
  },
});

describe("PatientBanner (CQ-35, ID-7)", () => {
  it("says who this is: family name in capitals, given name", () => {
    show();
    expect(screen.getByText(/RIVERS/)).toBeTruthy();
    expect(screen.getByText(/Morgan/)).toBeTruthy();
  });

  it("never reads as no allergies while they are unknown", () => {
    const { container } = show({ allergies: null });
    expect(container.textContent).not.toMatch(/no known allerg/i);
  });

  it("shows a report of death as an alert, before it is confirmed", () => {
    show({ death: asserted(false) });
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain(STRINGS.bannerDeathReported);
  });

  it("says when a look for a death failed, rather than saying nothing", () => {
    show({ death: { status: 500, answer: null } });
    expect(screen.getByRole("status").textContent).toBe(
      STRINGS.bannerDeathUnknown,
    );
  });

  it("says nothing about a death when none is reported, and never says alive", () => {
    const { container } = show({
      death: {
        status: 200,
        answer: { answer: "not-known-to-have-died", holds_contact: false },
      },
    });
    expect(screen.queryByRole("alert")).toBeNull();
    expect(container.textContent).not.toMatch(/alive|living/i);
  });

  it("marks a drill record and a merged record", () => {
    render(PatientBanner, {
      stored: stored({ drill: true, replaced_by: "p2" } as Partial<Stored>),
      today,
    });
    const alerts = screen.getAllByRole("alert").map((a) => a.textContent);
    expect(alerts).toContain(STRINGS.bannerDrill);
    expect(alerts.join()).toContain(STRINGS.bannerMerged.slice(0, 12));
  });
});
