---
name: electronic-patient-record
description: How to work in the electronic-patient-record monorepo (matcher, loco.rs service, SvelteKit UI): spec-first, gates, and the clinical-safety rules.
---

# Working in electronic-patient-record

1. Read the repository `AGENTS.md`, then the subproject `AGENTS.md` and `spec/index.md`.
2. Spec first: one commit with spec, code and test. Say what is not built.
3. Never loosen a matcher refusal, never add a delete path for clinical data, never show a missing value as zero.
4. Run the gates in `AGENTS.md`. Boot the service for real when you change routes or migrations.
5. Not for clinical use: do not claim compliance, certification or clinical validation.

See [`references/`](references/README.md).
