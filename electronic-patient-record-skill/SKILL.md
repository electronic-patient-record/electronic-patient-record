---
name: electronic-patient-record
description: How to work in the electronic-patient-record monorepo (matcher, loco.rs service, SvelteKit UI): spec-first, one commit per item, the gates, and the clinical-safety rules. Use before changing code, specs, the hazard log or translations here.
---

# Working in electronic-patient-record

Not for clinical use (milestone M0). Do not claim compliance, certification or clinical validation.

1. Read the repository `AGENTS.md`, then the subproject `AGENTS.md` and `spec/index.md`. `llms.txt` lists the documents worth reading.
2. Spec first: one commit with a spec edit, a code edit and a test edit; one item, one commit, one push, then look at CI. Stage named files, never `git add -A`.
3. Say what is not built or not tested in every `CHANGELOG.md` entry. Never weaken a guard or a census test to get green.
4. Clinical risk: update `governance/clinical-safety/hazard-log.csv` in the same commit (edit it with the csv module). Never set a hazard to `accepted` or `closed`.
5. Never loosen a matcher refusal, never add a delete path for clinical data, never show a missing value as zero.
6. Translations are machine-written and unreviewed: say so, never claim otherwise.
7. Run the gates in `AGENTS.md` (`sh scripts/generate-status.sh` and `python3 scripts/generate-llms.py` after adding tests, ticking tasks or moving documents). Boot the service for real when you change routes or migrations.
8. Versions: a dependency bump is one commit with its gates. TypeScript 7 does not work with SvelteKit 3.1 (`svelte-kit sync` calls `ts.readConfigFile`); pnpm holds releases under a day old.

See [`references/`](references/README.md).
