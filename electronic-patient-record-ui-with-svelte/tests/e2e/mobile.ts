import { appendFileSync } from "node:fs";
import type { Page } from "@playwright/test";

// The mobile audit (XP-1). Off unless `MOBILE_AUDIT` names a file: then each place where a test runs axe is also
// measured, one JSON line each, and nothing fails. Run with `playwright.mobile.config.ts`.
//
// `overflow`: px the page scrolls sideways, with the elements that stick out. `small`: controls under 24 by 24 CSS px
// (WCAG 2.5.8), links inside a sentence left out, since the criterion excepts them; its spacing exception is not worked
// out, so some of these pass. `cut`: banner text that is clipped. `bannerGone`: the banner is out of sight at the end of the
// page. `bannerShare`: the part of the screen's height, in percent, that a banner which stays in sight takes.
export async function mobileAudit(page: Page, title: string, project: string) {
  const file = process.env.MOBILE_AUDIT;
  if (!file) return;
  const found = await page.evaluate(() => {
    const doc = document.documentElement;
    const name = (e: Element) =>
      `${e.tagName.toLowerCase()}${e.className && typeof e.className === "string" ? `.${e.className.split(" ")[0]}` : ""} "${(
        e.getAttribute("aria-label") ??
        e.textContent ??
        ""
      )
        .trim()
        .replace(/\s+/g, " ")
        .slice(0, 28)}"`;
    const seen = (e: Element) => {
      const r = e.getBoundingClientRect();
      return (
        r.width > 0 &&
        r.height > 0 &&
        getComputedStyle(e).visibility !== "hidden"
      );
    };
    const overflow = doc.scrollWidth - doc.clientWidth;
    const out =
      overflow > 1
        ? [...document.body.querySelectorAll("*")]
            .filter(
              (e) =>
                seen(e) &&
                e.getBoundingClientRect().right > doc.clientWidth + 1 &&
                ![...e.children].some(
                  (c) => c.getBoundingClientRect().right > doc.clientWidth + 1,
                ),
            )
            .map(name)
        : [];
    const inSentence = (e: Element) =>
      e.tagName === "A" &&
      [...(e.parentElement?.childNodes ?? [])].some(
        (n) =>
          n.nodeType === Node.TEXT_NODE &&
          (n.textContent ?? "").trim().length > 3,
      );
    const small = [
      ...document.querySelectorAll(
        "a[href], button, input:not([type=hidden]), select, textarea, summary, [role=button]",
      ),
    ]
      .filter((e) => {
        const label = e.closest("label") ?? e;
        const r = label.getBoundingClientRect();
        return seen(e) && !inSentence(e) && (r.width < 24 || r.height < 24);
      })
      .map((e) => {
        const r = (e.closest("label") ?? e).getBoundingClientRect();
        return `${name(e)} ${Math.round(r.width)}x${Math.round(r.height)}`;
      });
    const banner = document.querySelector("[aria-label='Patient banner']");
    const cut = banner
      ? [banner, ...banner.querySelectorAll("*")]
          .filter(
            (e) =>
              e.scrollWidth > e.clientWidth + 1 &&
              getComputedStyle(e).overflowX !== "visible",
          )
          .map(name)
      : [];
    const wide = [...document.querySelectorAll("table")]
      .filter((t) => seen(t) && t.scrollWidth > doc.clientWidth)
      .map(name);
    window.scrollTo(0, doc.scrollHeight);
    const box = banner?.getBoundingClientRect();
    const bannerGone = box ? box.bottom < 0 : null;
    const bannerShare =
      box && getComputedStyle(banner as Element).position === "sticky"
        ? Math.round((100 * box.height) / window.innerHeight)
        : null;
    window.scrollTo(0, 0);
    return {
      overflow,
      out: [...new Set(out)].slice(0, 8),
      small: [...new Set(small)],
      cut,
      wide,
      bannerGone,
      bannerShare,
      height: doc.scrollHeight,
    };
  });
  const path = new URL(page.url()).pathname;
  appendFileSync(
    file,
    `${JSON.stringify({ project, path, title, ...found })}\n`,
  );
}
