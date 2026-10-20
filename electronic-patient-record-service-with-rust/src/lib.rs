//! Electronic patient record service (loco.rs, Axum, PostgreSQL).
//!
//! Scaffold (milestone M0): the app wiring, a secure-by-default PASETO guard,
//! the append-only audit table migration and one endpoint over the matcher.
//! The clinical record, the FHIR facade, the policy engine and the audit
//! writer are planned in `tasks.md` and are not built yet.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod app;
pub mod audit;
pub mod auth;
pub mod controllers;
