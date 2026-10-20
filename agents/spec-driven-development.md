# Spec-driven development

- Each subproject's `spec/` is the source of truth for that subproject's internals; the entity [`spec/`](../spec/index.md)
  is the source of truth for the cross-subproject contract. If they disagree, open a task; never silently rewrite the loser.
- **One commit** carries the spec edit, the code edit and the test edit. A change that crosses subprojects (a new
  `Patient` field) carries all the touched specs, the UI types and the changelogs.
- If the change could alter clinical behaviour, update the hazard log in the same commit (task AS-1).

| You are changing… | Update… |
|---|---|
| `Patient` fields or matching rules | matcher spec; entity spec §5; UI types |
| An endpoint or wire convention | entity spec §9; service spec; UI proxy rules if the path changes |
| A table or migration | entity spec §10; service spec |
| A requirement or its acceptance | entity spec §6/§7 and `tasks.md` |
| What is built | `spec/14-implementation-status.md` (run `sh scripts/generate-status.sh`) |
