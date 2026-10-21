import { splitPath } from "#lib/locales.js";
import type { LayoutLoad } from "./$types";

// The UI locale is read from the URL prefix (`/cy-001/…`).
export const load: LayoutLoad = ({ url, data }) => ({
  ...data,
  locale: splitPath(url.pathname).locale ?? undefined,
});
