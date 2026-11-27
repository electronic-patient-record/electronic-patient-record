// The fixed words the service gives for what a judgement of whether a person can decide does not record (RF-8,
// `decision_ability::gaps` in the service's domain crate), and the page's key for each. A unit test checks each against
// the service's source, so a change there is not shown in English on a Welsh page in silence.

import type { Strings } from "./i18n.js";

export const GAP_KEYS = {
  "no help is recorded before a judgement that they cannot decide":
    "daGapNoHelp",
  "they communicate by other means and no communication aid is recorded":
    "daGapNoAid",
  "whether they were conscious is not known": "daGapConscious",
} as const satisfies Record<string, keyof Strings>;

/** A gap in the page's words, or as the service said it when the page does not know it. */
export function gapText(gap: string, text: Strings): string {
  const key = (GAP_KEYS as Record<string, keyof Strings>)[gap];
  const words = key ? text[key] : undefined;
  return typeof words === "string" ? words : gap;
}
