import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "./fixtures";

const RESULT = {
  score: 0.5,
  decision: "review",
  fields: [
    {
      field: "family_name",
      similarity: 1,
      weight: 0.25,
      note: "similarity 1.00",
    },
    {
      field: "sex",
      similarity: null,
      weight: 0.05,
      note: "sex unknown on one or both records",
    },
  ],
  deterministic: null,
  conflicts: ["given name not available to confirm the link"],
};

test("a bare URL gains the locale prefix", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/en-001\/?$/);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Electronic Patient Record",
  );
  await expect(page.getByRole("note")).toContainText("Not for clinical use");
});

test("security headers are set", async ({ page }) => {
  const res = await page.goto("/en-001");
  const h = res!.headers();
  expect(h["x-content-type-options"]).toBe("nosniff");
  expect(h["x-frame-options"]).toBe("DENY");
  expect(h["content-security-policy"]).toContain("default-src 'self'");
});

for (const path of [
  "/en-001",
  "/en-001/patients",
  "/en-001/patients/new",
  "/en-001/patients/compare",
]) {
  test(`no axe violations on ${path}`, async ({ page }) => {
    await page.goto(path);
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag22aa"])
      .analyze();
    expect(results.violations).toEqual([]);
  });
}

test("compare shows the explainable result and 'not available', never zero", async ({
  page,
}) => {
  await page.route("**/api/proxy/api/patients/compare", (route) =>
    route.fulfill({ json: RESULT }),
  );
  await page.goto("/en-001/patients/compare");
  await page.getByLabel("Family name").first().fill("Smith");
  await page.getByRole("button", { name: "Compare" }).click();
  await expect(page.getByRole("heading", { level: 2 })).toContainText(
    "Needs human review",
  );
  await expect(page.getByRole("cell", { name: "not available" })).toBeVisible();
  await expect(
    page.getByText("given name not available to confirm the link"),
  ).toBeVisible();
});

test("a failed comparison is announced", async ({ page }) => {
  await page.route("**/api/proxy/api/patients/compare", (route) =>
    route.fulfill({ status: 500, body: "x" }),
  );
  await page.goto("/en-001/patients/compare");
  await page.getByRole("button", { name: "Compare" }).click();
  await expect(page.getByRole("alert")).toHaveText("The comparison failed.");
});

test("the proxy refuses paths outside /api", async ({ request }) => {
  expect((await request.get("/api/proxy/fhir/R4/Patient")).status()).toBe(404);
  expect((await request.get("/api/proxy/_health")).status()).toBe(404);
});

// CQ-38: the reasons a link cannot be automatic arrive as codes and are shown in the page's language
// (machine-written words, not reviewed); a code the page does not know shows its English text.
const WITH_CODES = {
  ...RESULT,
  conflicts: [
    "different verified NHS numbers",
    "birth date not available to confirm the link",
    "a reason from a newer matcher",
  ],
  conflict_codes: [
    { code: "different_nhs_numbers" },
    { code: "field_unavailable", field: "birth_date" },
    { code: "brand_new_reason" },
  ],
};

for (const [locale, first, second] of [
  [
    "de-001",
    "unterschiedliche geprüfte NHS-Nummern",
    "Geburtsdatum nicht verfügbar, um die Verknüpfung zu bestätigen",
  ],
  [
    "ar-001",
    "أرقام NHS الموثّقة مختلفة",
    "تاريخ الميلاد غير متوفر لتأكيد الربط",
  ],
] as const) {
  test(`compare shows the reasons in ${locale}, and an unknown code in English`, async ({
    page,
  }) => {
    await page.route("**/api/proxy/api/patients/compare", (route) =>
      route.fulfill({ json: WITH_CODES }),
    );
    await page.goto(`/${locale}/patients/compare`);
    await page.getByLabel("Family name").first().fill("Smith");
    await page.getByRole("button", { name: "Compare" }).click();
    await expect(page.getByText(first, { exact: true })).toBeVisible();
    await expect(page.getByText(second, { exact: true })).toBeVisible();
    await expect(
      page.getByText("a reason from a newer matcher", { exact: true }),
    ).toBeVisible();
    await expect(page.getByText("different verified NHS numbers")).toHaveCount(
      0,
    );
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag22aa"])
      .analyze();
    expect(results.violations).toEqual([]);
  });
}

test("registering shows a possible duplicate's reasons in French", async ({
  page,
}) => {
  const patient = {
    id: "11111111-1111-4111-8111-111111111111",
    names: [{ family: "Smith", given: ["Ann"] }],
    birth_date: "1980-03-14",
    sex: "female",
    identifiers: [],
    addresses: [],
    telecom: [],
  };
  await page.route("**/api/proxy/api/patients", (route) =>
    route.fulfill({
      status: 409,
      json: {
        error: "possible duplicate",
        candidates: [
          {
            existing: { id: patient.id, version: 1, author: "x", patient },
            score: 0.9,
            decision: "review",
            fields: [],
            deterministic: null,
            conflicts: ["names sound alike but are spelled differently"],
            conflict_codes: [{ code: "names_sound_alike" }],
          },
        ],
      },
    }),
  );
  await page.goto("/fr-001/patients/new");
  await page.getByLabel("Family name").fill("Smyth");
  await page.getByRole("button", { name: "Register", exact: true }).click();
  await expect(
    page.getByText(
      "les noms se prononcent de façon proche mais s'écrivent différemment",
    ),
  ).toBeVisible();
});
