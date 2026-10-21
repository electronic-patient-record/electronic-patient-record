import { expect, test, type Page } from "@playwright/test";

const USERNAME = process.env.EPR_E2E_USERNAME ?? "dev.clinician";
const PASSWORD = process.env.EPR_E2E_PASSWORD ?? "";
// A second user who can sign in but holds no role (optional).
const NOROLE = process.env.EPR_E2E_NOROLE_USERNAME ?? "";

test.skip(
  !PASSWORD,
  "set EPR_E2E_PASSWORD (and see playwright.keycloak.config.ts)",
);

async function compare(page: Page) {
  // Same-origin fetch from the page, as the Patients page does.
  return page.evaluate(async () => {
    const res = await fetch("/api/proxy/api/patients/compare", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ a: {}, b: {} }),
    });
    return res.status;
  });
}

async function signIn(page: Page, username = USERNAME) {
  await page.goto("/en-001/patients");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.locator("#username").fill(username);
  await page.locator("#password").fill(PASSWORD);
  await page.locator("#kc-login").click();
  await expect(page).toHaveURL(/\/en-001\/patients$/);
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
}

test("signed out, the API route is refused and the page offers sign-in", async ({
  page,
}) => {
  await page.goto("/en-001/patients");
  await expect(page.getByRole("link", { name: "Sign in" })).toBeVisible();
  expect(await compare(page)).toBe(401);
});

test("signs in, calls the protected route, and signs out", async ({
  page,
  context,
}) => {
  await signIn(page);

  // The API accepted the reader's token.
  expect(await compare(page)).toBe(200);

  // One httpOnly session cookie; no token is readable by page scripts.
  const cookies = await context.cookies();
  const session = cookies.find((c) => c.name === "epr_session");
  expect(session?.httpOnly).toBe(true);
  expect(session?.sameSite).toBe("Lax");
  expect(session?.value).not.toContain(".");
  expect(await page.evaluate(() => document.cookie)).not.toContain(
    "epr_session",
  );
  const pageSeesToken = await page.evaluate(() =>
    JSON.stringify([localStorage, sessionStorage, document.cookie]).includes(
      "eyJ",
    ),
  );
  expect(pageSeesToken).toBe(false);

  // Sign out ends the session: the same cookie no longer works.
  const stale = session!.value;
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page.getByRole("link", { name: "Sign in" })).toBeVisible();
  expect(await compare(page)).toBe(401);
  await context.addCookies([{ ...session!, value: stale }]);
  expect(await compare(page)).toBe(401);
});

test("a signed-in user with no role is refused by the API, and the page says so", async ({
  page,
}) => {
  test.skip(
    !NOROLE,
    "set EPR_E2E_NOROLE_USERNAME to a user with no realm role",
  );
  await signIn(page, NOROLE);
  // Authenticated (the token is valid) but not authorized: 403, not 401.
  expect(await compare(page)).toBe(403);
  await page.getByRole("button", { name: "Compare" }).click();
  await expect(page.getByRole("alert")).toHaveText(
    "Your role does not allow this.",
  );
});

