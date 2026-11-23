# Subprojects

```
UI (SvelteKit)  ──REST/FHIR──►  Service (Axum, axa)  ──crates.io dependency──►  Matcher (pure)
                                ▲    │
iOS app (SwiftUI)  ──REST───────┤    └──► PostgreSQL 18
Android app (Compose) ──REST────┘
```

The dependency direction is strictly downward. The matcher depends on nothing in the workspace. The service uses the
matcher's `Patient` and `MatchResult` directly as API bodies. The UI reaches the service only through its own
`/api/proxy` route. The two mobile apps are OAuth public clients that call the service's `/api` directly, with no
BFF; each keeps its rules in a core with no UI, and both build against a pinned Lily Design System checkout
(`scripts/fetch-lily.sh`). See [`spec/mobile-apps`](../spec/mobile-apps/index.md) and the entity spec §8.
