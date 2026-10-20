# Testing

| Gate | Proves | Does not prove |
|---|---|---|
| `cargo test` (matcher) | The matching behaviour the tests state | That thresholds are clinically right |
| `cargo test --lib` (service) | The auth decisions, the compare handler | That the app boots or routes are mounted |
| `--ignored` DB tests | Migrations apply; audit table refuses change | Anything about stored patient data (none yet) |
| `vitest` | Locale helpers, headers, proxy rules | Rendering |
| Playwright + axe | Pages render, headers present, no axe violations | Usability with real clinicians |

To check a change for real, boot the service (`cargo run -- start` against a test Postgres) and curl
`127.0.0.1:5150`. A passing test is evidence of the behaviour the test describes, nothing more.
