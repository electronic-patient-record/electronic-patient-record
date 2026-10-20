# UI — Specification

Source of truth for the UI's internals. The entity spec ([`../../spec/index.md`](../../spec/index.md)) wins on the
cross-subproject contract.

## 1. Built (M0)

- **URLs**: `/<locale>/…` with the ten `LOCALES`; a bare URL redirects to the remembered, browser-preferred or
  default (`en-001`) locale; `/api`, `/_app`, `/assets` are unlocalised.
- **Security**: strict CSP from `vite.config.ts` (nonces/hashes, no inline style except SvelteKit's own announcer),
  `nosniff`, `DENY` framing, referrer and permissions policies, HSTS over HTTPS.
- **BFF**: `/api/proxy/<path>` forwards GET/POST to `EPR_API_URL` only when the path starts with `api/` and has no
  empty, `.`, `..`, `%` or backslash segment. The browser holds no token.
- **Pages**: home; `/patients` compares two patients and renders the decision, the score, the per-field table and
  the reasons a link cannot be automatic. A missing similarity renders "not available".
- **Accessibility**: skip link, landmarks, labelled inputs, `role="alert"` on failure, axe (WCAG 2.2 AA) on both pages.

## 2. Not built

Sign-in and session (the proxy sends no token, so it works only against an API with login off); patient search,
registration, banner, summary; the clinical pages; the Lily widgets and `DataTable`; translations; the inbox; the
portal; dark-theme toggle (colours follow `prefers-color-scheme`).
