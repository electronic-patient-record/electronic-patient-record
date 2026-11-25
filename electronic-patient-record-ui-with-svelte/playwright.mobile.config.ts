import { defineConfig, devices } from "@playwright/test";
import base from "./playwright.config";

// The mobile audit (XP-1): the same tests on two phones, 320 CSS px wide. Not a gate, and not in CI, until XP-2 passes:
//
//   MOBILE_AUDIT=/tmp/mobile.jsonl PW_PORT=4187 pnpm exec playwright test -c playwright.mobile.config.ts
//
// Tests written for a wide window may fail here for reasons of their own; the audit is the file.
export default defineConfig({
  ...base,
  // So that a control that cannot be reached is named, and does not only time the test out.
  use: { ...base.use, actionTimeout: 5_000 },
  projects: [
    {
      name: "phone",
      use: { ...devices["Pixel 7"], viewport: { width: 320, height: 640 } },
    },
    { name: "iphone-se", use: { ...devices["iPhone SE"] } },
  ],
});
