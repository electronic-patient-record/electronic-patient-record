# Testing

| Gate | Proves | Does not prove |
|---|---|---|
| `cargo test` (matcher) | The matching behaviour the tests state | That thresholds are clinically right |
| `cargo test --lib` (service) | The auth decisions, the compare handler | That the app boots or routes are mounted |
| `--ignored` DB tests | Migrations apply; audit table refuses change | Anything about stored patient data (none yet) |
| `vitest` | Locale helpers, headers, proxy rules | Rendering |
| Playwright + axe | Pages render, headers present, no axe violations | Usability with real clinicians |
| `coverage` job (OS-4) | Lines ran: matcher at least 95 percent, service database-free tests at least 50 (measured 97.65 and 52.24 on 2026-10-30) | That anything was asserted about them; the database-gated tests, the UI and Rust static analysis are outside it |
| `doc_census` (OS-9) | Config, `SECURITY.md` and `deny.toml` name only files that exist | That their description is true |

To check a change for real, boot the service (`cargo run -- start` against a test Postgres) and curl
`127.0.0.1:5150`. A passing test is evidence of the behaviour the test describes, nothing more.
