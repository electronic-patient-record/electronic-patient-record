import { describe, expect, it } from "vitest";
import {
  bannerRefusals,
  type ListedRefusal,
  type RefusalLoad,
} from "../../src/lib/treatment-refusals";

const refusal = (
  treatment: string,
  state: ListedRefusal["refusal"]["state"],
  extra: Partial<ListedRefusal> & { identity?: "certain" | "uncertain" } = {},
): ListedRefusal => ({
  id: treatment,
  from_merged_record: extra.from_merged_record ?? false,
  refusal: { treatment, state, identity: extra.identity ?? "certain" },
});

const ok = (
  summary: "recorded" | "all-withdrawn" | "none-recorded",
  refusals: ListedRefusal[],
): RefusalLoad => ({
  status: 200,
  data: { summary: { state: summary }, refusals },
});

describe("bannerRefusals (RF-7)", () => {
  it("names each refusal in the record's words, those that stand first, with what is uncertain", () => {
    const line = bannerRefusals(
      ok("recorded", [
        refusal("a blood transfusion", "doubted"),
        refusal("blood products", "stands"),
        refusal("ventilation", "stands", {
          identity: "uncertain",
          from_merged_record: true,
        }),
        refusal("surgery", "withdrawn"),
        refusal("dialysis", "entered-in-error"),
      ]),
    );
    expect(line.show).toBe(true);
    expect(line.loud).toBe(true);
    expect(line.text).toBe(
      "blood products; ventilation (may be another person's, on a merged record); a blood transfusion (doubted)",
    );
  });

  it("says none recorded, never none, and all withdrawn when all are", () => {
    const none = bannerRefusals(ok("none-recorded", []));
    expect(none.text).toMatch(/none recorded here/);
    expect(none.text).toMatch(/does not mean there are none/);
    expect(none.loud).toBe(false);
    expect(
      bannerRefusals(ok("all-withdrawn", [refusal("x", "withdrawn")])).text,
    ).toBe("all withdrawn");
  });

  it("an unreadable list is louder than an empty one, and a role that may not read sees no line", () => {
    const failed = bannerRefusals({ status: 500, data: null });
    expect(failed.loud).toBe(true);
    expect(failed.text).toMatch(/not known, not as none/);
    expect(bannerRefusals({ status: 0, data: null }).loud).toBe(true);
    expect(bannerRefusals({ status: 403, data: null }).show).toBe(false);
    expect(bannerRefusals(null).text).toMatch(/checking/);
  });
});
