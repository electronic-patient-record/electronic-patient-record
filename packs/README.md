# Packs

Country data the service loads at start, enabled by `EPR_PACKS` (a comma-separated list of ids; unset means none).
A pack is data, never code. The design is "Pack mechanism" in [`spec/interoperability/index.md`](../spec/interoperability/index.md).
**Built so far (OS-10, first slice):** the phone calling code only (`phone.toml`). Every pack here is `reviewed = "none"`:
assembled from public sources, not signed by anyone who works with that country's standards.

| Pack | Calling code | Source |
| --- | --- | --- |
| `uk` | 44 | ITU-T E.164 country calling codes |
| `au` | 61 | ITU-T E.164 country calling codes |
