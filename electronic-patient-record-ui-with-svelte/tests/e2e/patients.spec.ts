import AxeBuilder from "@axe-core/playwright";
import type { Page, Route } from "@playwright/test";
import { expect, test } from "./fixtures";

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

const NOT_RECORDED = {
  allergies: [],
  statement: null,
  summary: { state: "not-recorded" },
};
const stubAllergies = (
  page: Page,
  body: unknown = NOT_RECORDED,
  status = 200,
) =>
  page.route("**/api/proxy/api/patients/*/allergies", (route) =>
    route.request().method() === "GET"
      ? json(route, status, body)
      : route.fallback(),
  );

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

test("registration problems come by code, in the page's language, with the API's English for an unknown code", async ({ page }) => {
  await page.route("**/api/proxy/api/patients", (route) =>
    json(route, 422, {
      errors: ["NHS number fails the modulus 11 check", "something new the UI has no code for"],
      error_codes: ["nhs_number_check_digit", "brand_new"],
    }),
  );
  await page.goto("/de-001/patients/new");
  await page.getByLabel("Family name").fill("Smith");
  await page.locator("form button[type=submit]").click();
  await expect(page.getByText("Die NHS-Nummer besteht die Prüfziffernprüfung nicht.")).toBeVisible();
  await expect(page.getByText("something new the UI has no code for")).toBeVisible();
  await expect(page.getByText("NHS number fails the modulus 11 check")).toHaveCount(0);
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
    page.getByRole("heading", { level: 2, name: "Smith, Ann", exact: true }),
  ).toBeVisible();
  // The same banner as on the staff page: two records, two banners.
  await expect(
    page.getByRole("region", { name: "Patient banner" }),
  ).toHaveCount(2);
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
  // The request follows the loading text a moment later: wait for it rather than race it.
  await expect.poll(() => seen.at(-1)).toBe("patient-access");
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

test("the patient banner shows who is on record, says allergies are unknown, and ignores what is being typed", async ({
  page,
}) => {
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, ANN),
  );
  await page.goto(`/en-001/patients/${ANN.id}`);
  const banner = page.getByRole("region", { name: "Patient banner" });
  await expect(banner).toBeVisible();
  await expect(
    banner.getByRole("heading", { name: "SMITH, Ann" }),
  ).toBeVisible();
  await expect(banner).toContainText("Born: 1980-03-14");
  await expect(banner).toContainText("Age: ");
  await expect(banner).toContainText("NHS number: 943 476 5919");
  await expect(banner).toContainText("Other identifier: none recorded");
  await expect(banner).toContainText("No photo held");
  // Not "none": the system holds no allergy data, and the banner says that in words.
  await expect(banner).toContainText(
    "Allergies: Not recorded in this system. This does not mean there are none.",
  );
  // The banner is what is on record: typing in the form does not change it.
  await page.getByLabel("Family name").fill("Someone Else");
  await expect(
    banner.getByRole("heading", { name: "SMITH, Ann" }),
  ).toBeVisible();
  await axe(page);
});

test("a record with little on it still gets a banner that says what is missing", async ({
  page,
}) => {
  await stubAllergies(page);
  const bare = {
    ...ANN,
    patient: {
      id: ANN.id,
      names: [],
      identifiers: [],
      addresses: [],
      telecom: [],
    },
  };
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, bare),
  );
  await page.goto(`/en-001/patients/${ANN.id}`);
  const banner = page.getByRole("region", { name: "Patient banner" });
  await expect(banner).toContainText("Birth date not recorded");
  await expect(banner).toContainText("NHS number: none recorded");
  await expect(banner).toContainText("Allergies: Not recorded");
  await axe(page);
});

test("the page says when the record was read, and says so loudly when it could not be refreshed", async ({
  page,
}) => {
  let up = true;
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    up ? json(route, 200, ANN) : json(route, 503, { error: "unavailable" }),
  );
  const show = () =>
    page.evaluate(() => {
      Object.defineProperty(document, "hidden", {
        value: false,
        configurable: true,
      });
      document.dispatchEvent(new Event("visibilitychange"));
    });
  await page.goto(`/en-001/patients/${ANN.id}`);
  await expect(page.getByText(/^Record read at \d{1,2}:\d{2}/)).toBeVisible();
  await expect(page.getByText("may not be current")).toHaveCount(0);

  // The next look fails: the same record stays on screen, with a warning that it may be out of date.
  up = false;
  await show();
  await expect(page.getByRole("alert").filter({ hasText: "may not be current" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "SMITH, Ann" })).toBeVisible();
  await axe(page);

  // A good look clears it.
  up = true;
  await show();
  await expect(page.getByText("may not be current")).toHaveCount(0);
  await expect(page.getByText(/^Record read at/)).toBeVisible();
});

test("the banner picks up another person's edit when the tab is shown again, and never overwrites typing", async ({
  page,
}) => {
  let current = ANN;
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, current),
  );
  const show = () =>
    page.evaluate(() => {
      Object.defineProperty(document, "hidden", {
        value: false,
        configurable: true,
      });
      document.dispatchEvent(new Event("visibilitychange"));
    });
  await page.goto(`/en-001/patients/${ANN.id}`);
  const banner = page.getByRole("region", { name: "Patient banner" });
  await expect(
    banner.getByRole("heading", { name: "SMITH, Ann" }),
  ).toBeVisible();

  // Clean form: the page simply catches up, form and banner.
  current = {
    ...ANN,
    version: 3,
    patient: { ...ANN.patient, names: [{ family: "Smythe", given: ["Ann"] }] },
  };
  await show();
  await expect(
    banner.getByRole("heading", { name: "SMYTHE, Ann" }),
  ).toBeVisible();
  await expect(page.getByLabel("Family name")).toHaveValue("Smythe");
  await expect(page.getByText("Version: 3")).toBeVisible();

  // Dirty form: typing is kept, the banner shows the newer record, and the page says so.
  await page.getByLabel("Given name").fill("Annie");
  current = {
    ...current,
    version: 4,
    patient: {
      ...current.patient,
      names: [{ family: "Smith-Jones", given: ["Ann"] }],
    },
  };
  await show();
  await expect(
    banner.getByRole("heading", { name: "SMITH-JONES, Ann" }),
  ).toBeVisible();
  await expect(page.getByRole("alert")).toContainText(
    "Someone else changed this record (now version 4)",
  );
  await expect(page.getByLabel("Given name")).toHaveValue("Annie");
  await axe(page);

  // Reloading is the explicit way to drop the typing.
  await page
    .getByRole("button", { name: "Reload and discard my changes" })
    .click();
  await expect(page.getByLabel("Given name")).toHaveValue("Ann");
  await expect(page.getByRole("alert")).toHaveCount(0);
});

// A 1x1 PNG.
const PNG_BYTES = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==",
  "base64",
);

test("the banner shows the photo, fetched under the declared purpose; the page uploads and removes one", async ({
  page,
}) => {
  let version: number | null = 1;
  const purposes: string[] = [];
  const writes: { method: string; url: string; type: string; size: number }[] =
    [];
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, photo_version: version }),
  );
  await page.route(`**/api/proxy/api/patients/${ANN.id}/photo**`, (route) => {
    const req = route.request();
    if (req.method() === "GET") {
      purposes.push(req.headers()["x-purpose-of-use"] ?? "");
      return route.fulfill({
        status: 200,
        contentType: "image/png",
        body: PNG_BYTES,
      });
    }
    writes.push({
      method: req.method(),
      url: req.url(),
      type: req.headers()["content-type"] ?? "",
      size: req.postDataBuffer()?.length ?? 0,
    });
    version = req.url().includes("/withdraw") ? null : (version ?? 0) + 1;
    return json(route, 200, { version, withdrawn: version === null });
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  const banner = page.getByRole("region", { name: "Patient banner" });
  await expect(
    banner.getByRole("img", { name: "Photo of SMITH, Ann" }),
  ).toBeVisible();
  await expect(banner).not.toContainText("No photo held");
  expect(purposes[0]).toBe("operations");
  await axe(page);

  // Upload: the bytes go as bytes, with the version the page last saw.
  await page.getByLabel(/Photo file/).setInputFiles({
    name: "ann.png",
    mimeType: "image/png",
    buffer: PNG_BYTES,
  });
  await page.getByRole("button", { name: "Upload photo" }).click();
  await expect(page.getByText("Photo saved.")).toBeVisible();
  expect(writes[0].method).toBe("PUT");
  expect(writes[0].url).toContain("expected_version=1");
  expect(writes[0].type).toBe("image/png");
  expect(writes[0].size).toBe(PNG_BYTES.length);

  // Remove: a POST to withdraw, and the banner goes back to saying there is no photo.
  await page.getByRole("button", { name: "Remove photo" }).click();
  await expect(page.getByText(/Photo removed/)).toBeVisible();
  expect(writes[1].method).toBe("POST");
  expect(writes[1].url).toContain("/photo/withdraw");
  await expect(banner).toContainText("No photo held");
  await expect(page.getByRole("button", { name: "Remove photo" })).toHaveCount(
    0,
  );
});

test("a file that is not a small PNG or JPEG is refused in the page before it is sent", async ({
  page,
}) => {
  let sent = 0;
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, ANN),
  );
  await page.route(`**/api/proxy/api/patients/${ANN.id}/photo**`, (route) => {
    sent++;
    return json(route, 200, {});
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  await page.getByRole("button", { name: "Upload photo" }).click();
  await expect(page.getByText("Choose a file first.")).toBeVisible();
  await page.getByLabel(/Photo file/).setInputFiles({
    name: "a.gif",
    mimeType: "image/gif",
    buffer: Buffer.from("GIF89a"),
  });
  await page.getByRole("button", { name: "Upload photo" }).click();
  await expect(
    page.getByText("The photo must be a PNG or a JPEG."),
  ).toBeVisible();
  await page.getByLabel(/Photo file/).setInputFiles({
    name: "big.png",
    mimeType: "image/png",
    buffer: Buffer.alloc(512 * 1024 + 1),
  });
  await page.getByRole("button", { name: "Upload photo" }).click();
  await expect(
    page.getByText("That file is larger than 512 KiB."),
  ).toBeVisible();
  expect(sent).toBe(0);
});

