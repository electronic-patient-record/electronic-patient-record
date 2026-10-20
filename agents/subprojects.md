# Subprojects

```
UI (SvelteKit)  ──REST/FHIR──►  Service (loco.rs)  ──crates.io dependency──►  Matcher (pure)
                                     │
                                     └──► PostgreSQL 18
```

The dependency direction is strictly downward. The matcher depends on nothing in the workspace. The service uses the
matcher's `Patient` and `MatchResult` directly as API bodies. The UI reaches the service only through its own
`/api/proxy` route. See the entity spec §8.
