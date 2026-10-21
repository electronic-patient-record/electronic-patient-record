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