test("the banner's allergy line says exactly what is known, and never reads as none when it is not", async ({
  page,
}) => {
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, ANN),
  );
  const banner = page.getByRole("region", { name: "Patient banner" });
  const line = banner.locator("p.allergies");
  const data = (summary: unknown, allergies: unknown[] = []) => ({
    allergies,
    statement: null,
    summary,
  });
  const cases: [number, unknown, RegExp, boolean][] = [
    [
      200,
      data({
        state: "active",
        items: [
          { substance: "Penicillin", criticality: "high" },
          { substance: "Latex", criticality: null },
        ],
      }),
      /^Allergies: Penicillin \(high risk\), Latex$/,
      true,
    ],
    [
      200,
      data({
        state: "no-known",
        asserted_at: "2026-10-21T09:00:12.000000Z",
        asserted_by: "x",
      }),
      /^Allergies: No known allergies \(stated 2026-10-21 09:00 UTC\)\.$/,
      false,
    ],
    [
      200,
      data({ state: "none-active", inactive: 2 }),
      /Not confirmed as none known/,
      false,
    ],
    [
      200,
      data({ state: "not-recorded" }),
      /does not mean there are none/,
      false,
    ],
    [
      403,
      { error: "forbidden" },
      /Not shown to you for this patient\. This does not mean/,
      false,
    ],
    [
      500,
      { error: "internal error" },
      /Could not be loaded\. This does not mean/,
      false,
    ],
  ];
  for (const [status, body, expected, high] of cases) {
    await page.unroute("**/api/proxy/api/patients/*/allergies");
    await stubAllergies(page, body, status);
    await page.goto(`/en-001/patients/${ANN.id}`);
    await expect(line).toHaveText(expected);
    await expect(line).toHaveClass(high ? /high/ : /^((?!high).)*$/);
    await axe(page);
  }
});

test("a clinician records, ends and retracts an allergy and states none known; the banner follows", async ({
  page,
}) => {
  const sent: { method: string; url: string; body: any }[] = [];
  let state: any = {
    allergies: [],
    statement: null,
    summary: { state: "not-recorded" },
  };
  const A = (status: string, version: number) => ({
    id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
    patient_id: ANN.id,
    version,
    author: "dr",
    recorded_at: "2026-10-21T09:00:00.000000Z",
    allergy: { substance: "Penicillin", criticality: "high", status },
  });
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, ANN),
  );
  await page.route("**/api/proxy/api/patients/*/allergies**", (route) => {
    const req = route.request();
    if (req.method() === "GET") return json(route, 200, state);
    sent.push({
      method: req.method(),
      url: req.url(),
      body: req.postDataJSON(),
    });
    state = {
      allergies: [A("active", 1)],
      statement: null,
      summary: {
        state: "active",
        items: [{ substance: "Penicillin", criticality: "high" }],
      },
    };
    return json(route, 201, A("active", 1));
  });
  await page.route("**/api/proxy/api/patients/*/allergies/*", (route) => {
    const req = route.request();
    sent.push({
      method: req.method(),
      url: req.url(),
      body: req.postDataJSON(),
    });
    const status = req.postDataJSON().allergy.status;
    state = {
      allergies: [A(status, 2)],
      statement: null,
      summary: {
        state: status === "inactive" ? "none-active" : "not-recorded",
        inactive: 1,
      },
    };
    return json(route, 200, A(status, 2));
  });
  await page.route("**/api/proxy/api/patients/*/allergy-statement", (route) => {
    sent.push({
      method: "PUT",
      url: route.request().url(),
      body: route.request().postDataJSON(),
    });
    state = {
      allergies: [],
      statement: {
        version: 1,
        author: "dr",
        recorded_at: "2026-10-21T11:00:00.000000Z",
        no_known_allergies: true,
      },
      summary: {
        state: "no-known",
        asserted_at: "2026-10-21T11:00:00.000000Z",
        asserted_by: "dr",
      },
    };
    return json(route, 200, state.statement);
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  const line = page
    .getByRole("region", { name: "Patient banner" })
    .locator("p.allergies");
  await expect(line).toContainText("Not recorded in this system");
  await axe(page);

  await page.getByLabel("Substance").fill("Penicillin");
  await page.getByLabel("Risk").selectOption("high");
  await page.getByRole("button", { name: "Record allergy" }).click();
  await expect(line).toHaveText("Allergies: Penicillin (high risk)");
  expect(sent[0].method).toBe("POST");
  expect(sent[0].body.allergy).toEqual({
    substance: "Penicillin",
    status: "active",
    criticality: "high",
  });

  await page.getByRole("button", { name: "No longer applies" }).click();
  await expect(line).toContainText(
    "No active allergies are recorded (1 inactive)",
  );
  expect(sent[1].method).toBe("PUT");
  expect(sent[1].body.expected_version).toBe(1);
  expect(sent[1].body.allergy.status).toBe("inactive");

  await page.getByRole("button", { name: "Entered in error" }).click();
  await expect(line).toContainText("Not recorded in this system");
  // The page sends the version it last saw: the list had been reloaded at version 2.
  expect(sent[2].body.expected_version).toBe(2);
  expect(sent[2].body.allergy.status).toBe("entered-in-error");

  await page
    .getByRole("button", { name: "Record: no known allergies" })
    .click();
  await expect(line).toContainText(
    "No known allergies (stated 2026-10-21 11:00 UTC)",
  );
  expect(sent[3].body).toEqual({ none_known: true, expected_version: 0 });
  await expect(
    page.getByRole("button", {
      name: "Withdraw the no-known-allergies statement",
    }),
  ).toBeVisible();
  await axe(page);
});

test("a role that may not read allergies sees no allergy section and a banner that says so", async ({
  page,
}) => {
  await stubAllergies(page, { error: "forbidden" }, 403);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, ANN),
  );
  await page.goto(`/en-001/patients/${ANN.id}`);
  await expect(
    page.getByRole("region", { name: "Patient banner" }),
  ).toContainText("Not shown to you for this patient");
  await expect(
    page.getByRole("heading", { name: "Allergies and intolerances" }),
  ).toHaveCount(0);
});

test("an administrator records and ends a care relationship, and records and withdraws consent", async ({
  page,
}) => {
  const sent: { method: string; url: string; body: any }[] = [];
  let rels: any[] = [];
  let consents: any[] = [];
  const REL = (status: string, version: number) => ({
    id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
    patient_id: ANN.id,
    version,
    author: "admin",
    relationship: {
      subject: "sub-clin-1",
      kind: "care-team",
      status,
      period: null,
    },
  });
  const CON = (status: string, version: number) => ({
    id: "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
    patient_id: ANN.id,
    version,
    author: "admin",
    consent: {
      kind: "general",
      status,
      provisions: [
        {
          effect: "deny",
          actors: ["receptionist"],
          purposes: [],
          resources: [],
          labels: [],
        },
      ],
    },
  });
  await page.route("**/api/proxy/api/patients/*/relationships", (route) => {
    const req = route.request();
    if (req.method() === "GET")
      return json(route, 200, { relationships: rels });
    sent.push({ method: "POST", url: req.url(), body: req.postDataJSON() });
    rels = [REL("active", 1)];
    return json(route, 201, rels[0]);
  });
  await page.route("**/api/proxy/api/patients/*/relationships/*", (route) => {
    sent.push({
      method: "PUT",
      url: route.request().url(),
      body: route.request().postDataJSON(),
    });
    rels = [REL("ended", 2)];
    return json(route, 200, rels[0]);
  });
  await page.route("**/api/proxy/api/patients/*/consents", (route) => {
    const req = route.request();
    if (req.method() === "GET") return json(route, 200, { consents });
    sent.push({ method: "POST", url: req.url(), body: req.postDataJSON() });
    consents = [CON("active", 1)];
    return json(route, 201, consents[0]);
  });
  await page.route("**/api/proxy/api/patients/*/consents/*", (route) => {
    sent.push({
      method: "PUT",
      url: route.request().url(),
      body: route.request().postDataJSON(),
    });
    consents = [CON(route.request().postDataJSON().consent.status, 2)];
    return json(route, 200, consents[0]);
  });
  await page.goto(`/en-001/patients/${ANN.id}/access`);
  await expect(page.getByText("No relationships recorded.")).toBeVisible();
  await expect(page.getByText(/No consent recorded/)).toBeVisible();
  await axe(page);

  await page.getByLabel(/^Person/).fill("sub-clin-1");
  await page
    .getByLabel("Relationship", { exact: true })
    .selectOption("care-team");
  await page.getByRole("button", { name: "Record relationship" }).click();
  await expect(page.locator("strong", { hasText: "Care team" })).toBeVisible();
  expect(sent[0].body.relationship).toEqual({
    subject: "sub-clin-1",
    kind: "care-team",
    status: "active",
    period: null,
  });
  await page.getByRole("button", { name: "End relationship" }).click();
  await expect(page.locator("li", { hasText: "Ended" })).toBeVisible();
  expect(sent[1].body.expected_version).toBe(1);
  expect(sent[1].body.relationship.status).toBe("ended");

  await page.getByLabel("receptionist").check();
  await page.getByRole("button", { name: "Record consent" }).click();
  await expect(
    page.getByText(
      "Deny: receptionist · for any purpose · any record · any label",
    ),
  ).toBeVisible();
  expect(sent[2].body.consent.provisions[0].actors).toEqual(["receptionist"]);
  await axe(page);
  await page.getByRole("button", { name: "Withdraw" }).click();
  await expect(page.getByText("Withdrawn", { exact: true })).toBeVisible();
  expect(sent[3].body.expected_version).toBe(1);
  expect(sent[3].body.consent.status).toBe("inactive");
});

