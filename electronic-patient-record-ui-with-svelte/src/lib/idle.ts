// The idle-session warning (CQ-8). The BFF ends a session after `EPR_SESSION_IDLE_SECONDS` without
// activity; background looks (the patient page's minute poll) do not count. The page keeps its own
// note of the last request that did count. It can only be older than the server's (the server also
// counts page loads and other tabs), so the warning comes early, never late.

/** How long before the idle limit the warning shows. */
export const WARN_SECONDS = 120;

let lastActivity = Date.now();

/** Record that a request which counts as activity was just made. */
export function noteActivity(now = Date.now()): void {
  lastActivity = now;
}

export function lastActivityAt(): number {
  return lastActivity;
}

/**
 * Whole minutes left before the session ends for inactivity (at least 1 while any time is left,
 * 0 once it is up), or null when no warning is due or sign-in is off.
 */
export function idleWarning(
  nowMs: number,
  lastMs: number,
  idleSeconds: number | null,
): number | null {
  if (idleSeconds === null) return null;
  const left = idleSeconds - (nowMs - lastMs) / 1000;
  if (left > WARN_SECONDS) return null;
  if (left <= 0) return 0;
  return Math.max(1, Math.ceil(left / 60));
}
