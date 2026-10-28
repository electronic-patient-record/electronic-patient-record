import { expect, test as base } from "@playwright/test";

// A page that waits for hydration after every navigation. The layout sets `data-hydrated` on <html> once
// the handlers are attached; a click before that is a native submit that does nothing, which made
// "an empty search sends nothing and says why" fail about once in forty runs on a loaded machine.
export const test = base.extend({
  page: async ({ page }, use) => {
    const hydrated = () =>
      page
        .waitForSelector("html[data-hydrated]", {
          state: "attached",
          timeout: 5000,
        })
        .catch(() => {});
    const goto = page.goto.bind(page);
    const reload = page.reload.bind(page);
    page.goto = (async (url: string, options?: Parameters<typeof goto>[1]) => {
      const response = await goto(url, options);
      await hydrated();
      return response;
    }) as typeof page.goto;
    page.reload = (async (options?: Parameters<typeof reload>[0]) => {
      const response = await reload(options);
      await hydrated();
      return response;
    }) as typeof page.reload;
    await use(page);
  },
});
export { expect };