test("a role that may not manage access is told so instead of seeing forms", async ({
  page,
}) => {
  await page.route("**/api/proxy/api/patients/*/relationships", (r) =>
    json(r, 403, { error: "forbidden" }),
  );
  await page.route("**/api/proxy/api/patients/*/consents", (r) =>
    json(r, 403, { error: "forbidden" }),
  );
  await page.goto(`/en-001/patients/${ANN.id}/access`);
  await expect(page.getByRole("status")).toHaveCount(2);
  await expect(
    page.getByRole("button", { name: "Record relationship" }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Record consent" }),
  ).toHaveCount(0);
  await axe(page);
});

test("a drill record says so at the top of its banner, and a real one does not", async ({
  page,
}) => {
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, drill: true }),
  );
  await page.goto(`/en-001/patients/${ANN.id}`);
  const banner = page.getByRole("region", { name: "Patient banner" });
  await expect(banner).toContainText("DRILL — NOT A REAL PATIENT");
  await expect(banner.getByRole("alert")).toHaveText(/DRILL/);
  await axe(page);

  // The same record from an API that does not send the flag, or sends false, carries no notice.
  for (const body of [ANN, { ...ANN, drill: false }]) {
    await page.unroute(`**/api/proxy/api/patients/${ANN.id}`);
    await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
      json(route, 200, body),
    );
    await page.goto(`/en-001/patients/${ANN.id}`);
    await expect(banner).toContainText("SMITH, Ann");
    await expect(banner).not.toContainText("DRILL");
  }
});

test("the break-the-glass review list marks a grant that has waited too long", async ({
  page,
}) => {
  const grant = (id: string, overdue?: boolean) => ({
    id,
    patient_id: ANN.id,
    subject: "dr-who",
    reason: "collapsed",
    granted_at: "2026-10-20T10:00:00.000000Z",
    expires_at: "2026-10-20T14:00:00.000000Z",
    ...(overdue === undefined ? {} : { overdue }),
  });
  await page.route("**/api/proxy/api/break-glass/reviews", (route) =>
    json(route, 200, {
      grants: [grant("a", true), grant("b", false), grant("c")],
      limit: 50,
    }),
  );
  await page.goto("/en-001/admin/break-glass");
  const items = page.getByRole("listitem").filter({ hasText: "collapsed" });
  await expect(items).toHaveCount(3);
  await expect(items.nth(0)).toContainText("OVERDUE FOR REVIEW");
  await expect(items.nth(1)).not.toContainText("OVERDUE");
  await expect(items.nth(2)).not.toContainText("OVERDUE");
  await axe(page);
});

test("break-the-glass and allergy-statement times are on the site's clock, with UTC beside them", async ({
  page,
}) => {
  await page.route("**/api/proxy/api/break-glass/reviews", (route) =>
    json(route, 200, {
      grants: [
        {
          id: "a",
          patient_id: ANN.id,
          subject: "dr-who",
          reason: "collapsed",
          granted_at: "2026-10-20T10:00:00.000000Z",
          expires_at: "2026-10-20T14:00:00.000000Z",
        },
      ],
      limit: 50,
      site_time_zone: "America/New_York",
    }),
  );
  await page.goto("/en-001/admin/break-glass");
  await expect(page.getByText(/2026-10-20 06:00 EDT \(10:00 UTC\)/)).toBeVisible();
  await expect(page.getByText(/2026-10-20 10:00 EDT \(14:00 UTC\)/)).toBeVisible();

  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, site_time_zone: "America/New_York" }),
  );
  await page.route("**/api/proxy/api/patients/*/allergies", (route) =>
    json(route, 200, {
      allergies: [],
      statement: null,
      summary: { state: "no-known", asserted_at: "2026-10-21T13:00:00.000000Z" },
    }),
  );
  await page.goto(`/en-001/patients/${ANN.id}`);
  await expect(page.getByRole("region", { name: "Patient banner" })).toContainText(
    "No known allergies (stated 2026-10-21 09:00 EDT (13:00 UTC))",
  );
});

test("the refused-message queue shows when a message arrived on the site's clock", async ({
  page,
}) => {
  await page.route("**/api/proxy/api/feed/refusals", (route) =>
    json(route, 200, { refusals: [REFUSAL()], limit: 50, site_time_zone: "America/New_York" }),
  );
  await page.goto("/en-001/admin/feed");
  await expect(page.getByText(/received 2026-10-21 05:00 EDT \(09:00 UTC\)/)).toBeVisible();
});

const OBS = (over: Record<string, unknown>) => ({
  id: "o-" + Math.random().toString(36).slice(2),
  patient_id: ANN.id,
  version: 1,
  author: "nurse-1",
  recorded_at: "2026-10-26T14:00:00.000000Z",
  site_zone: "America/New_York",
  code: "8867-4",
  value: 72,
  unit: "/min",
  effective_at: "2026-10-26T13:30:00.000000Z",
  status: "final",
  flag: null,
  ...over,
});

test("the vital signs list says where each value sits in words, and never calls a missing range normal", async ({
  page,
}) => {
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, site_time_zone: "America/New_York" }),
  );
  await page.route("**/api/proxy/api/patients/*/observations", (route) =>
    json(route, 200, {
      observations: [
        OBS({ value: 130, range: { low: 60, high: 100 }, flag: "high" }),
        OBS({ code: "8310-5", value: 36.6, unit: "Cel", effective_at: "2026-10-26T13:00:00.000000Z" }),
        OBS({
          code: "59408-5",
          value: null,
          unit: null,
          absent_reason: "failed",
          effective_at: "2026-10-26T12:30:00.000000Z",
        }),
        OBS({ code: "8480-6", value: 150, unit: "mm[Hg]", range: { low: 90, high: 140 }, flag: "critical-high", version: 2 }),
        OBS({ code: "29463-7", value: 700, unit: "kg", status: "entered-in-error" }),
      ],
    }),
  );
  await page.goto(`/en-001/patients/${ANN.id}`);
  const section = page.getByRole("region", { name: "Vital signs" });
  await expect(section.getByRole("list").first().getByText("Heart rate")).toBeVisible();
  await expect(section.getByText("130 /min")).toBeVisible();
  const heart = section.getByRole("listitem").filter({ hasText: "Heart rate" });
  await expect(heart).toContainText("· High ·");
  await expect(heart).toContainText("Range given: 60 to 100");
  // No range: not assessed, in words, never normal.
  const temp = section.getByRole("listitem").filter({ hasText: "Temperature" });
  await expect(temp).toContainText("36.6 °C");
  await expect(temp).toContainText("Not assessed (no range given)");
  await expect(temp).not.toContainText(/within|normal/i);
  // No value: why, never 0 or a dash.
  const spo2 = section.getByRole("listitem").filter({ hasText: "Oxygen saturation" });
  await expect(spo2).toContainText("No value: attempt failed");
  await expect(spo2).toContainText("Not assessed (no value)");
  await expect(section.getByRole("listitem").filter({ hasText: "Blood pressure, systolic" })).toContainText(
    "Critically high",
  );
  await expect(section.getByRole("listitem").filter({ hasText: "Blood pressure, systolic" })).toContainText(
    "version 2",
  );
  // The time is the site's clock with UTC beside it.
  await expect(section.getByText(/2026-10-26 09:30 EDT \(13:30 UTC\)/).first()).toBeVisible();
  // A mistake is collapsed, struck through, and not in the main list.
  const mistaken = section.getByRole("group").filter({ hasText: "Entered in error (1)" });
  await expect(mistaken).toBeVisible();
  await expect(section.getByRole("list").first()).not.toContainText("Weight");
  await axe(page);
});

test("the vital signs say so when there are none, when the role may not see them, and when they could not be loaded", async ({
  page,
}) => {
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) => json(route, 200, ANN));
  let state: [number, unknown] = [200, { observations: [] }];
  await page.route("**/api/proxy/api/patients/*/observations", (route) => json(route, state[0], state[1]));
  const section = () => page.getByRole("region", { name: "Vital signs" });
  await page.goto(`/en-001/patients/${ANN.id}`);
  await expect(section()).toContainText("No vital signs recorded in this system. This does not mean none were taken.");
  state = [403, { error: "forbidden" }];
  await page.reload();
  await expect(section()).toContainText("Vital signs are not shown to your role. This does not mean none are recorded.");
  await expect(section().getByRole("button", { name: "Record vital sign" })).toHaveCount(0);
  state = [500, { error: "internal error" }];
  await page.reload();
  await expect(section()).toContainText("Vital signs could not be loaded. This does not mean none are recorded.");
  await expect(section()).not.toContainText("No vital signs recorded");
});

