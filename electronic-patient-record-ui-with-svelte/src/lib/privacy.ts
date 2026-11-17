// The privacy notice (`/privacy`): everything this app keeps in the browser, and why. A row per cookie or storage key;
// `tests/unit/privacy.test.ts` fails when the code stores a name this list does not have, or a name here changes in
// the code without changing here. The words are in `strings.ts` (English, the source) and `strings.cy.ts` (Welsh,
// machine-written, not reviewed); other locales read English, as for the rest of the UI.

/** Where a value lives. */
export type Place = "cookie" | "cookieHttpOnly" | "session";

/** One thing kept in the browser. `what`, `when` and `kept` name keys of `TEXT.privacyRows`. */
export interface Kept {
  /** The cookie or storage name, exactly as the code writes it. Over HTTPS the sign-in cookies take a `__Host-` prefix. */
  name: string;
  place: Place;
  /** Key into `TEXT.privacyRows`. */
  row: "theme" | "locale" | "purpose" | "login" | "session";
  /** Only when sign-in is turned on. */
  signInOnly: boolean;
}

export const KEPT: readonly Kept[] = [
  { name: "epr-theme", place: "cookie", row: "theme", signInOnly: false },
  { name: "locale", place: "cookie", row: "locale", signInOnly: false },
  { name: "epr.purpose", place: "session", row: "purpose", signInOnly: false },
  {
    name: "epr_login",
    place: "cookieHttpOnly",
    row: "login",
    signInOnly: true,
  },
  {
    name: "epr_session",
    place: "cookieHttpOnly",
    row: "session",
    signInOnly: true,
  },
];
