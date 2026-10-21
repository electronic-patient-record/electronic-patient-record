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

export interface RelationshipRecord {
  subject: string;
  kind: RelationshipKind;
  status: "active" | "ended";
  period?: Period | null;
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
): RelationshipRecord {
  return {
    subject: subject.trim(),
    kind,
    status: "active",
    period: period(start, end),
  };
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
