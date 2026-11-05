# Service: index

| Path | Purpose |
|---|---|
| [AGENTS.md](AGENTS.md) | Rules for agents working on this crate |
| [spec/index.md](spec/index.md) | The crate specification |
| `src/app.rs` | loco `Hooks`: routes, auth middleware |
| `src/auth.rs` | Bearer guard and the startup gate |
| `src/controllers/patients.rs` | `POST /api/patients/compare` |
| `migrations/` | Schema migrations, as sqlx `.sql` files with no down files |
| `config/` | `development`, `test`, `container-dev`, `production` |
| `tests/audit_immutable.rs` | Database-gated test |
