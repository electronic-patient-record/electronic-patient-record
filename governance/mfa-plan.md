# Multi-factor authentication plan (PR-7, DRAFT)

**NOT FOR CLINICAL USE.** `spec/07-non-functional-requirements.md` NFR-6 lists MFA as a requirement. This says who is responsible for it today. Read from `src/auth.rs`, `operations/keycloak/README.md` and `operations/keycloak/epr-realm.json` on 2026-11-04. Nothing here says MFA is in place.

## What the service does

- It verifies a bearer token: a PASETO v4 (offline, against configured keys) or a Keycloak access token (signature, issuer, audience, expiry, verified e-mail). It reads **no claim about how the user signed in** (no `acr`, no `amr`), so it cannot tell a password login from a two-factor one and cannot demand the second factor.
- It refuses to start with half-configured Keycloak, an `http://` issuer that is not loopback, or no token keys.
- `EPR_REQUIRE_AUTH` keeps login on by default.

## What Keycloak enforces, as far as the development realm shows

- `bruteForceProtected` is on in `epr-realm.json`.
- A search of that file for one-time-password, WebAuthn, passkey, required-action and browser-flow settings found **none**: the development realm does not require a second factor.
- A production realm is not in this repository ("A production Keycloak sits behind TLS and is not built here").

## What is left to the deployment

1. Configure the realm to require a second factor for every staff role, ideally a phishing-resistant one (WebAuthn / passkey), with a recovery route that does not reduce it to a password. NIST SP 800-63-4 and the national rule that applies decide the assurance level **[deployer to decide; plan.md §25 marks passkey requirements uncertain]**.
2. Decide how a break-glass user and an integration account (a system with no human) authenticate; a service account cannot use a second factor and needs a different control (network limits, short-lived tokens).
3. Decide session length and re-authentication for sensitive actions (clearing the referral queue, break-glass review, hazard sign-off).
4. PASETO tokens come from an authentication service this repository does not build; its factors are that service's.

## Not built, not tested

- Reading `acr`/`amr` to refuse a token without a second factor is **not built**. Until it is, MFA depends wholly on the realm and an operator can switch it off unseen.
- No test signs in with a second factor.
- Patient and proxy sign-in is not covered by any realm here.
