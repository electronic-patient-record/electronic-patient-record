import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page, type Route } from "@playwright/test";

const NHS = "https://fhir.nhs.uk/Id/nhs-number";
const ANN = {
  id: "11111111-1111-4111-8111-111111111111",
  version: 2,
  author: "dr-who",
  patient: {
    id: "11111111-1111-4111-8111-111111111111",
    names: [
      { family: "Smith", given: ["Ann"] },
      { family: "Jones", given: [] },
    ],
    birth_date: "1980-03-14",
    sex: "female",
    identifiers: [{ system: NHS, value: "9434765919" }],
    addresses: [],
    telecom: [{ system: "email", value: "ann@example.org" }],
  },
};
const CANDIDATE = {
  existing: ANN,
  score: 0.97,
  decision: "auto_link",
  fields: [],
  deterministic: "same NHS number",
  conflicts: [],
};

const json = (route: Route, status: number, body: unknown) =>
  route.fulfill({ status, json: body });

async function axe(page: Page) {
  const r = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag22aa"])
    .analyze();
  expect(r.violations).toEqual([]);
}

test("an empty search sends nothing and says why", async ({ page }) => {
  let requests = 0;
  await page.route("**/api/proxy/api/patients?**", (route) => {
    requests += 1;
    return json(route, 200, { results: [], limit: 50 });
  });
  await page.goto("/en-001/patients");
  await page.getByRole("button", { name: "Search" }).click();
  await expect(page.getByRole("alert")).toHaveText(
    "Enter at least one search term.",
  );
  expect(requests).toBe(0);
});

test("a search lists matches and links to each record", async ({ page }) => {
  let url = "";
  await page.route("**/api/proxy/api/patients?**", (route) => {
    url = route.request().url();
    return json(route, 200, { results: [ANN], limit: 50 });
  });
  await page.goto("/en-001/patients");
  await page.getByLabel("Family name").fill("Smi");
  await page.getByLabel("NHS number").fill("943 476 5919");
  await page.getByRole("button", { name: "Search" }).click();
  await expect(
    page.getByRole("cell", { name: "Smith, Ann", exact: true }),
  ).toBeVisible();
  const q = new URL(url).searchParams;
  expect(q.get("family")).toBe("Smi");
  expect(q.get("identifier")).toBe(`${NHS}|9434765919`);
  await axe(page);
  await expect(page.getByRole("link", { name: /^Open/ })).toHaveAttribute(
    "href",
    `/en-001/patients/${ANN.id}`,
  );
});

test("a search with no match says so; a refusal is announced", async ({
  page,
}) => {
  await page.route("**/api/proxy/api/patients?**", (route) =>
    json(route, 200, { results: [], limit: 50 }),
  );
  await page.goto("/en-001/patients");
  await page.getByLabel("Family name").fill("Nobody");
  await page.getByRole("button", { name: "Search" }).click();
  await expect(page.getByText("No patient matches.")).toBeVisible();

  await page.unroute("**/api/proxy/api/patients?**");
  await page.route("**/api/proxy/api/patients?**", (route) =>
    json(route, 403, { error: "forbidden" }),
  );
  await page.getByRole("button", { name: "Search" }).click();
  await expect(page.getByRole("alert")).toHaveText(
    "Your role does not allow this.",
  );
});

test("registering shows the server's validation problems", async ({ page }) => {
  await page.route("**/api/proxy/api/patients", (route) =>
    json(route, 422, { errors: ["NHS number fails the modulus 11 check"] }),
  );
  await page.goto("/en-001/patients/new");
  await page.getByLabel("Family name").fill("Smith");
  await page.getByLabel("NHS number").fill("9434765910");
  await page.getByRole("button", { name: "Register" }).click();
  await expect(
    page.getByText("NHS number fails the modulus 11 check"),
  ).toBeVisible();
  await axe(page);
});