test("recording a vital sends its own unit and the site's time as UTC, never an empty value as zero, and shows the API's refusals", async ({
  page,
}) => {
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, site_time_zone: "America/New_York" }),
  );
  const sent: any[] = [];
  let answer: [number, unknown] = [201, OBS({})];
  let rows: unknown[] = [];
  await page.route("**/api/proxy/api/patients/*/observations", (route) => {
    if (route.request().method() === "POST") {
      sent.push(route.request().postDataJSON());
      if (answer[0] === 201) rows = [OBS({})];
      return json(route, answer[0], answer[1]);
    }
    return json(route, 200, { observations: rows });
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  const section = page.getByRole("region", { name: "Vital signs" });
  await expect(section).toContainText("No vital signs recorded");
  await axe(page);

  // The unit is fixed text beside the value, and follows the vital chosen.
  await expect(section.getByLabel("Value (/min)")).toBeVisible();
  await section.getByLabel("Vital sign").selectOption({ label: "Temperature" });
  await expect(section.getByLabel("Value (°C)")).toBeVisible();

  // An empty or unreadable value sends nothing (the box also refuses it).
  await section.getByLabel("Vital sign").selectOption({ label: "Heart rate" });
  await section.getByLabel("Value (/min)").fill("seventy");
  await section.getByLabel(/^Measured at/).fill("2026-10-26T09:30");
  await section.getByRole("button", { name: "Record vital sign" }).click();
  await expect(section.getByText("Give the value as a number.")).toBeVisible();
  expect(sent).toHaveLength(0);

  // A good one: its own unit, the number, and 09:30 in New York as 13:30 UTC.
  await section.getByLabel("Value (/min)").fill("72");
  await section.getByLabel("Range from (optional)").fill("60");
  await section.getByLabel("Range up to (optional)").fill("100");
  await section.getByRole("button", { name: "Record vital sign" }).click();
  await expect(section.getByText("Vital sign recorded.")).toBeVisible();
  expect(sent).toEqual([
    {
      observation: {
        code: "8867-4",
        effective_at: "2026-10-26T13:30:00Z",
        status: "final",
        value: 72,
        unit: "/min",
        range: { low: 60, high: 100 },
      },
    },
  ]);
  await expect(section.getByRole("listitem")).toHaveCount(1);

  // No value: a reason is needed, and the value is not sent at all.
  await section.getByLabel("No value").check();
  await section.getByLabel(/^Measured at/).fill("2026-10-26T09:45");
  await section.getByLabel("Why there is no value").selectOption("");
  await section.getByRole("button", { name: "Record vital sign" }).click();
  expect(sent).toHaveLength(1);
  await section.getByLabel("Why there is no value").selectOption({ label: "declined" });
  await section.getByRole("button", { name: "Record vital sign" }).click();
  await expect(section.getByText("Vital sign recorded.")).toBeVisible();
  expect(sent[1].observation).toEqual({
    code: "8867-4",
    effective_at: "2026-10-26T13:45:00Z",
    status: "final",
    absent_reason: "declined",
  });

  // The API's refusals are shown whole.
  answer = [422, { errors: ["the unit \"x\" does not fit this code"], error_codes: ["unit_mismatch"] }];
  await section.getByLabel("No value").uncheck();
  await section.getByLabel(/^Value/).fill("72");
  await section.getByRole("button", { name: "Record vital sign" }).click();
  await expect(section.getByText('the unit "x" does not fit this code')).toBeVisible();
  answer = [409, { error: "version conflict" }];
  await section.getByRole("button", { name: "Record vital sign" }).click();
  await expect(section.getByText(/changed or was merged/)).toBeVisible();
});

test("a record merged away says so everywhere and cannot be edited; the surviving record merges and unmerges", async ({
  page,
}) => {
  const TARGET = "33333333-3333-4333-8333-333333333333";
  await stubAllergies(page);
  // The merged-away record.
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, replaced_by: TARGET, replaces: [] }),
  );
  await page.goto(`/en-001/patients/${ANN.id}`);
  const banner = page.getByRole("region", { name: "Patient banner" });
  await expect(banner).toContainText("THIS RECORD WAS MERGED INTO ANOTHER");
  await expect(
    page.getByRole("link", { name: "Open the surviving record" }),
  ).toHaveAttribute("href", `/en-001/patients/${TARGET}`);
  await expect(page.getByRole("button", { name: "Save changes" })).toHaveCount(
    0,
  );
  await expect(
    page.getByRole("button", { name: "Merge into this record" }),
  ).toHaveCount(0);
  await axe(page);
});

test("merging and unmerging send the reason, show the server's refusal, and reload", async ({
  page,
}) => {
  const OLD = "44444444-4444-4444-8444-444444444444";
  const sent: { path: string; body: any }[] = [];
  let replaces: string[] = [];
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, replaced_by: null, replaces }),
  );
  await page.route(`**/api/proxy/api/patients/${ANN.id}/merge`, (route) => {
    const body = route.request().postDataJSON();
    sent.push({ path: "merge", body });
    if (body.source === "refuse-me") {
      return json(route, 422, {
        errors: ["these do not look like the same person (match score 0.10)"],
      });
    }
    replaces = [OLD];
    return json(route, 201, { id: "l1", version: 1, status: "active" });
  });
  await page.route(`**/api/proxy/api/patients/${ANN.id}/unmerge`, (route) => {
    sent.push({ path: "unmerge", body: route.request().postDataJSON() });
    replaces = [];
    return json(route, 200, { id: "l1", version: 2, status: "reversed" });
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  const mergeBtn = page.getByRole("button", { name: "Merge into this record" });
  await expect(mergeBtn).toBeDisabled();
  await page.getByLabel(/^Id of the record/).fill("refuse-me");
  await mergeBtn.click();
  await expect(
    page.getByText(/Why are these the same person/).last(),
  ).toBeVisible();
  expect(sent).toHaveLength(0); // no reason, nothing sent
  await page
    .getByLabel(/^Why are these the same person/)
    .fill("same NHS number, duplicate registration");
  await mergeBtn.click();
  await expect(
    page.getByText(/do not look like the same person/),
  ).toBeVisible();
  expect(sent[0].body).toEqual({
    source: "refuse-me",
    reason: "same NHS number, duplicate registration",
  });

  await page.getByLabel(/^Id of the record/).fill(OLD);
  await mergeBtn.click();
  await expect(page.getByText("Records merged into this one")).toBeVisible();
  await expect(page.getByText(OLD)).toBeVisible();
  await axe(page);

  await page
    .getByLabel(/^Why are these the same person/)
    .fill("they are two people");
  await page.getByRole("button", { name: "Undo this merge" }).click();
  await expect(page.getByText("Records merged into this one")).toHaveCount(0);
  expect(sent[2]).toEqual({
    path: "unmerge",
    body: { source: OLD, reason: "they are two people" },
  });
});

const REFUSAL = (over: Record<string, unknown> = {}) => ({
  id: "44444444-4444-4444-8444-444444444444",
  received_at: "2026-10-21T09:00:12.000000Z",
  agent: "mllp:adt-feed",
  peer: "10.0.0.5",
  event: "A08",
  control_id: "M-1",
  kind: "conflict",
  reasons: ["the birth date differs from the record"],
  resolution: null,
  ...over,
});

test("an administrator sees the refused messages, reads one, and closes it with a reason", async ({
  page,
}) => {
  let open = [REFUSAL()];
  const sent: { url: string; body: any }[] = [];
  await page.route("**/api/proxy/api/feed/refusals", (route) =>
    json(route, 200, { refusals: open, limit: 50 }),
  );
  await page.route("**/api/proxy/api/feed/refusals/*", (route) =>
    json(route, 200, {
      ...REFUSAL(),
      message: "MSH|^~\\&|REG\rPID|1||M-1^^^LOCAL||Smith^Ann||19800314|F\r",
    }),
  );
  await page.route("**/api/proxy/api/feed/refusals/*/resolve", (route) => {
    sent.push({
      url: route.request().url(),
      body: route.request().postDataJSON(),
    });
    open = [];
    return json(route, 200, REFUSAL({ resolution: {} }));
  });
  await page.goto("/en-001/admin/feed");
  await expect(page.getByText("1 waiting")).toBeVisible();
  await expect(
    page.getByText("the birth date differs from the record"),
  ).toBeVisible();
  await axe(page);

  await page.getByRole("button", { name: "Show the message" }).click();
  await expect(page.locator("pre")).toContainText("PID|1||M-1^^^LOCAL");
  // The segments are on separate lines.
  expect(await page.locator("pre").innerText()).toContain("\nPID|1");
  await axe(page);

  // No reason, no request.
  await page.getByRole("button", { name: "Close this entry" }).click();
  await expect(
    page.getByText("Give a reason to close an entry."),
  ).toBeVisible();
  expect(sent).toHaveLength(0);

  await page.getByLabel("Outcome").selectOption("dismissed");
  await page.getByLabel("Reason (required)").fill("duplicate of M-0");
  await page.getByRole("button", { name: "Close this entry" }).click();
  await expect(page.getByText("Closed.")).toBeVisible();
  await expect(
    page.getByText("No refused messages are waiting."),
  ).toBeVisible();
  expect(sent[0].url).toContain(
    "/44444444-4444-4444-8444-444444444444/resolve",
  );
  expect(sent[0].body).toEqual({
    outcome: "dismissed",
    reason: "duplicate of M-0",
  });
});

test("a role that may not read the queue is told so, and an entry closed by someone else reloads", async ({
  page,
}) => {
  await page.route("**/api/proxy/api/feed/refusals", (route) =>
    json(route, 403, { error: "forbidden" }),
  );
  await page.goto("/en-001/admin/feed");
  await expect(page.getByText("Your role does not allow this.")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Show the message" }),
  ).toHaveCount(0);
  await axe(page);

  await page.unroute("**/api/proxy/api/feed/refusals");
  let open = [REFUSAL()];
  await page.route("**/api/proxy/api/feed/refusals", (route) =>
    json(route, 200, { refusals: open, limit: 50 }),
  );
  await page.route("**/api/proxy/api/feed/refusals/*", (route) =>
    json(route, 200, { ...REFUSAL(), message: "MSH|^~\\&|REG\r" }),
  );
  await page.route("**/api/proxy/api/feed/refusals/*/resolve", (route) => {
    open = [];
    return json(route, 409, { error: "already resolved" });
  });
  await page.reload();
  await page.getByRole("button", { name: "Show the message" }).click();
  await page.getByLabel("Reason (required)").fill("x");
  await page.getByRole("button", { name: "Close this entry" }).click();
  await expect(
    page.getByText("Someone else closed it. The list was reloaded."),
  ).toBeVisible();
  await expect(
    page.getByText("No refused messages are waiting."),
  ).toBeVisible();
});