test("registers a patient, catches the duplicate, overrides it, and edits with a version check", async ({
  page,
  browser,
}) => {
  await signIn(page);
  const family = `Zz${Date.now()}`;
  const fill = async (p: Page) => {
    await p.goto("/en-001/patients/new");
    await p.getByLabel("Family name").fill(family);
    await p.getByLabel("Given name").fill("Ann");
    await p.getByLabel("Birth date").fill("1980-03-14");
    await p.getByLabel("NHS number").fill("943 476 5919");
    await p.getByRole("button", { name: "Register", exact: true }).click();
  };

  // A bad NHS number is refused with the reason.
  await page.goto("/en-001/patients/new");
  await page.getByLabel("Family name").fill(family);
  await page.getByLabel("NHS number").fill("9434765910");
  await page.getByRole("button", { name: "Register", exact: true }).click();
  await expect(
    page.getByText("NHS number fails the modulus 11 check"),
  ).toBeVisible();

  // Registered: the page moves to the new record, version 1.
  await fill(page);
  await expect(page).toHaveURL(/\/en-001\/patients\/[0-9a-f-]{36}$/);
  await expect(page.getByText("Version: 1")).toBeVisible();
  const first = page.url();

  // The same person again is a possible duplicate, shown side by side.
  await fill(page);
  await expect(
    page.getByRole("heading", {
      name: "This patient may already be registered",
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Open the existing record" }),
  ).toHaveAttribute("href", new URL(first).pathname);
  await page.getByRole("button", { name: "Register anyway" }).click();
  await expect(
    page.getByText("Give the reason this is a different person."),
  ).toBeVisible();
  await page.getByLabel(/This is a different person/).fill("test: twin");
  await page.getByRole("button", { name: "Register anyway" }).click();
  await expect(page).toHaveURL(/\/en-001\/patients\/[0-9a-f-]{36}$/);
  expect(page.url()).not.toBe(first);

  // Edit the first record in two windows: the second save is stale.
  const other = await browser.newPage({
    storageState: await page.context().storageState(),
  });
  await other.goto(first);
  await page.goto(first);
  await page.getByLabel("Birth date").fill("1980-03-15");
  await page.getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByRole("status")).toHaveText("Saved. Version 2.");
  await other.getByLabel("Birth date").fill("1980-03-16");
  await other.getByRole("button", { name: "Save changes" }).click();
  await expect(other.getByRole("alert")).toContainText(
    "Someone else changed this record",
  );
  await other.getByRole("button", { name: "Reload the record" }).click();
  await expect(other.getByLabel("Birth date")).toHaveValue("1980-03-15");

  // Search finds both records by family name.
  await page.goto("/en-001/patients");
  await page.getByLabel("Family name").fill(family);
  await page.getByRole("button", { name: "Search" }).click();
  await expect(page.getByRole("link", { name: /^Open/ })).toHaveCount(2);
  await other.close();
});

// Needs a user with realm role `clinical-admin` and one with `patient`, and the dev-only password
// grant (`epr-dev-cli`). The administrator links the patient user to a new record through the API;
// the patient then signs in through the UI and reads it.
const ADMIN = process.env.EPR_E2E_ADMIN_USERNAME ?? "";
const PATIENT = process.env.EPR_E2E_PATIENT_USERNAME ?? "";
const API = process.env.EPR_E2E_API_URL ?? "http://127.0.0.1:5150";
const ISSUER = process.env.EPR_E2E_ISSUER ?? "http://127.0.0.1:8081/realms/epr";

async function devToken(username: string): Promise<string> {
  const res = await fetch(`${ISSUER}/protocol/openid-connect/token`, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "password",
      client_id: "epr-dev-cli",
      username,
      password: PASSWORD,
    }),
  });
  return ((await res.json()) as { access_token: string }).access_token;
}

