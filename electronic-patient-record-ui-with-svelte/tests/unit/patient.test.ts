import { describe, expect, it } from "vitest";
import {
  NHS_NUMBER_SYSTEM,
  comparisonRows,
  emptyForm,
  formToPatient,
  isRestricted,
  nhsDigits,
  patientToForm,
  searchQuery,
  type PatientRecord,
} from "../../src/lib/patient";
import { commonProblem } from "../../src/lib/api";

const rich: PatientRecord = {
  id: "abc",
  names: [
    { family: "Smith", given: ["Ann", "Marie"] },
    { family: "Jones", given: ["Ann"] },
  ],
  birth_date: "1980-03-14",
  sex: "female",
  identifiers: [
    { system: "urn:example:mrn", value: "M-1" },
    { system: NHS_NUMBER_SYSTEM, value: "9434765919" },
  ],
  addresses: [{ postal_code: "AB1 2CD" }, { postal_code: "ZZ9 9ZZ" }],
  telecom: [{ system: "email", value: "ann@example.org" }],
};

describe("patientToForm / formToPatient", () => {
  it("round-trips the fields the form owns", () => {
    const form = patientToForm(rich);
    expect(form).toEqual({
      family: "Smith",
      given: "Ann Marie",
      birthDate: "1980-03-14",
      sex: "female",
      nhsNumber: "9434765919",
      postalCode: "AB1 2CD",
    });
    expect(formToPatient(form, rich)).toEqual({ ...rich, id: undefined });
  });

  it("keeps what the form does not show when editing", () => {
    const form = { ...patientToForm(rich), family: "Smythe" };
    const out = formToPatient(form, rich);
    expect(out.names[0]).toEqual({ family: "Smythe", given: ["Ann", "Marie"] });
    expect(out.names[1]).toEqual({ family: "Jones", given: ["Ann"] });
    expect(out.identifiers).toContainEqual({
      system: "urn:example:mrn",
      value: "M-1",
    });
    expect(out.addresses[1]).toEqual({ postal_code: "ZZ9 9ZZ" });
    expect(out.telecom).toEqual(rich.telecom);
  });

  it("never sends an id", () => {
    expect(
      "id" in formToPatient(patientToForm(rich), rich) &&
        formToPatient(patientToForm(rich), rich).id,
    ).toBeFalsy();
  });

  it("sends blank fields as null, not empty strings", () => {
    const out = formToPatient({ ...emptyForm(), family: "  Smith " });
    expect(out.names).toEqual([{ family: "Smith", given: [] }]);
    expect(out.birth_date).toBeNull();
    expect(out.sex).toBeNull();
    expect(out.identifiers).toEqual([]);
    expect(out.addresses).toEqual([]);
  });

  it("clearing the NHS number removes it and keeps other identifiers", () => {
    const out = formToPatient({ ...patientToForm(rich), nhsNumber: "" }, rich);
    expect(out.identifiers).toEqual([
      { system: "urn:example:mrn", value: "M-1" },
    ]);
  });

  it("clearing the postcode blanks it without deleting the address", () => {
    const out = formToPatient({ ...patientToForm(rich), postalCode: "" }, rich);
    expect(out.addresses[0]).toEqual({ postal_code: null });
    expect(out.addresses).toHaveLength(2);
  });

  it("strips spaces and hyphens from an NHS number as written", () => {
    expect(nhsDigits("943 476 5919")).toBe("9434765919");
    expect(
      formToPatient({ ...emptyForm(), nhsNumber: "943-476-5919" }).identifiers,
    ).toEqual([{ system: NHS_NUMBER_SYSTEM, value: "9434765919" }]);
  });

  it("splits given names on any whitespace", () => {
    expect(
      formToPatient({ ...emptyForm(), given: " Ann   Marie " }).names[0]?.given,
    ).toEqual(["Ann", "Marie"]);
  });
});

describe("searchQuery", () => {
  it("is empty when nothing was entered, so no empty search is sent", () => {
    expect(searchQuery(emptyForm())).toBe("");
    expect(searchQuery({ ...emptyForm(), family: "   ", given: "Ann" })).toBe(
      "",
    );
  });
  it("encodes the family name, birth date and NHS identifier", () => {
    const q = new URLSearchParams(
      searchQuery({
        ...emptyForm(),
        family: "O'Neil & Co",
        birthDate: "1980",
        nhsNumber: "943 476 5919",
      }),
    );
    expect(q.get("family")).toBe("O'Neil & Co");
    expect(q.get("birth_date")).toBe("1980");
    expect(q.get("identifier")).toBe(`${NHS_NUMBER_SYSTEM}|9434765919`);
  });
});

describe("comparisonRows", () => {
  it("marks equal, non-empty values as the same, ignoring case and NHS spacing", () => {
    const rows = Object.fromEntries(
      comparisonRows(
        {
          ...emptyForm(),
          family: "smith",
          nhsNumber: "943 476 5919",
          given: "Bob",
        },
        rich,
      ).map((r) => [r.field, r]),
    );
    expect(rows.family?.same).toBe(true);
    expect(rows.nhsNumber?.same).toBe(true);
    expect(rows.given?.same).toBe(false);
  });
  it("never calls two missing values the same", () => {
    const rows = comparisonRows(emptyForm(), {
      names: [],
      identifiers: [],
      addresses: [],
      telecom: [],
    });
    expect(rows.every((r) => !r.same)).toBe(true);
  });
});

describe("commonProblem", () => {
  it("maps the statuses every page treats alike", () => {
    expect(commonProblem(401)).toBe("signInRequired");
    expect(commonProblem(403)).toBe("forbidden");
    expect(commonProblem(0)).toBe("error");
    expect(commonProblem(502)).toBe("error");
    expect(commonProblem(404)).toBeNull();
    expect(commonProblem(409)).toBeNull();
    expect(commonProblem(200)).toBeNull();
  });
});

describe("isRestricted", () => {
  it("tells a stand-in from a full candidate", () => {
    expect(isRestricted({ restricted: true })).toBe(true);
    expect(
      isRestricted({
        existing: { id: "x", version: 1, author: "a", patient: rich },
        score: 1,
        decision: "auto_link",
        fields: [],
        deterministic: null,
        conflicts: [],
      }),
    ).toBe(false);
  });
});