test("the count of refused messages shows on the admin pages only, and not for a role that may not see it", async ({
  page,
}) => {
  let asked = 0;
  let status = 200;
  await page.route("**/api/proxy/api/feed/refusals/count", (route) => {
    asked += 1;
    return status === 200
      ? json(route, 200, { open: 3 })
      : json(route, 403, { error: "forbidden" });
  });
  await page.route("**/api/proxy/api/feed/refusals", (route) =>
    json(route, 200, { refusals: [REFUSAL()], limit: 50 }),
  );
  // Not asked for on the patient pages.
  await page.goto("/en-001/patients");
  await expect(
    page.getByRole("link", { name: "Refused messages" }),
  ).toBeVisible();
  expect(asked).toBe(0);
  await expect(page.getByText("(3 waiting)")).toHaveCount(0);

  // Asked for, and shown in the navigation, on the admin pages.
  await page.goto("/en-001/admin/feed");
  await expect(
    page.getByRole("navigation", { name: "Main" }).getByText("(3 waiting)"),
  ).toBeVisible();
  await axe(page);

  // A role refused the count sees no number and no error from it.
  status = 403;
  await page.reload();
  await expect(
    page.getByRole("link", { name: /Refused messages/ }).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("navigation", { name: "Main" }).getByText("(3 waiting)"),
  ).toHaveCount(0);
});

test("emergency access is offered only when the API offers it, needs a reason and a tick, and then shows the allergies", async ({
  page,
}) => {
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, ANN),
  );
  let granted = false;
  await page.route("**/api/proxy/api/patients/*/allergies", (route) =>
    granted
      ? json(route, 200, {
          allergies: [],
          statement: null,
          summary: {
            state: "active",
            items: [{ substance: "Penicillin", criticality: "high" }],
          },
        })
      : json(route, 403, { error: "forbidden", break_glass_offer: true }),
  );
  const sent: any[] = [];
  await page.route("**/api/proxy/api/patients/*/break-glass", (route) => {
    sent.push(route.request().postDataJSON());
    granted = true;
    return json(route, 201, {
      id: "g1",
      patient_id: ANN.id,
      expires_at: "2026-10-21T13:00:00.000000Z",
    });
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  await expect(
    page.getByRole("heading", { name: "Emergency access" }),
  ).toBeVisible();
  await axe(page);

  // Neither a reason alone nor a tick alone sends anything.
  await page.getByLabel(/^Reason for emergency access/).fill("collapsed");
  await page.getByRole("button", { name: /Open emergency access/ }).click();
  await expect(page.getByText(/Give a reason and tick the box/)).toBeVisible();
  expect(sent).toHaveLength(0);

  await page.getByLabel("I understand this is recorded and reviewed").check();
  await page.getByRole("button", { name: /Open emergency access/ }).click();
  await expect(
    page.getByText("Emergency access is open until 2026-10-21 13:00 UTC."),
  ).toBeVisible();
  expect(sent).toEqual([{ reason: "collapsed" }]);
  // The page read again under the grant, and the banner now shows the allergy.
  await expect(
    page.getByRole("region", { name: "Patient banner" }),
  ).toContainText("Penicillin");
});

test("a refusal that break-the-glass cannot lift shows no emergency form", async ({
  page,
}) => {
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, ANN),
  );
  await stubAllergies(page, { error: "forbidden" }, 403);
  await page.goto(`/en-001/patients/${ANN.id}`);
  await expect(
    page.getByText(/Not shown to you for this patient/).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Emergency access" }),
  ).toHaveCount(0);
});

test("the auditor sees the emergency access nobody has reviewed and records a verdict with a note", async ({
  page,
}) => {
  let grants = [
    {
      id: "g1",
      patient_id: ANN.id,
      subject: "sub-dr",
      reason: "collapsed in corridor",
      granted_at: "2026-10-21T09:00:12.000000Z",
      expires_at: "2026-10-21T13:00:12.000000Z",
    },
  ];
  const sent: any[] = [];
  await page.route("**/api/proxy/api/break-glass/reviews", (route) =>
    json(route, 200, { grants, limit: 50 }),
  );
  await page.route("**/api/proxy/api/break-glass/reviews/g1", (route) => {
    sent.push(route.request().postDataJSON());
    grants = [];
    return json(route, 200, { id: "g1" });
  });
  await page.goto("/en-001/admin/break-glass");
  await expect(page.getByText("collapsed in corridor")).toBeVisible();
  await axe(page);
  await page.getByRole("button", { name: "Review", exact: true }).click();
  await page.getByRole("button", { name: "Record the review" }).click();
  await expect(
    page.getByText("Write a note to record the review."),
  ).toBeVisible();
  expect(sent).toHaveLength(0);
  await page.getByLabel("Verdict").selectOption("inappropriate");
  await page.getByLabel("Note (required)").fill("no emergency found");
  await page.getByRole("button", { name: "Record the review" }).click();
  await expect(page.getByText("Review recorded.")).toBeVisible();
  await expect(
    page.getByText("No emergency access is waiting for review."),
  ).toBeVisible();
  expect(sent).toEqual([
    { outcome: "inappropriate", note: "no emergency found" },
  ]);
});

// CQ-6: moving from one patient to another must never show the first under the second's address.
const BOB = {
  ...ANN,
  id: "22222222-2222-4222-8222-222222222222",
  version: 1,
  replaced_by: null,
  replaces: [],
  patient: {
    ...ANN.patient,
    id: "22222222-2222-4222-8222-222222222222",
    names: [{ family: "Okafor", given: ["Bob"] }],
    identifiers: [],
  },
};
const allergyOf = (substance: string) => ({
  allergies: [],
  statement: null,
  summary: { state: "active", items: [{ substance, criticality: "high" }] },
});
const gate = () => {
  let open!: () => void;
  const opened = new Promise<void>((r) => (open = r));
  return { open, opened };
};

test("moving to another patient clears the first while the second loads", async ({
  page,
}) => {
  const bobArrives = gate();
  await page.route("**/api/proxy/api/patients/*/allergies", (route) =>
    json(
      route,
      200,
      allergyOf(
        route.request().url().includes(ANN.id) ? "Penicillin" : "Latex",
      ),
    ),
  );
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, replaced_by: BOB.id, replaces: [] }),
  );
  await page.route(`**/api/proxy/api/patients/${BOB.id}`, async (route) => {
    await bobArrives.opened;
    await json(route, 200, BOB);
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  const banner = page.getByRole("region", { name: "Patient banner" });
  await expect(
    banner.getByRole("heading", { name: "SMITH, Ann" }),
  ).toBeVisible();
  await expect(banner).toContainText("Penicillin");

  await page.getByRole("link", { name: "Open the surviving record" }).click();
  await expect(page).toHaveURL(new RegExp(`/patients/${BOB.id}$`));
  // Bob has not arrived: nothing of Ann's may be on screen under Bob's address.
  await expect(page.getByText("Loading…")).toBeVisible();
  await expect(page.getByText("SMITH, Ann")).toHaveCount(0);
  await expect(page.getByText("Penicillin")).toHaveCount(0);

  bobArrives.open();
  await expect(
    banner.getByRole("heading", { name: "OKAFOR, Bob" }),
  ).toBeVisible();
  await expect(banner).toContainText("Latex");
  await expect(page.getByText("Penicillin")).toHaveCount(0);
});

test("a late answer for the previous patient never lands on the current one", async ({
  page,
}) => {
  const annAllergiesArrive = gate();
  await page.route("**/api/proxy/api/patients/*/allergies", async (route) => {
    if (route.request().url().includes(ANN.id)) {
      await annAllergiesArrive.opened;
      await json(route, 200, allergyOf("Penicillin"));
    } else {
      await json(route, 200, allergyOf("Latex"));
    }
  });
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, replaced_by: BOB.id, replaces: [] }),
  );
  await page.route(`**/api/proxy/api/patients/${BOB.id}`, (route) =>
    json(route, 200, BOB),
  );
  await page.goto(`/en-001/patients/${ANN.id}`);
  const banner = page.getByRole("region", { name: "Patient banner" });
  await expect(
    banner.getByRole("heading", { name: "SMITH, Ann" }),
  ).toBeVisible();

  await page.getByRole("link", { name: "Open the surviving record" }).click();
  await expect(
    banner.getByRole("heading", { name: "OKAFOR, Bob" }),
  ).toBeVisible();
  await expect(banner).toContainText("Latex");
  // Ann's allergy answer arrives after Bob is on screen; it must be dropped.
  annAllergiesArrive.open();
  await page.waitForTimeout(300);
  await expect(banner).toContainText("Latex");
  await expect(page.getByText("Penicillin")).toHaveCount(0);
});

// CQ-36: when more records matched than the server compares, the page says only the closest were.
test("a cut candidate list says that only the closest records were compared", async ({
  page,
}) => {
  let cut = true;
  await page.route("**/api/proxy/api/patients", (route) =>
    json(route, 409, {
      error: "possible duplicate",
      candidates: [CANDIDATE],
      candidates_cut: cut,
    }),
  );
  await page.goto("/en-001/patients/new");
  await page.getByLabel("Family name").fill("Smith");
  await page.getByLabel("Birth date").fill("1980-03-14");
  await page.getByRole("button", { name: "Register", exact: true }).click();
  const note = page.getByText(
    "Many records matched; only the closest 200 were compared.",
    { exact: false },
  );
  await expect(note).toBeVisible();
  await axe(page);

  cut = false;
  await page.getByRole("button", { name: "Register", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      name: "This patient may already be registered",
    }),
  ).toBeVisible();
  await expect(note).toHaveCount(0);
});

