// How old the record on screen is (EC-12). A clinician must be able to tell a record read a moment ago
// from one that could not be refreshed. Pure, so a unit test can use it: the page passes the times.

/** The page looks again once a minute while visible; two missed looks and more make the record doubtful. */
export const STALE_AFTER_MS = 150_000;

export interface DataAge {
  /** The time of the last successful read, as shown. */
  readAt: string;
  /** True when the record may not be current: the last look failed, or too long since a read. */
  stale: boolean;
}

/** `HH:MM` in the reader's locale and time zone, with the zone named: a bare "09:40" means nothing across sites. */
export function clock(at: Date, locale?: string): string {
  return at.toLocaleTimeString(locale, {
    hour: "2-digit",
    minute: "2-digit",
    timeZoneName: "short",
  });
}

/** The age notice for a record last read at `lastRead`, or null when nothing was read yet. */
export function dataAge(
  lastRead: Date | null,
  now: Date,
  lastLookFailed: boolean,
  locale?: string,
): DataAge | null {
  if (!lastRead) return null;
  const old = now.getTime() - lastRead.getTime() > STALE_AFTER_MS;
  return { readAt: clock(lastRead, locale), stale: lastLookFailed || old };
}
