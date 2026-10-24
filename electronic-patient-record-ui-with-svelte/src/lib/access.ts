// Who may see a patient's record: care relationships (who is the patient, a proxy, on the care team)
// and the patient's consent. The pages in `/patients/[id]/access` use these types and helpers; the
// server decides everything, this only builds and shows requests.
import { STRINGS } from "./strings.js";

export type RelationshipKind =
  "care-team" | "active-encounter" | "self" | "proxy";
export const RELATIONSHIP_KINDS: readonly RelationshipKind[] = [
  "care-team",
  "active-encounter",
  "self",
  "proxy",
];

export interface Period {
  start?: string | null;
  end?: string | null;
}

export type ProxyScope = "identity" | "allergies" | "photo" | "access-log";
export const PROXY_SCOPES: readonly ProxyScope[] = ["identity", "allergies", "photo", "access-log"];
export type ProxyBasis =
  "parental-responsibility" | "power-of-attorney" | "deputyship" | "patient-request";
export const PROXY_BASES: readonly ProxyBasis[] = [
  "parental-responsibility",
  "power-of-attorney",
  "deputyship",
  "patient-request",
];

/** What a proxy may use and until when (PA-5). `expires_at` is Unix seconds. */
export interface ProxyTerms {
  basis: ProxyBasis;
  scopes: ProxyScope[];
  expires_at: number;
}

export interface RelationshipRecord {
  subject: string;
  kind: RelationshipKind;
  status: "active" | "ended";
  period?: Period | null;
  proxy?: ProxyTerms | null;
}

export interface StoredRelationship {
  id: string;
  patient_id: string;
  version: number;
  author: string;
  relationship: RelationshipRecord;
}

export type Effect = "permit" | "deny";

export interface Provision {
  effect: Effect;
  actors: string[];
  purposes: string[];
  resources: string[];
  labels: string[];
  period?: Period | null;
}

export interface ConsentRecord {
  kind: "general" | "national-data-opt-out";
  status: "active" | "inactive" | "entered-in-error";
  provisions: Provision[];
  period?: Period | null;
}

export interface StoredConsent {
  id: string;
  patient_id: string;
  version: number;
  author: string;
  consent: ConsentRecord;
}

/** The roles, purposes, record kinds and labels the server knows. An empty choice means "any". */
export const ROLES = [
  "clinician",
  "nurse",
  "pharmacist",
  "allied-health",
  "receptionist",
  "clinical-admin",
  "patient",
  "proxy",
  "auditor",
  "integrator",
] as const;
export const PURPOSES = [
  "treatment",
  "emergency",
  "operations",
  "payment",
  "research",
  "patient-access",
  "audit",
] as const;
export const RESOURCES = [
  "PatientIdentity",
  "Encounter",
  "Condition",
  "Observation",
  "MedicationRequest",
  "AllergyIntolerance",
  "Document",
] as const;
export const LABELS = ["R", "V", "PSY", "HIV", "SUD", "ETH"] as const;

export interface ConsentForm {
  nationalOptOut: boolean;
  effect: Effect;
  actors: string[];
  purposes: string[];
  resources: string[];
  labels: string[];
  start: string;
  end: string;
}

export const emptyConsentForm = (): ConsentForm => ({
  nationalOptOut: false,
  effect: "deny",
  actors: [],
  purposes: [],
  resources: [],
  labels: [],
  start: "",
  end: "",
});

const period = (start: string, end: string): Period | null =>
  start.trim() || end.trim()
    ? {
        ...(start.trim() ? { start: start.trim() } : {}),
        ...(end.trim() ? { end: end.trim() } : {}),
      }
    : null;

/** The consent record a form describes: one provision, or the opt-out with none of its own. */
export function formToConsent(form: ConsentForm): ConsentRecord {
  if (form.nationalOptOut) {
    return { kind: "national-data-opt-out", status: "active", provisions: [] };
  }
  return {
    kind: "general",
    status: "active",
    provisions: [
      {
        effect: form.effect,
        actors: form.actors,
        purposes: form.purposes,
        resources: form.resources,
        labels: form.labels,
        period: period(form.start, form.end),
      },
    ],
  };
}

/** The relationship record for a new link. Dates are optional. */
export function relationshipRecord(
  subject: string,
  kind: RelationshipKind,
  start: string,
  end: string,
  proxy?: ProxyTerms | null,
): RelationshipRecord {
  return {
    subject: subject.trim(),
    kind,
    status: "active",
    period: period(start, end),
    ...(kind === "proxy" && proxy ? { proxy } : {}),
  };
}

/**
 * A proxy's terms from the form. The last day is inclusive, so the authority ends at the start of the day
 * after it (UTC). Null when the date is not a real `YYYY-MM-DD` day: the form then sends no terms and the
 * server refuses the proxy, rather than the page guessing an expiry.
 */
export function proxyTerms(
  basis: ProxyBasis,
  scopes: ProxyScope[],
  lastDay: string,
): ProxyTerms | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(lastDay.trim());
  if (!m) return null;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  const start = Date.UTC(y, mo - 1, d);
  const real = new Date(start);
  if (real.getUTCFullYear() !== y || real.getUTCMonth() !== mo - 1 || real.getUTCDate() !== d)
    return null;
  return { basis, scopes, expires_at: start / 1000 + 86_400 };
}

/** One line for the list: what the proxy may use, and when it ends. */
export function describeProxy(t: ProxyTerms | null | undefined): string {
  if (!t) return STRINGS.proxyNoTerms;
  // The authority ends at the start of the day after its last day.
  const end = new Date((t.expires_at - 86_400) * 1000).toISOString().slice(0, 10);
  return `${t.scopes.map((s) => STRINGS.proxyScopes[s]).join(", ")} · ${STRINGS.proxyUntil} ${end}`;
}

const list = (xs: string[], any: string) => (xs.length ? xs.join(", ") : any);

/** One provision in words, e.g. "Deny: clinician, nurse · for any purpose · any record · any label". */
export function describeProvision(p: Provision): string {
  const per =
    p.period?.start || p.period?.end
      ? ` · ${p.period.start ?? "…"} to ${p.period.end ?? "…"}`
      : "";
  return (
    `${p.effect === "permit" ? STRINGS.consentPermit : STRINGS.consentDeny}: ` +
    `${list(p.actors, STRINGS.anyRole)} · ${STRINGS.consentFor} ` +
    `${list(p.purposes, STRINGS.anyPurpose)} · ` +
    `${list(p.resources, STRINGS.anyRecord)} · ` +
    `${p.labels.length ? p.labels.join(", ") : STRINGS.anyLabel}${per}`
  );
}

export function describeConsent(c: ConsentRecord): string {
  return c.kind === "national-data-opt-out"
    ? STRINGS.consentOptOut
    : c.provisions.map(describeProvision).join(" | ") ||
        STRINGS.consentNoProvisions;
}