test("my record shows when it was read and warns when a later look fails", async ({ page }) => {
  let up = true;
  await stubAllergies(page);
  await page.route("**/api/proxy/api/me/patients", (route) =>
    up
      ? json(route, 200, { records: [{ relationship: "self", patient: ANN }] })
      : json(route, 503, { error: "unavailable" }),
  );
  const show = () =>
    page.evaluate(() => {
      Object.defineProperty(document, "hidden", { value: false, configurable: true });
      document.dispatchEvent(new Event("visibilitychange"));
    });
  await page.goto("/en-001/my-record");
  await expect(page.getByText(/^Record read at \d{1,2}:\d{2}/)).toBeVisible();
  await expect(page.getByText("may not be current")).toHaveCount(0);

  up = false;
  await show();
  await expect(page.getByRole("alert").filter({ hasText: "may not be current" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Smith, Ann", exact: true })).toBeVisible();
  await axe(page);

  up = true;
  await show();
  await expect(page.getByText("may not be current")).toHaveCount(0);
  await expect(page.getByText(/^Record read at/)).toBeVisible();
});

test("recording a proxy sends its basis, scopes and last day, and the list shows them", async ({
  page,
}) => {
  const sent: any[] = [];
  let rels: any[] = [];
  await page.route("**/api/proxy/api/patients/*/relationships", (route) => {
    const req = route.request();
    if (req.method() === "GET") return json(route, 200, { relationships: rels });
    sent.push(req.postDataJSON());
    rels = [
      {
        id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
        patient_id: ANN.id,
        version: 1,
        author: "admin",
        relationship: sent[0].relationship,
      },
      {
        id: "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
        patient_id: ANN.id,
        version: 1,
        author: "admin",
        relationship: { subject: "old-proxy", kind: "proxy", status: "active" },
      },
    ];
    return json(route, 201, rels[0]);
  });
  await page.route("**/api/proxy/api/patients/*/consents", (route) =>
    json(route, 200, { consents: [] }),
  );
  await page.goto(`/en-001/patients/${ANN.id}/access`);
  await expect(page.getByLabel("Basis for the proxy")).toHaveCount(0);
  await page.getByLabel(/^Person/).fill("sub-mum");
  await page.getByLabel("Relationship", { exact: true }).selectOption("proxy");
  await page.getByLabel("Basis for the proxy").selectOption("parental-responsibility");
  await page.getByLabel("Allergies").check();
  await page.getByLabel(/^Last day/).fill("2026-12-31");
  await axe(page);
  await page.getByRole("button", { name: "Record relationship" }).click();
  expect(sent[0].relationship.proxy).toEqual({
    basis: "parental-responsibility",
    scopes: ["allergies"],
    expires_at: Date.UTC(2027, 0, 1) / 1000,
  });
  await expect(
    page.getByText("Allergies · until the end of 2026-12-31"),
  ).toBeVisible();
  await expect(page.getByText(/this proxy sees nothing/)).toBeVisible();
});

test("a double click on save sends one request, and the button is off while it is in flight", async ({
  page,
}) => {
  let puts = 0;
  let release: () => void = () => {};
  const held = new Promise<void>((r) => (release = r));
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, async (route) => {
    if (route.request().method() === "PUT") {
      puts++;
      await held;
      return json(route, 200, { ...ANN, version: 3 });
    }
    return json(route, 200, ANN);
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  const save = page.getByRole("button", { name: "Save changes" });
  await expect(save).toBeEnabled();
  await save.dblclick();
  await expect(save).toBeDisabled();
  release();
  await expect(page.getByRole("status").filter({ hasText: "Saved." })).toBeVisible();
  expect(puts).toBe(1);
  await expect(save).toBeEnabled();
});

test("two clicks at once on record relationship, and on record consent, send one request each", async ({
  page,
}) => {
  const posts = { rel: 0, consent: 0 };
  let release: () => void = () => {};
  const held = new Promise<void>((r) => (release = r));
  const rec = (kind: string) => ({
    id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
    patient_id: ANN.id,
    version: 1,
    author: "admin",
    [kind]: {},
  });
  await page.route("**/api/proxy/api/patients/*/relationships", async (route) => {
    if (route.request().method() === "GET")
      return json(route, 200, { relationships: [] });
    posts.rel++;
    await held;
    return json(route, 201, rec("relationship"));
  });
  await page.route("**/api/proxy/api/patients/*/consents", async (route) => {
    if (route.request().method() === "GET") return json(route, 200, { consents: [] });
    posts.consent++;
    await held;
    return json(route, 201, rec("consent"));
  });
  await page.goto(`/en-001/patients/${ANN.id}/access`);
  await page.getByLabel(/^Person/).fill("sub-1");
  const addRel = page.getByRole("button", { name: "Record relationship" });
  const addConsent = page.getByRole("button", { name: "Record consent" });
  // Two clicks in one task, before the page can redraw the button as disabled: only the guard in code stops these.
  await addRel.evaluate((b: HTMLButtonElement) => {
    b.click();
    b.click();
  });
  await addConsent.evaluate((b: HTMLButtonElement) => {
    b.click();
    b.click();
  });
  await expect(addRel).toBeDisabled();
  await expect(addConsent).toBeDisabled();
  release();
  await expect(addRel).toBeEnabled();
  await expect(addConsent).toBeEnabled();
  expect(posts).toEqual({ rel: 1, consent: 1 });
});

test("the main pages are right-to-left and pass axe in ar-001", async ({ page }) => {
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) => json(route, 200, ANN));
  await page.route("**/api/proxy/api/patients/*/relationships", (route) =>
    json(route, 200, { relationships: [] }),
  );
  await page.route("**/api/proxy/api/patients/*/consents", (route) =>
    json(route, 200, { consents: [] }),
  );
  await page.route("**/api/proxy/api/me/patients", (route) =>
    json(route, 200, { records: [{ relationship: "self", patient: ANN }] }),
  );
  for (const path of [
    "/ar-001/patients",
    "/ar-001/patients/new",
    `/ar-001/patients/${ANN.id}`,
    `/ar-001/patients/${ANN.id}/access`,
    "/ar-001/my-record",
  ]) {
    await page.goto(path);
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await axe(page);
  }
});

test("two clicks at once on record allergy send one request and switch the other allergy buttons off", async ({ page }) => {
  const posts = { add: 0, statement: 0 };
  let release: () => void = () => {};
  const held = new Promise<void>((r) => (release = r));
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) => json(route, 200, ANN));
  await page.route("**/api/proxy/api/patients/*/allergies", async (route) => {
    if (route.request().method() === "GET") return json(route, 200, NOT_RECORDED);
    posts.add++;
    await held;
    return json(route, 201, {});
  });
  await page.route("**/api/proxy/api/patients/*/allergy-statement", async (route) => {
    posts.statement++;
    await held;
    return json(route, 200, {});
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  await page.getByLabel("Substance").fill("Penicillin");
  const click2 = (b: import("@playwright/test").Locator) =>
    b.evaluate((el: HTMLButtonElement) => {
      el.click();
      el.click();
    });
  await click2(page.getByRole("button", { name: "Record allergy" }));
  // While the first write is in flight the other allergy buttons are off too.
  await expect(page.getByRole("button", { name: "Record: no known allergies" })).toBeDisabled();
  release();
  await expect(page.getByRole("button", { name: "Record allergy" })).toBeEnabled();
  expect(posts).toEqual({ add: 1, statement: 0 });
});

test("two clicks at once on open emergency access, and on record the review, send one request each", async ({ page }) => {
  const click2 = (b: import("@playwright/test").Locator) =>
    b.evaluate((el: HTMLButtonElement) => {
      el.click();
      el.click();
    });
  let release: () => void = () => {};
  const held = new Promise<void>((r) => (release = r));
  const posts = { glass: 0, review: 0 };
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) => json(route, 200, ANN));
  await stubAllergies(page, { error: "forbidden", break_glass_offer: true }, 403);
  await page.route("**/api/proxy/api/patients/*/break-glass", async (route) => {
    posts.glass++;
    await held;
    return json(route, 500, {});
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  await page.getByLabel(/^Reason for emergency access/).fill("collapsed");
  await page.getByLabel("I understand this is recorded and reviewed").check();
  await click2(page.getByRole("button", { name: /Open emergency access/ }));
  await expect(page.getByRole("button", { name: /Open emergency access/ })).toBeDisabled();
  release();
  await expect(page.getByRole("button", { name: /Open emergency access/ })).toBeEnabled();
  expect(posts.glass).toBe(1);

  let release2: () => void = () => {};
  const held2 = new Promise<void>((r) => (release2 = r));
  await page.route("**/api/proxy/api/break-glass/reviews", (route) =>
    json(route, 200, {
      grants: [
        {
          id: "g1",
          patient_id: ANN.id,
          subject: "sub-dr",
          reason: "collapsed in corridor",
          granted_at: "2026-10-21T09:00:12.000000Z",
          expires_at: "2026-10-21T13:00:12.000000Z",
        },
      ],
      limit: 50,
    }),
  );
  await page.route("**/api/proxy/api/break-glass/reviews/g1", async (route) => {
    posts.review++;
    await held2;
    return json(route, 500, {});
  });
  await page.goto("/en-001/admin/break-glass");
  await page.getByRole("button", { name: "Review", exact: true }).click();
  await page.getByLabel("Note (required)").fill("no emergency found");
  await click2(page.getByRole("button", { name: "Record the review" }));
  await expect(page.getByRole("button", { name: "Record the review" })).toBeDisabled();
  release2();
  await expect(page.getByRole("button", { name: "Record the review" })).toBeEnabled();
  expect(posts.review).toBe(1);
});

test("two clicks at once on merge and on close this entry send one request each", async ({ page }) => {
  const click2 = (b: import("@playwright/test").Locator) =>
    b.evaluate((el: HTMLButtonElement) => {
      el.click();
      el.click();
    });
  const posts = { merge: 0, close: 0 };
  let release: () => void = () => {};
  const held = new Promise<void>((r) => (release = r));
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, replaced_by: null, replaces: [] }),
  );
  await page.route(`**/api/proxy/api/patients/${ANN.id}/merge`, async (route) => {
    posts.merge++;
    await held;
    return json(route, 500, {});
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  await page.getByLabel(/^Id of the record/).fill("some-id");
  await page.getByLabel(/Why are these the same person/).fill("same person");
  await click2(page.getByRole("button", { name: "Merge into this record" }));
  await expect(page.getByRole("button", { name: "Merge into this record" })).toBeDisabled();
  release();
  await expect(page.getByRole("button", { name: "Merge into this record" })).toBeEnabled();
  expect(posts.merge).toBe(1);

  let release2: () => void = () => {};
  const held2 = new Promise<void>((r) => (release2 = r));
  await page.route("**/api/proxy/api/feed/refusals", (route) =>
    json(route, 200, { refusals: [REFUSAL()], limit: 50 }),
  );
  await page.route("**/api/proxy/api/feed/refusals/*", (route) =>
    json(route, 200, { ...REFUSAL(), message: "MSH|^~\\&|REG\r" }),
  );
  await page.route("**/api/proxy/api/feed/refusals/*/resolve", async (route) => {
    posts.close++;
    await held2;
    return json(route, 500, {});
  });
  await page.goto("/en-001/admin/feed");
  await page.getByRole("button", { name: "Show the message" }).click();
  await page.getByLabel("Reason (required)").fill("duplicate of M-0");
  await click2(page.getByRole("button", { name: "Close this entry" }));
  await expect(page.getByRole("button", { name: "Close this entry" })).toBeDisabled();
  release2();
  await expect(page.getByRole("button", { name: "Close this entry" })).toBeEnabled();
  expect(posts.close).toBe(1);
});

test("two clicks at once on upload photo send one request and switch remove photo off", async ({ page }) => {
  let writes = 0;
  let release: () => void = () => {};
  const held = new Promise<void>((r) => (release = r));
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, photo_version: 1 }),
  );
  await page.route(`**/api/proxy/api/patients/${ANN.id}/photo**`, async (route) => {
    if (route.request().method() === "GET")
      return route.fulfill({ status: 200, contentType: "image/png", body: PNG_BYTES });
    writes++;
    await held;
    return json(route, 500, {});
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  await page.getByLabel(/Photo file/).setInputFiles({
    name: "ann.png",
    mimeType: "image/png",
    buffer: PNG_BYTES,
  });
  await page.getByRole("button", { name: "Upload photo" }).evaluate((el: HTMLButtonElement) => {
    el.click();
    el.click();
  });
  await expect(page.getByRole("button", { name: "Remove photo" })).toBeDisabled();
  release();
  await expect(page.getByRole("button", { name: "Upload photo" })).toBeEnabled();
  expect(writes).toBe(1);
});

