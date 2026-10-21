// How many refused HL7 messages wait, shown in the navigation on the admin pages only. Each fetch is
// an audited search of the queue on the server, so it is not made on other pages, and a role that
// may not see the queue (any status but 200) shows nothing.
import { call } from "./api.js";

/** Whether a path is an admin page: the queue itself, or a patient's access and consent page. */
export function isAdminPage(pathname: string): boolean {
  return (
    /\/admin(\/|$)/.test(pathname) ||
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
