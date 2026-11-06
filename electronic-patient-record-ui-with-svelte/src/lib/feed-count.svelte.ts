// How many refused HL7 messages wait, shown in the navigation on the admin pages only. Each fetch is
// an audited search of the queue on the server, so it is not made on other pages, and a role that
// may not see the queue (any status but 200) shows nothing.
import { call } from "./api.js";

/**
 * Whether the refused-message count is fetched on this path: the queue itself and a patient's access
 * page. Not the emergency-access review page, which is the auditor's: the auditor's role is refused the
 * count, and every refusal is an audit row.
 */
export function isAdminPage(pathname: string): boolean {
  return (
    /\/admin\/feed\/?$/.test(pathname) ||
    /\/admin\/referral-queue\/?$/.test(pathname) ||
    /\/patients\/[^/]+\/access\/?$/.test(pathname)
  );
}

export const feedCount = $state<{ open: number | null }>({ open: null });

export async function refreshFeedCount(): Promise<void> {
  const res = await call<{ open: number }>("GET", "/api/feed/refusals/count");
  feedCount.open =
    res.status === 200 && Number.isInteger(res.body?.open)
      ? res.body.open
      : null;
}

// How many inbound referral messages wait unmatched (PC-2). Same rules as the feed count: admin pages only,
// and a role that may not see the queue (any status but 200) shows nothing.
export const referralQueueCount = $state<{ open: number | null }>({
  open: null,
});

export async function refreshReferralQueueCount(): Promise<void> {
  const res = await call<{ open: number }>("GET", "/api/referral-queue/count");
  referralQueueCount.open =
    res.status === 200 && Number.isInteger(res.body?.open)
      ? res.body.open
      : null;
}

/** Whether the referral-reply count is fetched on this path: the list itself and a patient's record, where a
 *  clinician works. A role that may not have a worklist makes one audited refusal per load there. */
export function isWorklistPage(pathname: string): boolean {
  return (
    /\/referrals\/?$/.test(pathname) ||
    /\/patients\/(?!new\/?$|compare\/?$)[^/]+\/?$/.test(pathname)
  );
}

/** Whether the count of death reports is fetched on this path: wherever staff start and where they work. A role
 *  that may not see the list makes one audited refusal per load there. */
export function isDeathReportsPage(pathname: string): boolean {
  return (
    /\/death-reports\/?$/.test(pathname) ||
    /\/patients\/?$/.test(pathname) ||
    /^(\/[a-z]{2,3}(-[A-Za-z0-9]+)*)?\/?$/.test(pathname) ||
    /\/patients\/(?!new\/?$|compare\/?$)[^/]+\/?$/.test(pathname)
  );
}

// How many reports of a death wait for a person (H-046, OQ-BV4). Any status but 200 shows nothing.
export const deathReportsCount = $state<{ open: number | null }>({
  open: null,
});

export async function refreshDeathReportsCount(): Promise<void> {
  const res = await call<{ open: number }>("GET", "/api/death-reports/count");
  deathReportsCount.open =
    res.status === 200 && Number.isInteger(res.body?.open)
      ? res.body.open
      : null;
}

// How many of this person's referrals have an answer they have not seen (H-044). Any status but 200 shows nothing.
export const worklistCount = $state<{ open: number | null }>({ open: null });

export async function refreshWorklistCount(): Promise<void> {
  const res = await call<{ open: number }>(
    "GET",
    "/api/referrals/worklist/count",
  );
  worklistCount.open =
    res.status === 200 && Number.isInteger(res.body?.open)
      ? res.body.open
      : null;
}