test("a possible duplicate is shown side by side and needs a reason to override", async ({
  page,
}) => {
  const bodies: Record<string, unknown>[] = [];
  await page.route("**/api/proxy/api/patients", async (route) => {
    const body = route.request().postDataJSON() as Record<string, unknown>;
    bodies.push(body);
    if (body.override_reason)
      return json(route, 201, {
        ...ANN,
        id: "22222222-2222-4222-8222-222222222222",
        version: 1,
      });
    return json(route, 409, {
      error: "possible duplicate",
      candidates: [CANDIDATE],
    });
  });
  await page.route(
    "**/api/proxy/api/patients/22222222-2222-4222-8222-222222222222",
    (route) =>
      json(route, 200, {
        ...ANN,
        id: "22222222-2222-4222-8222-222222222222",
        version: 1,
      }),
  );
  await page.goto("/en-001/patients/new");
  await page.getByLabel("Family name").fill("Smith");
  await page.getByLabel("Given name").fill("Ann");
  await page.getByLabel("Birth date").fill("1980-03-14");
  await page.getByLabel("NHS number").fill("943 476 5919");
  await page.getByRole("button", { name: "Register", exact: true }).click();

  await expect(
    page.getByRole("heading", {
      name: "This patient may already be registered",
    }),
  ).toBeVisible();
  await expect(page.getByText("Automatic link")).toBeVisible();
  await expect(page.getByText("same NHS number")).toBeVisible();
  const table = page.getByRole("table");
  await expect(
    table.getByRole("columnheader", { name: "As entered" }),
  ).toBeVisible();
  await expect(
    table.getByRole("row", { name: /Family name Smith Smith \(same\)/ }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Open the existing record" }),
  ).toHaveAttribute("href", `/en-001/patients/${ANN.id}`);
  await axe(page);

  // No reason: nothing is sent.
  await page.getByRole("button", { name: "Register anyway" }).click();
  await expect(
    page.getByRole("alert").filter({ hasText: "Give the reason" }),
  ).toBeVisible();
  expect(bodies).toHaveLength(1);

  await page
    .getByLabel(/This is a different person/)
    .fill("twin brother, same birth date");
  await page.getByRole("button", { name: "Register anyway" }).click();
  await expect(page).toHaveURL(/\/en-001\/patients\/22222222-/);
  expect(bodies).toHaveLength(2);
  expect(bodies[1]?.override_reason).toBe("twin brother, same birth date");
  expect(bodies[0]?.override_reason).toBeUndefined();
});

test("editing sends the expected version and keeps what the form does not show", async ({
  page,
}) => {
  let put: Record<string, any> | null = null;
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, async (route) => {
    if (route.request().method() === "PUT") {
      put = route.request().postDataJSON();
      return json(route, 200, { ...ANN, version: 3, patient: put!.patient });
    }
    return json(route, 200, ANN);
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  await expect(page.getByLabel("Family name")).toHaveValue("Smith");
  await expect(page.getByText("Version: 2")).toBeVisible();
  await axe(page);
  await page.getByLabel("Birth date").fill("1980-03-15");
  await page.getByLabel("Reason").fill("corrected");
  await page.getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByRole("status")).toHaveText("Saved. Version 3.");
  expect(put!.expected_version).toBe(2);
  expect(put!.reason).toBe("corrected");
  expect(put!.patient.birth_date).toBe("1980-03-15");
  expect(put!.patient.id).toBeUndefined();
  expect(put!.patient.names[1]).toEqual({ family: "Jones", given: [] });
  expect(put!.patient.telecom).toEqual(ANN.patient.telecom);
});

test("a stale edit says the record changed and offers a reload", async ({
  page,
}) => {
  let gets = 0;
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) => {
    if (route.request().method() === "PUT")
      return json(route, 409, {
        error: "version conflict",
        current_version: 3,
      });
    gets += 1;
    return json(route, 200, ANN);
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  await page.getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Someone else changed this record",
  );
  await page.getByRole("button", { name: "Reload the record" }).click();
  await expect.poll(() => gets).toBe(2);
});

test("a missing patient and a signed-out reader get a message, not a form", async ({
  page,
}) => {
  await page.route("**/api/proxy/api/patients/*", (route) =>
    json(route, 404, { error: "not found" }),
  );
  await page.goto("/en-001/patients/33333333-3333-4333-8333-333333333333");
  await expect(page.getByRole("alert")).toHaveText("No such patient.");
  await expect(page.getByRole("button", { name: "Save changes" })).toHaveCount(
    0,
  );

  await page.unroute("**/api/proxy/api/patients/*");
  await page.route("**/api/proxy/api/patients/*", (route) =>
    json(route, 401, { error: "sign in required" }),
  );
  await page.reload();
  await expect(page.getByRole("alert")).toHaveText("Sign in to continue.");
});

test("a restricted candidate blocks registration without showing the record", async ({
  page,
}) => {
  await page.route("**/api/proxy/api/patients", (route) =>
    json(route, 409, {
      error: "possible duplicate",
      candidates: [{ restricted: true }],
    }),
  );
  await page.goto("/en-001/patients/new");
  await page.getByLabel("Family name").fill("Smith");
  await page.getByRole("button", { name: "Register", exact: true }).click();
  await expect(
    page.getByText("its owner has restricted who can see it"),
  ).toBeVisible();
  await expect(page.getByRole("table")).toHaveCount(0);
  await expect(
    page.getByRole("link", { name: "Open the existing record" }),
  ).toHaveCount(0);
  await expect(page.getByLabel(/This is a different person/)).toBeVisible();
  await axe(page);
});