test("a patient signs in, sees only the record an administrator linked to them, and loses it when it ends", async ({
  page,
}) => {
  test.skip(
    !ADMIN || !PATIENT,
    "set EPR_E2E_ADMIN_USERNAME and EPR_E2E_PATIENT_USERNAME",
  );
  const admin = await devToken(ADMIN);
  const patientToken = await devToken(PATIENT);
  const sub = JSON.parse(
    Buffer.from(patientToken.split(".")[1]!, "base64url").toString(),
  ).sub as string;
  const api = async (method: string, path: string, body?: unknown) => {
    const res = await fetch(`${API}${path}`, {
      method,
      headers: {
        authorization: `Bearer ${admin}`,
        "content-type": "application/json",
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    return { status: res.status, body: (await res.json()) as any };
  };
  const family = `Zmine${Date.now()}`;
  const made = await api("POST", "/api/patients", {
    patient: { names: [{ family, given: ["Pat"] }], birth_date: "1975-05-05" },
  });
  expect(made.status).toBe(201);
  const other = await api("POST", "/api/patients", {
    patient: { names: [{ family: family + "x" }], birth_date: "1976-06-06" },
  });
  expect(other.status).toBe(201);

  // Before the link: signed in, but nothing to show.
  await signIn(page, PATIENT);
  await page.goto("/en-001/my-record");
  await expect(page.getByRole("status")).toContainText(
    "No record is linked to your sign-in",
  );

  const link = await api(
    "POST",
    `/api/patients/${made.body.id}/relationships`,
    {
      relationship: { subject: sub, kind: "self", status: "active" },
      reason: "e2e",
    },
  );
  expect(link.status).toBe(201);
  await page.reload();
  await expect(
    page.getByRole("heading", { level: 2, name: `${family}, Pat` }),
  ).toBeVisible();
  await expect(page.getByText(family + "x")).toHaveCount(0);

  // The patient cannot open the other record, or the staff pages' data.
  const status = await page.evaluate(async (id) => {
    const r = await fetch(`/api/proxy/api/patients/${id}`, {
      headers: { "x-purpose-of-use": "patient-access" },
    });
    return r.status;
  }, other.body.id);
  expect(status).toBe(403);

  const ended = await api(
    "PUT",
    `/api/patients/${made.body.id}/relationships/${link.body.id}`,
    {
      relationship: { subject: sub, kind: "self", status: "ended" },
      expected_version: 1,
    },
  );
  expect(ended.status).toBe(200);
  await page.reload();
  await expect(page.getByRole("status")).toContainText(
    "No record is linked to your sign-in",
  );
});

test("a clinician declares treatment and the API accepts it", async ({
  page,
}) => {
  await signIn(page);
  const purposes: string[] = [];
  page.on("request", (r) => {
    const p = r.headers()["x-purpose-of-use"];
    if (r.url().includes("/api/proxy/") && p) purposes.push(p);
  });
  await page.getByLabel("Purpose of use").selectOption("treatment");
  await page.getByLabel("Family name").fill(`Zp${Date.now()}`);
  await page.getByRole("button", { name: "Search" }).click();
  await expect(page.getByText("No patient matches.")).toBeVisible();
  expect(purposes).toEqual(["treatment"]);
});

test("a tampered session cookie is refused", async ({ page, context }) => {
  await signIn(page);
  const session = (await context.cookies()).find(
    (c) => c.name === "epr_session",
  )!;
  await context.addCookies([
    { ...session, value: session.value.slice(0, -2) + "AA" },
  ]);
  expect(await compare(page)).toBe(401);
});

test("a cross-site POST to the proxy and to sign-out is refused", async ({
  page,
  request,
}) => {
  await signIn(page);
  const evil = { origin: "http://evil.example" };
  const post = await page.request.post("/api/proxy/api/patients/compare", {
    headers: { ...evil, "content-type": "application/json" },
    data: { a: {}, b: {} },
  });
  expect(post.status()).toBe(403);
  const out = await page.request.post("/auth/logout", {
    headers: evil,
    maxRedirects: 0,
  });
  expect(out.status()).toBe(403);
  // Still signed in afterwards.
  expect(await compare(page)).toBe(200);
  void request;
});

test("sign-in with a forged state or no login in progress fails", async ({
  page,
}) => {
  const res = await page.goto("/auth/callback?code=x&state=y");
  expect(res!.status()).toBe(400);
  await expect(page.locator("body")).not.toContainText("mismatch");
});

test("an idle session ends", async ({ page }) => {
  test.skip(
    true,
    "idle expiry is covered by unit tests; a 900 s wait is not run here",
  );
  void page;
});

test("a patient sees that a clinician viewed their record, as staff without a name", async ({
  page,
}) => {
  test.skip(
    !ADMIN || !PATIENT,
    "set EPR_E2E_ADMIN_USERNAME and EPR_E2E_PATIENT_USERNAME",
  );
  const sub = async (t: string) =>
    JSON.parse(Buffer.from(t.split(".")[1]!, "base64url").toString())
      .sub as string;
  const admin = await devToken(ADMIN);
  const clinician = await devToken(USERNAME);
  const patientToken = await devToken(PATIENT);
  const call = async (
    token: string,
    method: string,
    path: string,
    body?: unknown,
    purpose = "operations",
  ) => {
    const res = await fetch(`${API}${path}`, {
      method,
      headers: {
        authorization: `Bearer ${token}`,
        "content-type": "application/json",
        "x-purpose-of-use": purpose,
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    return { status: res.status, body: (await res.json()) as any };
  };
  const made = await call(admin, "POST", "/api/patients", {
    patient: {
      names: [{ family: `Zlog${Date.now()}`, given: ["Lou"] }],
      birth_date: "1962-02-02",
    },
  });
  expect(made.status).toBe(201);
  const id = made.body.id as string;
  for (const [who, kind] of [
    [await sub(patientToken), "self"],
    [await sub(clinician), "care-team"],
  ] as const) {
    const r = await call(admin, "POST", `/api/patients/${id}/relationships`, {
      relationship: { subject: who, kind, status: "active" },
      reason: "e2e",
    });
    expect(r.status).toBe(201);
  }
  // The clinician opens the record, declaring treatment.
  const seen = await call(
    clinician,
    "GET",
    `/api/patients/${id}`,
    undefined,
    "treatment",
  );
  expect(seen.status).toBe(200);

  await signIn(page, PATIENT);
  await page.goto("/en-001/my-record");
  await page
    .getByRole("button", { name: "Show who has accessed this record" })
    .click();
  await expect(
    page.getByText(
      /A member of staff \(clinician(, [a-z-]+)?\) viewed your identity record\. Purpose: treatment\./,
    ),
  ).toBeVisible();
  // No staff identifier reaches the page.
  expect(await page.content()).not.toContain(await sub(clinician));
});
