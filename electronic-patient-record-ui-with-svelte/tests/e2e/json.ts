// A request body a test reads back. Playwright's `postDataJSON()` returns `any`; casting it to
// `Json` at the point it is stored keeps `any` out of the tests: a property is a `Json` (so a
// nested read still works) and anything that is not a `Json` must be said.
export interface Json {
  [key: string]: Json;
}
