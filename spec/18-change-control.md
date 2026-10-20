## 18. Change Control

1. A behavioural change is one commit: spec + code + test ([`../agents/spec-driven-development.md`](../agents/spec-driven-development.md)).
2. **Clinical-safety impact assessment on every change**: does it alter what a clinician sees, decides, signs or is
   alerted to? If yes, update the hazard log (AS-1) in the same commit; when unsure, treat it as clinical.
3. `CHANGELOG.md` states what was not done or verified. Do not weaken a guard or test to get green.
4. Disagreement between specs is a task, not a silent rewrite.
5. Regulation moves (HTI-5, the HIPAA NPRM, the DCB review): re-check [12](12-compliance.md) at each milestone.
