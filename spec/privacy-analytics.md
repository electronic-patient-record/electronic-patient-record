# Privacy-preserving data analysis, including homomorphic encryption

**A direction from the author's instruction of 2026-11-04 (plan.md §27). Nothing here is built, and no library was run.** The aim: analyse patient data (quality, research, service planning) without the analyst or the compute host seeing the records in the clear.

## What homomorphic encryption is, and is not

Homomorphic encryption (HE) lets a program compute on encrypted values and return an encrypted result that only the key holder can open. **Partially** homomorphic schemes do one operation (addition or multiplication); **fully** homomorphic (FHE) schemes do both, so in principle any computation. Common schemes: BFV and CKKS (arithmetic on integers or approximate reals) and TFHE (bit-level, with bootstrapping).
The bounds, from the sources read: encrypted queries are **orders of magnitude slower** than plaintext; ciphertexts can be **megabytes for one value**; library support for common statistical operations is limited (a 2026 Chalmers thesis); HE "adds massive computational cost" (a 2024 thesis). Standards are thin: the HomomorphicEncryption.org security standard dates from 2018, and ISO/IEC 18033-6 covers only two partially homomorphic schemes (exponential ElGamal and Paillier). Libraries named in the search: Microsoft SEAL and OpenFHE; whether `tfhe-rs` suits this project was not established.
HE does not make an answer safe to publish. A result of "1 patient" is still a disclosure. The usual pairing is HE with **differential privacy** or small-number suppression on the output.

## Where it could help here, and where it must not go

- **Could help:** narrow, aggregate analyses (counts, sums, means, simple models) over data a third party must not see in the clear: a multi-site comparison, an outside analyst, a cloud compute host.
- **Must not touch the clinical path.** The record stays as it is (encrypted at rest, plaintext to the service); no read or write for care goes through HE. It is for **secondary use** only.
- **Not a substitute** for access control, audit or a lawful basis. Whether data encrypted this way is still personal data is a question for the data protection officer (OQ-HE1).

## Options for secondary-use analysis, compared

| Option | Idea | For | Against |
|---|---|---|---|
| De-identified extract | Remove identifiers, release rows | Simple, fast | Re-identification risk; rows leave the controller |
| Trusted research environment | Analyst works inside a locked environment, only checked outputs leave | Established in practice | Staff to run it; output checking is manual |
| Federated analysis | Each site computes locally, shares aggregates | No rows move | Needs software at every site; aggregates can still leak |
| Differential privacy | Add calibrated noise to outputs | A formal guarantee | Accuracy cost; a privacy budget to manage |
| Secure enclaves | Compute in hardware-protected memory | Fast | Trust in the hardware vendor; side-channel history |
| **Homomorphic encryption** | Compute on ciphertext | Host never sees plaintext | Slow, large, limited operations, key management, immature standards |

## Design rules (proposed)

1. A **secondary-use export** is a separate, audited job with a stated purpose; it reads under that purpose, never under a care relationship.
2. **The data owner holds the decryption key.** An analyst who needs the result asks the owner to decrypt a checked aggregate. Key custody and rotation follow the integrity-key practice already in the service; this is a new key with its own plan.
3. **Output rules:** aggregates only, a minimum cell size, and a noise or suppression step before any result leaves.
4. **Start with a spike on synthetic data** (`XP-8`): an encrypted mean and count across two simulated sites, measuring time and size against plaintext and against differential privacy; adopt only if the result is useful at a cost the sites accept.
5. **Post-1.0.** This is not on the path to 1.0.

## Not done, not verified

No library was installed or benchmarked; the speed and size figures are from two theses found by search, not from our data. The standards status is from search results. Rust bindings or maturity of any library were not checked. Legal status of encrypted personal data was not researched. Open questions: OQ-HE1, OQ-HE2. Tasks: `XP-8`, `XP-9`.
