# Packs

Country data the service loads at start, enabled by `EPR_PACKS` (a comma-separated list of ids; unset means none).
A pack is data, never code. The design is "Pack mechanism" in [`spec/interoperability/index.md`](../spec/interoperability/index.md).
**Built so far (OS-10):** the phone calling code (`phone.toml`) and, optionally, the Australian IHI rule (`identifiers.toml`: `use_ihi = true` plus a `[[match]]` and a `[[differ]]` vector of two IHIs each; off unless a pack says so). Every pack here is `reviewed = "none"`:
assembled from public sources, not signed by anyone who works with that country's standards.

| Pack | Calling code | Source |
| --- | --- | --- |
| `uk` | 44 | ITU-T E.164 country calling codes |
| `au` | 61, and the IHI rule on | ITU-T E.164 country calling codes; the IHI shape (16 digits, prefix 800360, Luhn) from the plan's note, not re-read |
