// The purpose of use a staff member declares for what they do next. It travels as
// `X-Purpose-Of-Use`, the API checks it against the role, and the audit trail records it. No
// runes and no `$app/*` imports, so unit tests can use it.
//
// Only the purposes a staff role can use today are offered. `emergency` is left out until
// break-the-glass (SE-3) exists: declaring it would suggest an override that does not happen.
// `payment`, `research` and `audit` are permitted to no staff role.

export const STAFF_PURPOSES = ["operations", "treatment"] as const;
export type StaffPurpose = (typeof STAFF_PURPOSES)[number];

/** What the API assumes when none is sent. */
export const DEFAULT_PURPOSE: StaffPurpose = "operations";

export const STORAGE_KEY = "epr.purpose";

export function parseStaffPurpose(value: unknown): StaffPurpose | null {
  return (STAFF_PURPOSES as readonly unknown[]).includes(value)
    ? (value as StaffPurpose)
    : null;
}

/** The remembered choice for this browser tab, or the default. Storage may be blocked. */
export function readStoredPurpose(
  storage: Pick<Storage, "getItem"> | undefined,
): StaffPurpose {
  try {
    return parseStaffPurpose(storage?.getItem(STORAGE_KEY)) ?? DEFAULT_PURPOSE;
  } catch {
    return DEFAULT_PURPOSE;
  }
}

export function storePurpose(
  storage: Pick<Storage, "setItem"> | undefined,
  value: StaffPurpose,
): void {
  try {
    storage?.setItem(STORAGE_KEY, value);
  } catch {
    // Not remembered; the choice still applies until the page is reloaded.
  }
}