test("two clicks at once on register send one request (a second would make a duplicate patient)", async ({ page }) => {
  let posts = 0;
  let release: () => void = () => {};
  const held = new Promise<void>((r) => (release = r));
  await page.route("**/api/proxy/api/patients", async (route) => {
    posts++;
    await held;
    return json(route, 422, { errors: ["held"] });
  });
  await page.goto("/en-001/patients/new");
  await page.getByLabel("Family name").fill("Smith");
  await page.getByRole("button", { name: "Register", exact: true }).evaluate((el: HTMLButtonElement) => {
    el.click();
    el.click();
  });
  await expect(page.getByRole("button", { name: "Register", exact: true })).toBeDisabled();
  release();
  await expect(page.getByRole("button", { name: "Register", exact: true })).toBeEnabled();
  expect(posts).toBe(1);
});

test("two clicks at once on remove photo, and on undo this merge, send one request each", async ({ page }) => {
  const OLD = "44444444-4444-4444-8444-444444444444";
  const click2 = (b: import("@playwright/test").Locator) =>
    b.evaluate((el: HTMLButtonElement) => {
      el.click();
      el.click();
    });
  const posts = { withdraw: 0, unmerge: 0 };
  let release: () => void = () => {};
  const held = new Promise<void>((r) => (release = r));
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, photo_version: 1, replaced_by: null, replaces: [OLD] }),
  );
  await page.route(`**/api/proxy/api/patients/${ANN.id}/photo**`, async (route) => {
    if (route.request().method() === "GET")
      return route.fulfill({ status: 200, contentType: "image/png", body: PNG_BYTES });
    posts.withdraw++;
    await held;
    return json(route, 500, {});
  });
  await page.route(`**/api/proxy/api/patients/${ANN.id}/unmerge`, async (route) => {
    posts.unmerge++;
    await held;
    return json(route, 500, {});
  });
  await page.goto(`/en-001/patients/${ANN.id}`);
  await click2(page.getByRole("button", { name: "Remove photo" }));
  await expect(page.getByRole("button", { name: "Upload photo" })).toBeDisabled();
  release();
  await expect(page.getByRole("button", { name: "Upload photo" })).toBeEnabled();
  expect(posts.withdraw).toBe(1);

  await page.getByLabel(/Why are these the same person/).fill("merged in error");
  await click2(page.getByRole("button", { name: "Undo this merge" }));
  await expect(page.getByRole("button", { name: "Undo this merge" })).toBeEnabled();
  expect(posts.unmerge).toBe(1);
});

test.describe("in the patient's own time zone", () => {
test.use({ timezoneId: "Pacific/Auckland" });
test("my record narrows the access log by day, who, outcome and purpose, in the patient's zone, and keeps the filter on older entries", async ({ page }) => {
  const ID = ANN.id;
  const urls: string[] = [];
  const entry = (id: number) => ({
    id,
    at: "2026-10-21T09:00:12.000000Z",
    by: "staff",
    did: "viewed",
    what: "identity record",
    refused: false,
    purpose: "treatment",
    break_glass: false,
  });
  await page.route("**/api/proxy/api/me/patients", (route) =>
    json(route, 200, { records: [{ relationship: "self", patient: ANN }] }),
  );
  await page.route(`**/api/proxy/api/me/patients/${ID}/access-log*`, (route) => {
    urls.push(route.request().url());
    const older = route.request().url().includes("before=");
    return json(route, 200, { entries: [entry(older ? 3 : 9)], next: older ? null : 9 });
  });
  await page.goto("/en-001/my-record");
  await page.getByRole("button", { name: "Show who has accessed this record" }).click();
  // The entry's time is shown in the patient's zone, which the form names.
  await expect(page.getByText("2026-10-21 22:00 GMT+13")).toBeVisible();
  await expect(page.getByText("Days and times here are in your time zone: Pacific/Auckland.")).toBeVisible();
  await axe(page);

  // A day that is not real sends nothing.
  const before = urls.length;
  await page.getByLabel("From (day)").fill("2026-10-01");
  await page.getByLabel("Up to and including (day)").fill("2026-10-31");
  await page.getByLabel("Who", { exact: true }).selectOption("staff");
  await page.getByLabel("Outcome").selectOption("true");
  await page.getByLabel("Purpose", { exact: true }).selectOption("operations");
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await expect.poll(() => urls.length).toBe(before + 1);
  const q = new URL(urls.at(-1)!).searchParams;
  // Auckland is UTC+13 in October: its 1 October begins at 11:00 UTC on 30 September.
  expect(Object.fromEntries(q)).toEqual({ from: "2026-09-30T11:00:00Z", to: "2026-10-31T11:00:00Z", by: "staff", refused: "true", purpose: "operations" });

  // Older entries are asked for under the same filter.
  await page.getByRole("button", { name: "Show older entries" }).click();
  await expect.poll(() => urls.length).toBe(before + 2);
  expect(Object.fromEntries(new URL(urls.at(-1)!).searchParams)).toEqual({
    from: "2026-09-30T11:00:00Z",
    to: "2026-10-31T11:00:00Z",
    by: "staff",
    refused: "true",
    purpose: "operations",
    before: "9",
  });

  // Clear goes back to the whole log.
  await page.getByRole("button", { name: "Clear" }).click();
  await expect.poll(() => urls.length).toBe(before + 3);
  expect(new URL(urls.at(-1)!).search).toBe("");
});
});

test("the access-log filter form is in the page's language, and still passes axe (machine-written words)", async ({ page }) => {
  const ID = ANN.id;
  await page.route("**/api/proxy/api/me/patients", (route) =>
    json(route, 200, { records: [{ relationship: "self", patient: ANN }] }),
  );
  await page.route(`**/api/proxy/api/me/patients/${ID}/access-log*`, (route) =>
    json(route, 200, { entries: [], next: null }),
  );
  for (const [locale, from, purpose, dir] of [
    ["de-001", "Von (Tag)", "Zweck", "ltr"],
    ["ar-001", "من (يوم)", "الغرض", "rtl"],
    ["zh-cn", "从(日期)", "目的", "ltr"],
  ] as const) {
    await page.goto(`/${locale}/my-record`);
    await expect(page.locator("html")).toHaveAttribute("dir", dir);
    await page.getByRole("button", { name: "Show who has accessed this record" }).click();
    await expect(page.getByLabel(from, { exact: true })).toBeVisible();
    await expect(page.getByLabel(purpose, { exact: true })).toBeVisible();
    await axe(page);
  }
});

test("my record downloads the access log as a CSV file under the filter that is showing", async ({ page }) => {
  const ID = ANN.id;
  const urls: string[] = [];
  await page.route("**/api/proxy/api/me/patients", (route) =>
    json(route, 200, { records: [{ relationship: "self", patient: ANN }] }),
  );
  await page.route(`**/api/proxy/api/me/patients/${ID}/access-log*`, (route) => {
    urls.push(route.request().url());
    if (route.request().url().includes("format=csv"))
      return route.fulfill({
        status: 200,
        contentType: "text/csv; charset=utf-8",
        headers: { "x-next-before": "7" },
        body: "id,at_utc,by,roles,organisations,did,what,refused,purpose,emergency_access\n7,2026-10-21T09:00:12.000000Z,staff,,,viewed,identity record,false,treatment,false\n",
      });
    return json(route, 200, { entries: [], next: null });
  });
  await page.goto("/en-001/my-record");
  await page.getByRole("button", { name: "Show who has accessed this record" }).click();
  await page.getByLabel("Who", { exact: true }).selectOption("staff");
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await expect.poll(() => urls.length).toBe(2);
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: /^Download as a CSV file/ }).click();
  const file = await download;
  expect(file.suggestedFilename()).toBe("access-log.csv");
  const text = await (await import("node:fs/promises")).readFile((await file.path())!, "utf8");
  expect(text).toContain("id,at_utc,by,roles");
  expect(Object.fromEntries(new URL(urls.at(-1)!).searchParams)).toEqual({ by: "staff", format: "csv" });
  // The file was cut (the API said where the rest starts): the page says so.
  await expect(page.getByText("The file holds only the 1000 most recent entries. Narrow the days to get older ones.")).toBeVisible();
  await axe(page);
});

