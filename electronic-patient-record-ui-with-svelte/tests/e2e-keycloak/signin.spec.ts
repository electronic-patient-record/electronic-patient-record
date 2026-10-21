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
    page.getByRole("heading", {
      level: 2,
      name: `${family}, Pat`,
      exact: true,
    }),
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

test("the FHIR Patient endpoint creates, reads, searches and refuses a stale update", async () => {
  test.skip(!ADMIN, "set EPR_E2E_ADMIN_USERNAME");
  const admin = await devToken(ADMIN);
  const fhir = async (
    method: string,
    path: string,
    body?: unknown,
    headers: Record<string, string> = {},
  ) => {
    const res = await fetch(`${API}/fhir/R4${path}`, {
      method,
      headers: {
        authorization: `Bearer ${admin}`,
        "content-type": "application/fhir+json",
        ...headers,
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    return {
      status: res.status,
      etag: res.headers.get("etag"),
      body: (await res.json()) as any,
    };
  };
  const family = `Zfhir${Date.now()}`;
  const patient = {
    resourceType: "Patient",
    name: [{ family, given: ["Ada"] }],
    birthDate: "1971-01-02",
  };
  const made = await fhir("POST", "/Patient", patient);
  expect(made.status).toBe(201);
  expect(made.etag).toBe('W/"1"');
  const id = made.body.id as string;

  const read = await fhir("GET", `/Patient/${id}`);
  expect(read.status).toBe(200);
  expect(read.body.resourceType).toBe("Patient");

  const found = await fhir("GET", `/Patient?family=${family}`);
  expect(found.body.resourceType).toBe("Bundle");
  expect(found.body.entry).toHaveLength(1);

  // A possible duplicate is refused without showing the other record.
  const dup = await fhir("POST", "/Patient", patient);
  expect(dup.status).toBe(409);
  expect(JSON.stringify(dup.body)).not.toContain(id);

  // Elements the record cannot hold are refused, not dropped.
  const extra = await fhir("POST", "/Patient", {
    ...patient,
    name: [{ family: family + "x" }],
    maritalStatus: { text: "M" },
  });
  expect(extra.status).toBe(422);

  const changed = { ...patient, birthDate: "1971-01-03" };
  expect((await fhir("PUT", `/Patient/${id}`, changed)).status).toBe(428);
  const ok = await fhir("PUT", `/Patient/${id}`, changed, {
    "if-match": 'W/"1"',
  });
  expect(ok.status).toBe(200);
  expect(ok.etag).toBe('W/"2"');
  const stale = await fhir("PUT", `/Patient/${id}`, changed, {
    "if-match": 'W/"1"',
  });
  expect(stale.status).toBe(412);
});

const AUDITOR = process.env.EPR_E2E_AUDITOR_USERNAME ?? "";

test("only an auditor can search the audit trail, and each search is itself recorded", async () => {
  test.skip(
    !ADMIN || !AUDITOR,
    "set EPR_E2E_ADMIN_USERNAME and EPR_E2E_AUDITOR_USERNAME",
  );
  const get = async (user: string, path: string) => {
    const res = await fetch(`${API}${path}`, {
      headers: { authorization: `Bearer ${await devToken(user)}` },
    });
    return {
      status: res.status,
      body: (await res.json().catch(() => null)) as any,
    };
  };
  // A non-auditor is refused, and the refusal is in the trail the auditor then reads.
  expect((await get(ADMIN, "/api/audit")).status).toBe(403);
  expect((await get(ADMIN, "/fhir/R4/AuditEvent")).status).toBe(403);
  const refused = await get(AUDITOR, "/api/audit?outcome=4&purpose=");
  expect(refused.status).toBe(200);
  expect(
    refused.body.events.some(
      (e: any) => e.resource === "/api/audit" && e.outcome === 4,
    ),
  ).toBe(true);

  // The auditor's own search wrote a row naming the criteria, not their values.
  const mine = await get(AUDITOR, "/api/audit?action=R");
  const search = mine.body.events.find(
    (e: any) =>
      e.resource === "AuditLog" && e.detail?.search?.includes("outcome"),
  );
  expect(search?.detail.search).toEqual(["outcome"]);

  const fhir = await get(AUDITOR, "/fhir/R4/AuditEvent?outcome=4");
  expect(fhir.body.resourceType).toBe("Bundle");
  expect(fhir.body.entry[0].resource.resourceType).toBe("AuditEvent");
});

test("a patient's photo is stored as bytes, refused if it is not an image, and shown in their banner", async ({
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
  const json = async (method: string, path: string, body?: unknown) => {
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
  const made = await json("POST", "/api/patients", {
    patient: {
      names: [{ family: `Zphoto${Date.now()}`, given: ["Pic"] }],
      birth_date: "1955-05-05",
    },
  });
  expect(made.status).toBe(201);
  const id = made.body.id as string;
  expect(
    (
      await json("POST", `/api/patients/${id}/relationships`, {
        relationship: { subject: sub, kind: "self", status: "active" },
        reason: "e2e",
      })
    ).status,
  ).toBe(201);

  const png = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==",
    "base64",
  );
  const put = (bytes: Buffer, v = 0) =>
    fetch(`${API}/api/patients/${id}/photo?expected_version=${v}`, {
      method: "PUT",
      headers: {
        authorization: `Bearer ${admin}`,
        "content-type": "image/png",
      },
      body: bytes,
    });
  // Not an image, whatever the header claims.
  const svg = await put(Buffer.from("<svg onload=alert(1)>"));
  expect(svg.status).toBe(422);
  const ok = await put(png);
  expect(ok.status).toBe(200);
  expect((await put(png)).status).toBe(409);

  // The patient's own banner shows it, fetched under patient-access through the proxy.
  await signIn(page, PATIENT);
  await page.goto("/en-001/my-record");
  const img = page.getByRole("img", { name: /^Photo of / });
  await expect(img).toBeVisible();
  expect(await img.evaluate((i: HTMLImageElement) => i.naturalWidth)).toBe(1);

  // The raw route is private and unsniffed.
  const raw = await fetch(`${API}/api/patients/${id}/photo`, {
    headers: { authorization: `Bearer ${admin}` },
  });
  expect(raw.headers.get("content-type")).toBe("image/png");
  expect(raw.headers.get("x-content-type-options")).toBe("nosniff");
  expect(raw.headers.get("cache-control")).toContain("no-store");
  expect(Buffer.from(await raw.arrayBuffer()).equals(png)).toBe(true);
});

test("allergies are clinical content: a care relationship is needed, the patient sees them in their banner, and the audit rows hold no substance", async ({
  page,
}) => {
  test.skip(
    !ADMIN || !PATIENT || !AUDITOR,
    "set EPR_E2E_ADMIN_USERNAME, EPR_E2E_PATIENT_USERNAME and EPR_E2E_AUDITOR_USERNAME",
  );
  const subOf = (t: string) =>
    JSON.parse(Buffer.from(t.split(".")[1]!, "base64url").toString())
      .sub as string;
  const [admin, clinician, patientToken, auditor] = await Promise.all([
    devToken(ADMIN),
    devToken(USERNAME),
    devToken(PATIENT),
    devToken(AUDITOR),
  ]);
  const api = async (
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
    // The route gate answers a plain "forbidden", not JSON.
    const text = await res.text();
    let parsed: any = text;
    try {
      parsed = JSON.parse(text);
    } catch {
      /* plain text */
    }
    return { status: res.status, body: parsed };
  };
  // A birth date no earlier run used, so the duplicate check stays quiet.
  const dob = () =>
    `19${40 + Math.floor(Math.random() * 40)}-0${1 + Math.floor(Math.random() * 9)}-${10 + Math.floor(Math.random() * 18)}`;
  const make = async (given: string, _unused?: string) => {
    const r = await api(admin, "POST", "/api/patients", {
      patient: {
        names: [{ family: `Zallergy${Date.now()}${given}`, given: [given] }],
        birth_date: dob(),
      },
    });
    expect(r.status).toBe(201);
    return r.body.id as string;
  };
  const withRel = async (id: string, subject: string, kind: string) =>
    expect(
      (
        await api(admin, "POST", `/api/patients/${id}/relationships`, {
          relationship: { subject, kind, status: "active" },
          reason: "e2e",
        })
      ).status,
    ).toBe(201);

  const id = await make("Alex", "1949-04-04");
  const stranger = await make("Blair", "1950-05-05");
  await withRel(id, subOf(patientToken), "self");
  await withRel(id, subOf(clinician), "care-team");

  // The clinician with a relationship records a high-risk allergy, declaring treatment.
  const made = await api(
    clinician,
    "POST",
    `/api/patients/${id}/allergies`,
    {
      allergy: {
        substance: "Penicillin",
        criticality: "high",
        category: "medication",
        status: "active",
      },
    },
    "treatment",
  );
  expect(made.status).toBe(201);
  const read = await api(
    clinician,
    "GET",
    `/api/patients/${id}/allergies`,
    undefined,
    "treatment",
  );
  expect(read.body.summary.state).toBe("active");
  // "No known allergies" cannot contradict the record.
  const none = await api(
    clinician,
    "PUT",
    `/api/patients/${id}/allergy-statement`,
    { none_known: true, expected_version: 0 },
    "treatment",
  );
  expect(none.status).toBe(422);

  // An administrator has no allergy permission; a clinician with no relationship is refused.
  expect(
    (await api(admin, "GET", `/api/patients/${id}/allergies`)).status,
  ).toBe(403);
  expect(
    (
      await api(
        clinician,
        "GET",
        `/api/patients/${stranger}/allergies`,
        undefined,
        "treatment",
      )
    ).status,
  ).toBe(403);

  // The patient's own banner carries it.
  await signIn(page, PATIENT);
  await page.goto("/en-001/my-record");
  await expect(
    page
      .locator(`section[aria-labelledby="rec-${id}"]`)
      .getByRole("region", { name: "Patient banner" })
      .locator("p.allergies"),
  ).toHaveText("Allergies: Penicillin (high risk)");

  // The trail records the access, and not the substance.
  const trail = await api(
    auditor,
    "GET",
    `/api/audit?patient=${id}`,
    undefined,
    "audit",
  );
  const rows = trail.body.events.filter((e: any) =>
    e.resource?.startsWith("AllergyIntolerance"),
  );
  expect(rows.length).toBeGreaterThanOrEqual(3);
  expect(JSON.stringify(trail.body)).not.toContain("Penicillin");
  // The clinician's refused read of a stranger's allergies is in the trail too.
  const refused = await api(
    auditor,
    "GET",
    `/api/audit?patient=${stranger}&outcome=4`,
    undefined,
    "audit",
  );
  expect(
    refused.body.events.some((e: any) =>
      String(e.resource).startsWith("AllergyIntolerance"),
    ),
  ).toBe(true);
});

test("an administrator grants, ends and restricts access through the page, and the API follows", async ({
  page,
}) => {
  test.skip(!ADMIN, "set EPR_E2E_ADMIN_USERNAME");
  const [admin, clinician] = await Promise.all([
    devToken(ADMIN),
    devToken(USERNAME),
  ]);
  const clinicianSub = JSON.parse(
    Buffer.from(clinician.split(".")[1]!, "base64url").toString(),
  ).sub as string;
  const dob = `19${40 + Math.floor(Math.random() * 40)}-0${1 + Math.floor(Math.random() * 9)}-${10 + Math.floor(Math.random() * 18)}`;
  const made = await fetch(`${API}/api/patients`, {
    method: "POST",
    headers: {
      authorization: `Bearer ${admin}`,
      "content-type": "application/json",
    },
    body: JSON.stringify({
      patient: {
        names: [{ family: `Zaccess${Date.now()}`, given: ["Pat"] }],
        birth_date: dob,
      },
    }),
  });
  expect(made.status).toBe(201);
  const id = ((await made.json()) as any).id as string;
  const read = async () =>
    (
      await fetch(`${API}/api/patients/${id}/allergies`, {
        headers: {
          authorization: `Bearer ${clinician}`,
          "x-purpose-of-use": "treatment",
        },
      })
    ).status;
  // Allergies are clinical content: no relationship, no access.
  expect(await read()).toBe(403);

  await signIn(page, ADMIN);
  await page.goto(`/en-001/patients/${id}/access`);
  await page.getByLabel(/^Person/).fill(clinicianSub);
  await page.getByRole("button", { name: "Record relationship" }).click();
  await expect(page.getByText("Saved.").first()).toBeVisible();
  await expect(page.locator("li", { hasText: clinicianSub })).toContainText(
    "In force",
  );
  expect(await read()).toBe(200);

  // The patient's consent beats the relationship: deny clinicians, and the same read is refused.
  await page.getByLabel("clinician", { exact: true }).check();
  await page.getByRole("button", { name: "Record consent" }).click();
  await expect(
    page.locator("li", { hasText: "Deny: clinician" }),
  ).toBeVisible();
  expect(await read()).toBe(403);
  await page
    .locator("li", { hasText: "Deny: clinician" })
    .getByRole("button", { name: "Withdraw" })
    .click();
  await expect(page.locator("li", { hasText: "Withdrawn" })).toBeVisible();
  expect(await read()).toBe(200);

  // Ending the relationship takes the access away again.
  await page.getByRole("button", { name: "End relationship" }).click();
  await expect(page.locator("li", { hasText: clinicianSub })).toContainText(
    "Ended",
  );
  expect(await read()).toBe(403);
});

test("merging two records is a link: allergies and consent follow it, the old record goes read-only, and unmerging restores both", async () => {
  test.skip(
    !ADMIN || !AUDITOR,
    "set EPR_E2E_ADMIN_USERNAME and EPR_E2E_AUDITOR_USERNAME",
  );
  const [admin, clinician, auditor] = await Promise.all([
    devToken(ADMIN),
    devToken(USERNAME),
    devToken(AUDITOR),
  ]);
  const subOf = (t: string) =>
    JSON.parse(Buffer.from(t.split(".")[1]!, "base64url").toString())
      .sub as string;
  const api = async (
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
    const text = await res.text();
    let parsed: any = text;
    try {
      parsed = JSON.parse(text);
    } catch {
      /* plain text */
    }
    return { status: res.status, body: parsed };
  };
  const family = `Zmerge${Date.now()}`;
  const dob = `19${40 + Math.floor(Math.random() * 40)}-0${1 + Math.floor(Math.random() * 9)}-${10 + Math.floor(Math.random() * 18)}`;
  const person = { names: [{ family, given: ["Robin"] }], birth_date: dob };
  const first = await api(admin, "POST", "/api/patients", { patient: person });
  expect(first.status).toBe(201);
  // The second registration is a duplicate; registering anyway needs a reason.
  const second = await api(admin, "POST", "/api/patients", {
    patient: person,
    override_reason: "e2e: a deliberate duplicate to merge",
  });
  expect(second.status).toBe(201);
  const [target, source] = [first.body.id as string, second.body.id as string];
  const link = (id: string) =>
    api(admin, "POST", `/api/patients/${id}/relationships`, {
      relationship: {
        subject: subOf(clinician),
        kind: "care-team",
        status: "active",
      },
      reason: "e2e",
    });
  expect((await link(target)).status).toBe(201);
  expect((await link(source)).status).toBe(201);

  // An allergy is recorded on the record that will be merged away.
  const allergy = await api(
    clinician,
    "POST",
    `/api/patients/${source}/allergies`,
    {
      allergy: {
        substance: "Penicillin",
        criticality: "high",
        status: "active",
      },
    },
    "treatment",
  );
  expect(allergy.status).toBe(201);
  const allergiesOf = async (id: string) =>
    (
      await api(
        clinician,
        "GET",
        `/api/patients/${id}/allergies`,
        undefined,
        "treatment",
      )
    ).body;
  expect((await allergiesOf(target)).allergies).toHaveLength(0);

  // Only an administrator merges; a clinician is refused by the gate.
  expect(
    (
      await api(clinician, "POST", `/api/patients/${target}/merge`, {
        source,
        reason: "x",
      })
    ).status,
  ).toBe(403);
  // A reason is required, and a different person is refused with the matcher's words.
  expect(
    (
      await api(admin, "POST", `/api/patients/${target}/merge`, {
        source,
        reason: "  ",
      })
    ).status,
  ).toBe(422);
  const stranger = await api(admin, "POST", "/api/patients", {
    patient: {
      names: [{ family: `Zother${Date.now()}`, given: ["Quinn"] }],
      birth_date: "1999-12-31",
    },
  });
  const refused = await api(admin, "POST", `/api/patients/${target}/merge`, {
    source: stranger.body.id,
    reason: "wrong",
  });
  expect(refused.status).toBe(422);
  expect(refused.body.errors[0]).toContain("do not look like the same person");

  const merged = await api(admin, "POST", `/api/patients/${target}/merge`, {
    source,
    reason: "e2e: same person registered twice",
  });
  expect(merged.status).toBe(201);
  expect(merged.body.status).toBe("active");

  // The surviving record now shows the allergy, which still names the record it was made on.
  const after = await allergiesOf(target);
  expect(after.summary.state).toBe("active");
  expect(after.allergies[0].patient_id).toBe(source);
  // The old record says where it went, is not found by search, and refuses changes.
  const old = await api(admin, "GET", `/api/patients/${source}`);
  expect(old.body.replaced_by).toBe(target);
  expect(
    (await api(admin, "GET", `/api/patients/${target}`)).body.replaces,
  ).toEqual([source]);
  const found = await api(admin, "GET", `/api/patients?family=${family}`);
  expect(found.body.results.map((r: any) => r.id)).toEqual([target]);
  const edit = await api(admin, "PUT", `/api/patients/${source}`, {
    patient: person,
    expected_version: 1,
  });
  expect(edit.status).toBe(409);
  expect(edit.body.replaced_by).toBe(target);
  // FHIR says the same.
  const fhir = await fetch(`${API}/fhir/R4/Patient/${source}`, {
    headers: { authorization: `Bearer ${admin}` },
  });
  const fhirBody = (await fhir.json()) as any;
  expect(fhirBody.active).toBe(false);
  expect(fhirBody.link[0]).toMatchObject({
    type: "replaced-by",
    other: { reference: `Patient/${target}` },
  });

  // A refusal recorded on the old record still protects the surviving one.
  const deny = await api(admin, "POST", `/api/patients/${source}/consents`, {
    consent: {
      kind: "general",
      status: "active",
      provisions: [
        {
          effect: "deny",
          actors: ["clinician"],
          purposes: [],
          resources: [],
          labels: [],
        },
      ],
    },
  });
  expect(deny.status).toBe(201);
  expect(
    (
      await api(
        clinician,
        "GET",
        `/api/patients/${target}/allergies`,
        undefined,
        "treatment",
      )
    ).status,
  ).toBe(403);
  const withdrawn = await api(
    admin,
    "PUT",
    `/api/patients/${source}/consents/${deny.body.id}`,
    {
      consent: { ...deny.body.consent, status: "inactive" },
      expected_version: 1,
    },
  );
  expect(withdrawn.status).toBe(200);
  expect((await allergiesOf(target)).allergies).toHaveLength(1);

  // Unmerge: both records are exactly as they were.
  const undone = await api(admin, "POST", `/api/patients/${target}/unmerge`, {
    source,
    reason: "e2e: undo",
  });
  expect(undone.status).toBe(200);
  expect(undone.body.status).toBe("reversed");
  expect((await allergiesOf(target)).allergies).toHaveLength(0);
  expect((await allergiesOf(source)).allergies).toHaveLength(1);
  expect(
    (await api(admin, "GET", `/api/patients/${source}`)).body.replaced_by,
  ).toBeNull();
  const again = await api(admin, "GET", `/api/patients?family=${family}`);
  expect(again.body.results).toHaveLength(2);
  expect(
    (
      await api(admin, "PUT", `/api/patients/${source}`, {
        patient: person,
        expected_version: 1,
      })
    ).status,
  ).toBe(200);

  // Merge and unmerge are in the trail, with no demographics and no reason text.
  const trail = await api(
    auditor,
    "GET",
    `/api/audit?patient=${target}`,
    undefined,
    "audit",
  );
  const rows = trail.body.events.filter(
    (e: any) => e.resource === "PatientLink",
  );
  expect(rows.length).toBeGreaterThanOrEqual(2);
  const text = JSON.stringify(trail.body);
  expect(text).not.toContain(family);
  expect(text).not.toContain("same person registered twice");
});

// The refused-message queue (OR-3). Needs the API started with EPR_MLLP_BIND=127.0.0.1:<port>, and that
// port in EPR_E2E_MLLP_PORT. An ORU message is refused by the feed; the administrator finds it in the
// queue, reads it and closes it; a clinician is refused the queue.
const MLLP_PORT = Number(process.env.EPR_E2E_MLLP_PORT ?? 0);

test("a refused HL7 message waits in the queue for an administrator, and a clinician cannot see it", async ({
  page,
}) => {
  test.skip(
    !ADMIN || !MLLP_PORT,
    "set EPR_E2E_ADMIN_USERNAME and EPR_E2E_MLLP_PORT",
  );
  const { connect } = await import("node:net");
  const control = `E2E${Date.now()}`;
  const message = `MSH|^~\\&|LAB|CLINIC|EPR|HOSP|20261021090000||ORU^R01^ORU_R01|${control}|P|2.5\rPID|1||X1^^^LOCAL||Zqueue^Test||19800101|F\r`;
  const ack = await new Promise<string>((resolve, reject) => {
    const socket = connect(MLLP_PORT, "127.0.0.1", () =>
      socket.write(
        Buffer.concat([
          Buffer.from([0x0b]),
          Buffer.from(message),
          Buffer.from([0x1c, 0x0d]),
        ]),
      ),
    );
    socket.once("data", (d) => {
      socket.end();
      resolve(d.toString());
    });
    socket.once("error", reject);
  });
  expect(ack).toContain("MSA|AR|" + control);

  const clinician = await devToken(USERNAME);
  const asClinician = await fetch(`${API}/api/feed/refusals`, {
    headers: { authorization: `Bearer ${clinician}` },
  });
  expect(asClinician.status).toBe(403);

  await signIn(page, ADMIN);
  await page.goto("/en-001/admin/feed");
  const entry = page.locator("li", { hasText: control }).first();
  await expect(entry).toContainText("Not a message type the feed handles");
  await entry.getByRole("button", { name: "Show the message" }).click();
  await expect(page.locator("pre")).toContainText("PID|1||X1^^^LOCAL");
  await page.getByLabel("Outcome").selectOption("dismissed");
  await page
    .getByLabel("Reason (required)")
    .fill("laboratory results are not handled here");
  await page.getByRole("button", { name: "Close this entry" }).click();
  await expect(page.getByText("Closed.")).toBeVisible();
  await expect(page.locator("li", { hasText: control })).toHaveCount(0);
});
