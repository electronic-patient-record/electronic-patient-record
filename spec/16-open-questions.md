## 16. Open Questions

| ID | Question | Owner |
|---|---|---|
| OQ-1 | Licence: keep the five-way choice, or Apache-2.0 alone? | maintainer |
| OQ-2 | Who is the named Clinical Safety Officer; is there a clinical advisory group? | maintainer |
| OQ-3 | Is the SMART authorisation server part of this service or the shared authentication service? | maintainer |
| OQ-4 | First market: UK or US? (Both profile sets are planned; order matters for M6) | maintainer |
| OQ-5 | Is offline-first data entry needed for target sites? | maintainer |
| OQ-6 | Which features would be SaMD, and is any wanted for 1.0? | maintainer |
| OQ-7 | Terminology server: Snowstorm (open, SNOMED only) or Ontoserver (commercial)? | maintainer |
| OQ-T1 | May the project encode Manchester Triage flowcharts and discriminators, and under what licence? Interim: framework only, no MTS content in the repo; Wiley and the group not yet asked (TR-1, plan.md §15.1.1) | maintainer |
| OQ-T2 | Is emergency triage in scope for 1.0, or post-1.0? **Decided 2026-10-23: post-1.0** (TR-2) | maintainer |
| OQ-T3 | Which FHIR element and code system carry the triage category? Proposed in `spec/triage/index.md` §3, not confirmed (TR-3) | maintainer |
| OQ-T4 | Do we support other triage systems (ESI, CTAS, ATS) through a pluggable pack? **Decided 2026-10-23: pluggable interface, build only MTS** (TR-2) | maintainer |
| OQ-P1 | Does the project or each deploying organisation hold the Wong-Baker FACES licence? No artwork in the repo until settled (PN-1) | maintainer |
| OQ-P2 | Which LOINC codes and value set bind pain observations (72514-3, 38221-8 unverified)? (PN-2) | maintainer |
| OQ-P3 | Do we support observer pain scales (FLACC, CPOT, PAINAD)? (PN-10) | maintainer |
| OQ-M1 | Which module groups are in the 1.0 claim, and which are post-1.0? **Proposed 2026-10-24, not yet confirmed by the maintainer:** eMAR, nursing assessments and discharge summary in M7, the rest post-1.0 (see [19](19-module-map.md)). (EM-1) | maintainer |
| OQ-M2 | Which risk tools (Waterlow, Braden, MUST, falls, VTE) may we encode, and under what licence? (EM-3) | maintainer |
| OQ-M3 | Do we build closed-loop barcode scanning ourselves or integrate a wristband and drug-barcode service? (EM-2) | maintainer |
| OQ-PA1 | Does the EPR defer to the Patient Administration System (PAS), keep a minimal scheduler behind a port, or neither? **Proposed 2026-10-23, not yet confirmed by the maintainer: a minimal scheduler behind a scheduling port the PAS can implement** (PA-1, plan.md §18.2; the split is in `spec/02-scope.md`) | maintainer |
| OQ-PA2 | Which system owns the person identity, and how does a merge reach the PAS? (PA-2) | maintainer |
| OQ-PA3 | FHIR R4 or R5 at the PAS boundary, and who builds the edge? (PA-9; IO-16 built the read-only R5 facade derived from R4 on 2026-10-29; the write side and the PAS mapping stay open) | maintainer |
| OQ-PA4 | Do the SNOMED CT UK acuity concepts the PAS cites exist and mean what it says? (PA-10) | maintainer |
| OQ-E1 | Does the EPR own resuscitation status (DNACPR or ReSPECT) and show it in the banner, or read it from a shared care record? Recommendation: a read-only copy with its source and date. (EC-5) | maintainer |
| OQ-E2 | How is an unidentified patient's record joined to the real record later? Recommendation: a merge with a reason, never an automatic link. (EC-2) | maintainer |
| OQ-E3 | Does the ED board need a major-incident mode in 1.0? Recommendation: no, post-1.0. (EC-9) | maintainer |
| OQ-OS1 | Does a request limit apply to clinicians, or only to unauthenticated and integrator traffic? Recommendation: per-subject limits that never trip in clinical use, strict on sign-in and `$match`. **Assumed 2026-10-30 (maintainer asked for OS-2; the Clinical Safety Officer has not answered):** the recommendation is built, with defaults far above clinical use; H-039 is open. (OS-2, plan.md §22.4) | maintainer, Clinical Safety Officer |
| OQ-OS2 | Should the ABAC policy become data reviewable by clinical governance (Medplum-style), or stay code with census tests? Recommendation: code until a Clinical Safety Officer asks to review it. (plan.md §22.2) | maintainer |
| OQ-OS4 | What may an OTLP exporter send out of the process? Spans can carry request ids, route templates, subject and timings; SQL spans from `sqlx::query` can carry statement text. Recommendation: export only the service's own spans, never `sqlx::query` or `tower_http` fields, with the subject hashed, and record the collector as a recipient in `governance/record-of-processing.md` first. (OS-6, plan.md §22) | maintainer, data protection officer |
| OQ-OS5 | Pack file format and home: TOML rules with CSV tables in `packs/` inside this repository, or YAML like the service config, or one repository per pack? Recommendation: TOML and CSV, in this repository until a second maintainer owns a country. (OS-10) | maintainer |
| OQ-OS6 | May a pack with `reviewed = none` be enabled in production? Recommendation: yes, with a startup warning and a line in `metadata`, because the alternative is no country support at all; never labelled reviewed. (OS-10) | maintainer, Clinical Safety Officer |
| OQ-OS3 | CodeQL's Rust analysis or semgrep as the SAST gate? Recommendation: CodeQL for the UI now, Rust when confirmed, semgrep meanwhile. (OS-4) | maintainer |

No answers recorded yet, except where a row says decided or proposed.
