import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const RESULT = {
  score: 0.5,
  decision: "review",
  fields: [
    { field: "family_name", similarity: 1, weight: 0.25, note: "similarity 1.00" },
    { field: "sex", similarity: null, weight: 0.05, note: "sex unknown on one or both records" },
  ],
  deterministic: null,
  conflicts: ["given name not available to confirm the link"],
};

test("a bare URL gains the locale prefix", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/en-001\/?$/);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Electronic Patient Record");
  await expect(page.getByRole("note")).toContainText("Not for clinical use");
});

test("security headers are set", async ({ page }) => {
  const res = await page.goto("/en-001");
  const h = res!.headers();
  expect(h["x-content-type-options"]).toBe("nosniff");
  expect(h["x-frame-options"]).toBe("DENY");
  expect(h["content-security-policy"]).toContain("default-src 'self'");
});

for (const path of ["/en-001", "/en-001/patients"]) {
  test(`no axe violations on ${path}`, async ({ page }) => {
    await page.goto(path);
    const results = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag22aa"]).analyze();
    expect(results.violations).toEqual([]);
  });
}

test("compare shows the explainable result and 'not available', never zero", async ({ page }) => {
  await page.route("**/api/proxy/api/patients/compare", (route) => route.fulfill({ json: RESULT }));
  await page.goto("/en-001/patients");
  await page.getByLabel("Family name").first().fill("Smith");
  await page.getByRole("button", { name: "Compare" }).click();
  await expect(page.getByRole("heading", { level: 2 })).toContainText("Needs human review");
  await expect(page.getByRole("cell", { name: "not available" })).toBeVisible();
  await expect(page.getByText("given name not available to confirm the link")).toBeVisible();
});

test("a failed comparison is announced", async ({ page }) => {
  await page.route("**/api/proxy/api/patients/compare", (route) => route.fulfill({ status: 500, body: "x" }));
  await page.goto("/en-001/patients");
  await page.getByRole("button", { name: "Compare" }).click();
  await expect(page.getByRole("alert")).toHaveText("The comparison failed.");
});

test("the proxy refuses paths outside /api", async ({ request }) => {
  expect((await request.get("/api/proxy/fhir/R4/Patient")).status()).toBe(404);
  expect((await request.get("/api/proxy/_health")).status()).toBe(404);
});
