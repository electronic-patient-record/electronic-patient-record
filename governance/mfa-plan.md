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

## The native apps: a lost or shared phone (MP-11, DRAFT)

Read from `spec/mobile-apps/index.md` §2 and §3 on 2026-11-23. Neither app signs in yet (MP-5); this is the plan the sign-in must meet, not a description of a running system. It does not settle OQ-MP1 (who owns the device) or OQ-MP3 (distribution): those are the maintainer's, and each answer changes what is controlled (below).

- **A phone unlock is not the second factor.** Face ID, a fingerprint or the passcode unlocks the app on the device (the idle lock, spec §3 rule 6). The service never sees it and cannot tell it happened. The second factor for the service is whatever the realm requires at sign-in, through the system browser; the app cannot enforce it, for the same reason the service cannot (it reads no `acr` or `amr`). Nothing about the apps makes MFA stronger than the realm.
- **What a thief or a colleague gets from a locked phone:** nothing from the record. The access token is in memory only, the refresh token (if issued) is in the Keychain or Keystore, readable only while the device is unlocked and not copied to another device or a backup (`…ThisDeviceOnly`; Android app data excluded from backup, to be set and tested in MP-6). **From an unlocked phone left open:** the app's idle lock (900 s by default) is the only barrier, so the time a record stays readable is the idle limit plus however long the phone stays unlocked; a shorter limit for the app is a decision for the deployer.
- **What a lost phone costs, by device ownership (OQ-MP1):**
  - *Managed by the trust (the recommendation):* the trust can wipe the device, enforce a passcode and block a jailbroken one. Remote wipe is the control for the refresh token on the device.
  - *Owned by the clinician:* no wipe by the trust. The control is then to revoke the user's sessions and refresh tokens in Keycloak on the report, which needs a named person and a time limit for the report **[deployer to decide]**. Until the apps have an app-level sign-out that clears the Keychain item on the next launch with network, a stolen unlocked-once phone keeps its refresh token until the realm's session ends. The recommendation in the spec is bring-your-own only with that and the MFA plan.
- **Refresh token lifetime:** an absolute limit of 28800 s in the app (spec §3 rule 6), and the realm's own session and refresh-token lifetimes, which this repository's development realm does not set for `epr-ios` or `epr-android` (the clients do not exist yet). No `offline_access` scope is to be requested: an offline token outlives the session and defeats revocation by session end.
- **A shared phone** (a ward device used by several clinicians) is not supported by the plan: one signed-in user per app install, sign-out clears the Keychain item, and there is no fast user switch. Whether a shared ward device is wanted is a question for the maintainer, and it would need its own hazard analysis.
- **Distribution (OQ-MP3):** managed distribution (Apple Business Manager, managed Google Play) lets the trust control which build runs and revoke it; the public stores do not, and a public listing invites a fake app that asks for the same sign-in. The recommendation is managed first.

Not built, not tested here: sign-in, the Keychain and Keystore storage, the idle lock, the snapshot cover, the backup exclusion, the Keycloak clients, a revocation procedure, a lost-device report route. No device was lost, wiped or revoked in any test.

## Not built, not tested

- Reading `acr`/`amr` to refuse a token without a second factor is **not built**. Until it is, MFA depends wholly on the realm and an operator can switch it off unseen.
- No test signs in with a second factor.
- Patient and proxy sign-in is not covered by any realm here.
