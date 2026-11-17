// The HL7 feed's health (service `feed_health.rs`, IH-8), as words for the admin screen. The state is given in words
// and never by colour alone; no data is "unknown", never "healthy".
import { siteWhenText } from "./access-log.js";
import { TEXT } from "./i18n.js";

export type FeedState = "unknown" | "healthy" | "warning" | "critical";

export type FeedHealth = {
  sender: string;
  state: FeedState;
  window: "day" | "evening" | "weekend";
  reasons: string[];
  since: string | null;
  last_message_at: string | null;
  refusals_last_hour: number;
  open_refusals: number;
};

export type FeedHealthAnswer = { feeds: FeedHealth[]; site_time_zone?: string };

/** The lines that describe one feed, the state first. */
export function healthLines(f: FeedHealth, siteZone?: string): string[] {
  const lines = [`${TEXT.feedHealthStates[f.state]}: ${f.sender}`];
  if (f.reasons.length) lines.push(f.reasons.join("; "));
  lines.push(
    TEXT.feedHealthLast.replace(
      "{when}",
      f.last_message_at
        ? siteWhenText(f.last_message_at, siteZone)
        : TEXT.feedHealthNever,
    ),
  );
  lines.push(
    TEXT.feedHealthCounts
      .replace("{hour}", String(f.refusals_last_hour))
      .replace("{open}", String(f.open_refusals)),
  );
  lines.push(
    TEXT.feedHealthWindow.replace("{window}", TEXT.feedHealthWindows[f.window]),
  );
  if (f.since)
    lines.push(
      TEXT.feedHealthSince.replace("{when}", siteWhenText(f.since, siteZone)),
    );
  return lines;
}
