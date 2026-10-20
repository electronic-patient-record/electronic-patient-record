// English-only strings. Translations for the other locales in `LOCALES` are
// not built yet (tasks SF-4); a missing translation falls back to this table.
export const STRINGS = {
  brand: "Electronic Patient Record",
  skip: "Skip to main content",
  navHome: "Home",
  navPatients: "Patients",
  homeTitle: "Electronic Patient Record",
  homeIntro:
    "Scaffold only. No clinical record exists yet; this build can compare two patient identity records and explain the result.",
  notClinical: "Not for clinical use. Do not enter real patient data.",
  compareTitle: "Compare two patients",
  patientA: "Patient A",
  patientB: "Patient B",
  family: "Family name",
  given: "Given name",
  birthDate: "Birth date (YYYY-MM-DD)",
  compare: "Compare",
  decision: "Decision",
  score: "Score",
  field: "Field",
  similarity: "Similarity",
  notAvailable: "not available",
  conflicts: "Reasons this cannot be linked automatically",
  error: "The comparison failed.",
  decisions: {
    auto_link: "Automatic link",
    review: "Needs human review",
    non_match: "Not the same person",
  },
} as const;
