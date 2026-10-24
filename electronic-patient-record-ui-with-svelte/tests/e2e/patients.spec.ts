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