test("a refused merge gives its reasons in the page's language, and the API's own words when it cannot", async ({ page }) => {
  const english = "these do not look like the same person (match score 0.10; different verified NHS numbers)";
  let known = true;
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, replaced_by: null, replaces: [] }),
  );
  await page.route(`**/api/proxy/api/patients/${ANN.id}/merge`, (route) =>
    json(route, 422, {
      errors: [english],
      conflict_codes: known ? [{ code: "different_nhs_numbers" }] : [{ code: "brand_new_reason" }],
    }),
  );
  await page.goto("/de-001/patients/" + ANN.id);
  await page.getByLabel(/^Id of the record/).fill("other-id");
  await page.getByLabel(/Why are these the same person/).fill("same person");
  await page.getByRole("button", { name: "Merge into this record" }).click();
  await expect(
    page.getByText("Das scheinen nicht dieselben Personen zu sein: unterschiedliche geprüfte NHS-Nummern"),
  ).toBeVisible();
  known = false;
  await page.getByRole("button", { name: "Merge into this record" }).click();
  await expect(page.getByText(english)).toBeVisible();
});

test("a merge refused for a rule says so in the page's language, naming the other record", async ({ page }) => {
  const english = "the record to merge away was already merged into abc-123";
  let known = true;
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, replaced_by: null, replaces: [] }),
  );
  await page.route(`**/api/proxy/api/patients/${ANN.id}/merge`, (route) =>
    json(route, 422, {
      errors: [english],
      ...(known ? { refusal: "source_already_merged", refusal_into: "abc-123" } : {}),
    }),
  );
  await page.goto("/de-001/patients/" + ANN.id);
  await page.getByLabel(/^Id of the record/).fill("other-id");
  await page.getByLabel(/Why are these the same person/).fill("same person");
  await page.getByRole("button", { name: "Merge into this record" }).click();
  await expect(page.getByText("Der aufzulösende Datensatz wurde bereits mit abc-123 zusammengeführt.")).toBeVisible();
  known = false;
  await page.getByRole("button", { name: "Merge into this record" }).click();
  await expect(page.getByText(english)).toBeVisible();
});

test("the banner's age is counted on the site's calendar day, not the reader's", async ({ page }) => {
  // Born 1980-03-14. At 23:30 UTC on 13 March it is already the 14th in Auckland and still the 13th in New York.
  let zone = "Pacific/Auckland";
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, replaced_by: null, replaces: [], site_time_zone: zone }),
  );
  await page.clock.setFixedTime(new Date("2026-03-13T23:30:00Z"));
  await page.goto(`/en-001/patients/${ANN.id}`);
  const banner = page.getByRole("region", { name: "Patient banner" });
  await expect(banner).toContainText("46 years");
  zone = "America/New_York";
  await page.goto(`/en-001/patients/${ANN.id}`);
  await expect(banner).toContainText("45 years");
  // An older API sends no zone: UTC, where it is still the 13th.
  zone = undefined as unknown as string;
  await page.goto(`/en-001/patients/${ANN.id}`);
  await expect(banner).toContainText("45 years");
});

test("a vital sign is marked entered in error with a reason, sending the whole record and its version, and is kept", async ({
  page,
}) => {
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, site_time_zone: "America/New_York" }),
  );
  const mine = OBS({ id: "o-1", version: 2, range: { low: 60, high: 100 }, flag: "normal" });
  let rows: unknown[] = [mine];
  const sent: any[] = [];
  let answer = 200;
  await page.route("**/api/proxy/api/patients/*/observations/o-1", (route) => {
    sent.push(route.request().postDataJSON());
    if (answer === 200) rows = [{ ...mine, version: 3, status: "entered-in-error" }];
    return json(route, answer, answer === 200 ? rows[0] : { errors: ["x"] });
  });
  await page.route("**/api/proxy/api/patients/*/observations", (route) =>
    json(route, 200, { observations: rows }),
  );
  await page.goto(`/en-001/patients/${ANN.id}`);
  const section = page.getByRole("region", { name: "Vital signs" });
  await section.getByRole("button", { name: "Mark entered in error" }).click();
  await axe(page);
  // A reason is needed, and nothing is sent without it.
  await section.getByRole("button", { name: "Confirm entered in error" }).click();
  await expect(section.getByText("Say why this entry was a mistake.")).toBeVisible();
  expect(sent).toHaveLength(0);

  answer = 409;
  await section.getByLabel("Why this entry was a mistake").fill("wrong patient");
  await section.getByRole("button", { name: "Confirm entered in error" }).click();
  await expect(section.getByText(/changed or was merged/)).toBeVisible();

  answer = 200;
  await section.getByRole("button", { name: "Confirm entered in error" }).click();
  await expect(section.getByText("Marked entered in error. It is kept, not deleted.")).toBeVisible();
  expect(sent[1]).toEqual({
    observation: {
      code: "8867-4",
      value: 72,
      unit: "/min",
      range: { low: 60, high: 100 },
      effective_at: "2026-10-26T13:30:00.000000Z",
      status: "entered-in-error",
    },
    expected_version: 2,
    reason: "wrong patient",
  });
  await expect(section.getByText("Entered in error (1)")).toBeVisible();
  await expect(section.getByRole("button", { name: "Mark entered in error" })).toHaveCount(0);
});

test("correcting a vital loads it into the form, needs a reason, and sends a new version with the version it was made from", async ({
  page,
}) => {
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, site_time_zone: "America/New_York" }),
  );
  const mine = OBS({ id: "o-2", version: 1, value: 172, critical: { low: 40, high: 150 } });
  let rows: unknown[] = [mine];
  const sent: any[] = [];
  await page.route("**/api/proxy/api/patients/*/observations/o-2", (route) => {
    sent.push(route.request().postDataJSON());
    rows = [{ ...mine, version: 2, value: 72 }];
    return json(route, 200, rows[0]);
  });
  await page.route("**/api/proxy/api/patients/*/observations", (route) =>
    json(route, 200, { observations: rows }),
  );
  await page.goto(`/en-001/patients/${ANN.id}`);
  const section = page.getByRole("region", { name: "Vital signs" });
  await section.getByRole("button", { name: "Correct", exact: true }).click();
  await expect(section.getByLabel("Value (/min)")).toHaveValue("172");
  await expect(section.getByLabel("Measured at (site time, America/New_York)")).toHaveValue("2026-10-26T09:30");
  await axe(page);
  await section.getByLabel("Value (/min)").fill("72");
  await section.getByRole("button", { name: "Save correction" }).click();
  await expect(section.getByText("Say what you are correcting and why.")).toBeVisible();
  expect(sent).toHaveLength(0);
  await section.getByLabel("Reason for this entry (optional)").fill("keyed 172 for 72");
  await section.getByRole("button", { name: "Save correction" }).click();
  await expect(section.getByText("Correction saved as a new version.")).toBeVisible();
  expect(sent[0]).toEqual({
    observation: {
      code: "8867-4",
      value: 72,
      unit: "/min",
      critical: { low: 40, high: 150 },
      effective_at: "2026-10-26T13:30:00Z",
      status: "final",
    },
    expected_version: 1,
    reason: "keyed 172 for 72",
  });
  await expect(section.getByText("version 2")).toBeVisible();
  await expect(section.getByRole("button", { name: "Record vital sign" })).toBeVisible();
});

test("a vital's history lists every version with its reason, keeps a retracted value visible, and says when it cannot be shown", async ({
  page,
}) => {
  await stubAllergies(page);
  await page.route(`**/api/proxy/api/patients/${ANN.id}`, (route) =>
    json(route, 200, { ...ANN, site_time_zone: "America/New_York" }),
  );
  let historyStatus = 200;
  await page.route("**/api/proxy/api/patients/*/observations", (route) =>
    json(route, 200, {
      observations: [OBS({ id: "o-9", version: 3, value: 72, status: "entered-in-error" }), OBS({ id: "o-8", version: 2, value: 88 })],
    }),
  );
  await page.route("**/api/proxy/api/patients/*/observations/o-9/versions", (route) =>
    json(route, 200, {
      versions: [
        OBS({ id: "o-9", version: 1, value: 172, reason: "on arrival", author: "nurse-1" }),
        OBS({ id: "o-9", version: 2, value: 72, reason: "keyed 172 for 72", author: "nurse-2" }),
        OBS({ id: "o-9", version: 3, value: 72, status: "entered-in-error", reason: null, author: "dr-1" }),
      ],
    }),
  );
  await page.route("**/api/proxy/api/patients/*/observations/o-8/versions", (route) =>
    json(route, historyStatus, { error: "x" }),
  );
  await page.goto(`/en-001/patients/${ANN.id}`);
  const section = page.getByRole("region", { name: "Vital signs" });

  // The retracted entry's history still shows the value it replaced.
  await section.getByText("Entered in error (1)").click();
  await section.getByRole("button", { name: "History" }).nth(1).click();
  const list = section.getByRole("list", { name: "History" });
  await expect(list.getByRole("listitem")).toHaveCount(3);
  await expect(list.getByRole("listitem").nth(0)).toContainText("172 /min");
  await expect(list.getByRole("listitem").nth(0)).toContainText("Reason: on arrival");
  await expect(list.getByRole("listitem").nth(2)).toContainText("entered in error");
  await expect(list.getByRole("listitem").nth(2)).toContainText("Reason: no reason given");
  await axe(page);
  await section.getByRole("button", { name: "Hide history" }).click();
  await expect(list).toHaveCount(0);

  // Refused and failed loads say so in words.
  historyStatus = 403;
  await section.getByRole("button", { name: "History" }).first().click();
  await expect(section.getByText("The history is not shown to your role.")).toBeVisible();
  await section.getByRole("button", { name: "Hide history" }).click();
  historyStatus = 500;
  await section.getByRole("button", { name: "History" }).first().click();
  await expect(section.getByText("The history could not be loaded. The earlier versions still exist.")).toBeVisible();
});
