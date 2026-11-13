// The path to production (PR-7): what stands between this software and use in real care, one line per area of
// `governance/production-readiness.md`, with a readiness estimate from 0 to 100.
//
// The checklist is the source. `tests/unit/path-to-production.test.ts` reads its table and fails when an area,
// question, "what exists today" or estimate here differs from it, so the page and the document cannot drift.
// The lines stay in English on every locale for the same reason: they are the checklist's own words.
//
// An estimate is a judgement, not a measurement. It was written by an AI coding assistant from this
// repository on 2026-11-13, and no person has checked it. 100 on a line means its evidence exists and a person
// has looked at it on a real deployment; nothing else may reach 100. There is no overall figure, on purpose:
// every line must be done, and an average would hide a line at 0.

/** One line of the checklist. */
export interface ProductionStep {
  /** The checklist's area, such as "Safety". */
  area: string;
  /** What must be true before production. */
  question: string;
  /** What exists today, in the checklist's words. */
  today: string;
  /** Readiness, 0 to 100: a judgement, never a measurement. */
  estimate: number;
  /** Why the estimate is what it is. */
  why: string;
}

/** The day the estimates were written. */
export const ESTIMATED_ON = "2026-11-13";

/** The checklist's lines, in its order. */
export const PRODUCTION_STEPS: readonly ProductionStep[] = [
  {
    area: "Safety",
    question:
      "Is each hazard reviewed, and is every open one accepted or closed by the Clinical Safety Officer?",
    today:
      "55 hazards on 2026-11-09, 34 open, none accepted or closed; no reviewer is named",
    estimate: 5,
    why: "The hazard log exists and is kept with the code, but no Clinical Safety Officer is named, so no hazard can be accepted or closed.",
  },
  {
    area: "Safety",
    question: "Does each control name a test that still exists?",
    today: "Named for H-044 only",
    estimate: 5,
    why: "The check exists; one hazard of 55 uses it.",
  },
  {
    area: "Privacy",
    question: "DPIA signed, retention periods set, data-subject routes tested",
    today: "Drafts; the periods are not set",
    estimate: 15,
    why: "Drafts exist. Nobody has signed them, and the retention periods are a decision nobody has made.",
  },
  {
    area: "Identity",
    question: "Second factor required for staff",
    today: "Not enforced or checked",
    estimate: 10,
    why: "The identity provider can ask for a second factor; nothing here requires it or checks that it did.",
  },
  {
    area: "Transport",
    question:
      "TLS everywhere, including in front of MLLP, and `DB_URL` sets `sslmode` to `require`, `verify-ca` or `verify-full`",
    today: "Not built here; MLLP is plaintext",
    estimate: 20,
    why: "The service refuses an unencrypted database connection in production; the HL7 feed has no encryption.",
  },
  {
    area: "Secrets",
    question:
      "Keys held outside the image and rotated; `EPR_INTEGRITY_MAC_KEY` set",
    today: "Variables exist; rotation untested end to end",
    estimate: 30,
    why: "Keys are read from the environment, not built in; rotating one has never been tried.",
  },
  {
    area: "Data",
    question: "Backups taken and a restore tried; recovery time known",
    today: "**No backup procedure exists**",
    estimate: 0,
    why: "Nothing is written.",
  },
  {
    area: "Audit",
    question: "Chain verified on a schedule; archive anchors checked",
    today: "Verify route and tests exist; no schedule",
    estimate: 40,
    why: "The audit trail can be verified and the tests prove it; nothing runs the check on a schedule.",
  },
  {
    area: "Operations",
    question: "Metrics scraped, alerts set, logs kept",
    today: "`/metrics` exists (off by default); no alerts",
    estimate: 20,
    why: "Metrics can be read; nothing collects them or raises an alert.",
  },
  {
    area: "Operations",
    question: "One replica runs each background job",
    today: "Built (PR-4); not tried with two real processes",
    estimate: 50,
    why: "Built and tested in one process; not tried with two.",
  },
  {
    area: "Workforce",
    question:
      "Workforce planning holds the rota and keeps an `on-duty-clinician` on every shift, and the role changes in the identity provider at each handover",
    today:
      "No link built; the role is given by hand, and the service cannot see a gap",
    estimate: 5,
    why: "The role exists; giving it to the right person at each handover is by hand.",
  },
  {
    area: "Referrals",
    question:
      "Timeout periods set per priority; an owner for the unmatched queue",
    today: "Off by default; no owner named",
    estimate: 30,
    why: "The timeouts and the queue are built; setting the periods and naming an owner is the deploying organisation's work.",
  },
  {
    area: "Interop",
    question: "Each feed and message type tested with the real partner",
    today: "Only the synthetic and unit cases",
    estimate: 5,
    why: "No real partner has sent a message.",
  },
  {
    area: "Accessibility",
    question: "Screen-reader and keyboard pass on the real screens",
    today: "Automated checks only",
    estimate: 25,
    why: "An automated audit opens every page; no person has used the screens with a screen reader.",
  },
  {
    area: "Language",
    question: "Any non-English text reviewed by a speaker",
    today: "Machine-written; none reviewed",
    estimate: 10,
    why: "The Welsh and the form words are machine-written, and no speaker has read them.",
  },
  {
    area: "Release",
    question:
      "Supply chain, SBOM, licences, vulnerability scan green on the release commit",
    today: "CI runs these on each push",
    estimate: 50,
    why: "The checks run on every push; no release has been made, so none has run on a release commit.",
  },
  {
    area: "Support",
    question: "A named on-call, an incident route, a rollback",
    today: "Not written",
    estimate: 0,
    why: "Nothing is written and nobody is named.",
  },
];

/** The checklist's `today` text for display: its Markdown emphasis and code marks removed. */
export function plain(text: string): string {
  return text.replace(/\*\*(.*?)\*\*/g, "$1").replace(/`([^`]*)`/g, "$1");
}
