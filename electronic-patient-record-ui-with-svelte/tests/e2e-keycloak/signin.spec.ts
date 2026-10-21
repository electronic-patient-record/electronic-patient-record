import { expect, test, type Page } from "@playwright/test";

const USERNAME = process.env.EPR_E2E_USERNAME ?? "dev.clinician";
const PASSWORD = process.env.EPR_E2E_PASSWORD ?? "";

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

async function signIn(page: Page) {
  await page.goto("/en-001/patients");
  await page.getByRole("link", { name: "Sign in" }).click();
  await page.locator("#username").fill(USERNAME);
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