test("my record asks for patient-access and shows the linked record read-only", async ({
  page,
}) => {
  let purpose: string | null = null;
  await page.route("**/api/proxy/api/me/patients", (route) => {
    purpose = route.request().headers()["x-purpose-of-use"] ?? null;
    return json(route, 200, {
      records: [
        { relationship: "self", patient: ANN },
        {
          relationship: "proxy",
          patient: {
            ...ANN,
            id: "44444444-4444-4444-8444-444444444444",
            patient: {
              ...ANN.patient,
              names: [{ family: "Young", given: ["Sam"] }],
              identifiers: [],
            },
          },
        },
      ],
    });
  });
  await page.goto("/en-001/my-record");
  await expect(
    page.getByRole("heading", { level: 2, name: "Smith, Ann" }),
  ).toBeVisible();
  await expect(
    page.getByText("You are viewing this record as a proxy."),
  ).toBeVisible();
  await expect(
    page.getByRole("term").filter({ hasText: "NHS number" }).first(),
  ).toBeVisible();
  await expect(page.getByRole("textbox")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Save changes" })).toHaveCount(
    0,
  );
  expect(purpose).toBe("patient-access");
  await axe(page);
});

test("my record says plainly when nothing is linked, or the reader may not use it", async ({
  page,
}) => {
  await page.route("**/api/proxy/api/me/patients", (route) =>
    json(route, 200, { records: [] }),
  );
  await page.goto("/en-001/my-record");
  await expect(page.getByRole("status")).toContainText(
    "No record is linked to your sign-in",
  );
  await axe(page);

  await page.unroute("**/api/proxy/api/me/patients");
  await page.route("**/api/proxy/api/me/patients", (route) =>
    json(route, 403, { error: "forbidden" }),
  );
  await page.reload();
  await expect(page.getByRole("alert")).toHaveText(
    "Your role does not allow this.",
  );
});

test("a clinician can declare treatment; it is sent, remembered for the tab, and not sent from my record", async ({
  page,
}) => {
  const seen: (string | null)[] = [];
  await page.route("**/api/proxy/api/patients?**", (route) => {
    seen.push(route.request().headers()["x-purpose-of-use"] ?? null);
    return json(route, 200, { results: [], limit: 50 });
  });
  await page.route("**/api/proxy/api/me/patients", (route) => {
    seen.push(route.request().headers()["x-purpose-of-use"] ?? null);
    return json(route, 200, { records: [] });
  });
  const search = async () => {
    await page.getByLabel("Family name").fill("Smith");
    await page.getByRole("button", { name: "Search" }).click();
    await expect(page.getByText("No patient matches.")).toBeVisible();
  };

  await page.goto("/en-001/patients");
  await expect(page.getByLabel("Purpose of use")).toHaveValue("operations");
  await search();
  await page.getByLabel("Purpose of use").selectOption("treatment");
  await search();

  // Remembered across a reload in this tab.
  await page.reload();
  await expect(page.getByLabel("Purpose of use")).toHaveValue("treatment");
  await search();
  expect(seen).toEqual(["operations", "treatment", "treatment"]);

  // "My record" declares patient access whatever the staff choice, and shows no picker.
  await page.goto("/en-001/my-record");
  await expect(page.getByRole("status")).toBeVisible();
  await expect(page.getByLabel("Purpose of use")).toHaveCount(0);
  expect(seen.at(-1)).toBe("patient-access");
  await axe(page);
});

test("my record shows who has accessed it, on request, as staff without names", async ({
  page,
}) => {
  const ID = ANN.id;
  const urls: string[] = [];
  await page.route("**/api/proxy/api/me/patients", (route) =>
    json(route, 200, { records: [{ relationship: "self", patient: ANN }] }),
  );
  await page.route(
    `**/api/proxy/api/me/patients/${ID}/access-log*`,
    (route) => {
      urls.push(route.request().url());
      const older = route.request().url().includes("before=");
      return json(
        route,
        200,
        older
          ? {
              entries: [
                {
                  id: 3,
                  at: "2026-10-20T08:00:00.000000Z",
                  by: "staff",
                  did: "was refused access to",
                  what: "identity record",
                  refused: true,
                  purpose: "operations",
                  break_glass: false,
                },
              ],
              next: null,
            }
          : {
              entries: [
                {
                  id: 9,
                  at: "2026-10-21T09:00:12.000000Z",
                  by: "staff",
                  roles: ["clinician"],
                  organisations: ["riverside"],
                  did: "viewed",
                  what: "identity record",
                  refused: false,
                  purpose: "treatment",
                  break_glass: false,
                },
              ],
              next: 9,
            },
      );
    },
  );
  await page.goto("/en-001/my-record");
  // Not fetched until asked: reading the log is itself recorded.
  expect(urls).toHaveLength(0);
  await page
    .getByRole("button", { name: "Show who has accessed this record" })
    .click();
  await expect(
    page.getByText(
      "A member of staff (clinician, riverside) viewed your identity record. Purpose: treatment.",
    ),
  ).toBeVisible();
  await expect(page.getByText("2026-10-21 09:00 UTC")).toBeVisible();
  await axe(page);
  await page.getByRole("button", { name: "Show older entries" }).click();
  await expect(
    page.getByText("was refused access to your identity record"),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Show older entries" }),
  ).toHaveCount(0);
  expect(urls[1]).toContain("before=9");
});
