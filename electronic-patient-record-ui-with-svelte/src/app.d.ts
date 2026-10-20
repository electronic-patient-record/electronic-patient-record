// See https://svelte.dev/docs/kit/types#app
declare global {
  namespace App {
    interface Error {
      code?: string;
      details?: unknown;
    }
    // The locale named by the URL prefix (`/cy-001/…`), set in `hooks.server.ts`;
    // absent on an unlocalized path (`/api/…`).
    interface Locals {
      locale?: import("#lib/locales.js").Locale;
    }
    interface PageData {
      title?: string;
    }
    interface PageState {}
    interface Platform {}
  }
}

export {};
