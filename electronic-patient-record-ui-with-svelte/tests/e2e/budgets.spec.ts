import type { Locator, Page, Route } from "@playwright/test";
import { expect, test } from "./fixtures";
import budgets from "./budgets.json" with { type: "json" };

// AS-5: how many things a clinician must do for the critical tasks. Every action goes through `user`,
// which counts it: a click, a text box typed into, a list chosen from. Reaching a field and tabbing
// are not counted, and nor is time: with every API call stubbed a time would measure the stub.
const json = (route: Route, status: number, body: unknown) => route.fulfill({ status, json: body });

function user() {
  let actions = 0;
  return {
    get actions() {
      return actions;
    },
    click: (l: Locator) => (actions++, l.click()),
    type: (l: Locator, text: string) => (actions++, l.fill(text)),
    choose: (l: Locator, value: string) => (actions++, l.selectOption(value)),
  };
}
function within(name: keyof typeof budgets, actual: number) {
  const budget = (budgets[name] as { actions: number }).actions;
  expect(actual, `${name}: ${actual} actions, budget ${budget}`).toBeLessThanOrEqual(Math.floor(budget * 1.1));
}

const ID = "11111111-1111-4111-8111-111111111111";
const ANN = {
  id: ID,
  version: 1,
  author: "dr-who",
  patient: {
    id: ID,
    names: [{ family: "Smith", given: ["Ann"] }],
    birth_date: "1980-03-14",
    sex: "female",
    identifiers: [],
    addresses: [],
    telecom: [],
  },
  replaced_by: null,
  replaces: [],
};
const NOT_RECORDED = { allergies: [], statement: null, summary: { state: "not-recorded" } };
const stubPatient = (page: Page) =>
  page.route(`**/api/proxy/api/patients/${ID}`, (route) => json(route, 200, ANN));

test("budget: register a patient from the search page", async ({ page }) => {
  const u = user();
  await stubPatient(page);
  await page.route("**/api/proxy/api/patients", (route) => json(route, 201, ANN));
  await page.route("**/api/proxy/api/patients/*/allergies", (route) => json(route, 200, NOT_RECORDED));
  await page.goto("/en-001/patients");
  await u.click(page.getByRole("link", { name: "Register a new patient" }).first());
  await u.type(page.getByLabel("Family name"), "Smith");
  await u.type(page.getByLabel("Given name"), "Ann");
  await u.type(page.getByLabel("Birth date"), "1980-03-14");
  await u.choose(page.getByLabel("Sex"), "female");
  await u.click(page.getByRole("button", { name: "Register", exact: true }));
  await expect(page).toHaveURL(new RegExp(ID));
  within("register-a-patient", u.actions);
});

test("budget: record an allergy from the search page", async ({ page }) => {
  const u = user();
  let posted = 0;
  await stubPatient(page);
  await page.route("**/api/proxy/api/patients?**", (route) => json(route, 200, { results: [ANN], limit: 50 }));
  await page.route("**/api/proxy/api/patients/*/allergies", (route) => {
    if (route.request().method() === "GET") return json(route, 200, NOT_RECORDED);
    posted++;
    return json(route, 201, {});
  });
  await page.goto("/en-001/patients");
  await u.type(page.getByLabel("Family name"), "Smith");
  await u.click(page.getByRole("button", { name: "Search" }));
  await u.click(page.getByRole("link", { name: /Smith, Ann/ }));
  await u.type(page.getByLabel("Substance"), "Penicillin");
  await u.click(page.getByRole("button", { name: "Record allergy" }));
  await expect.poll(() => posted).toBe(1);
  within("record-an-allergy", u.actions);
});
