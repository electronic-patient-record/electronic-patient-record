//! Electronic patient record service (loco.rs, Axum, PostgreSQL).
//!
//! Scaffold (milestone M0): the app wiring, a secure-by-default PASETO guard,
//! the append-only audit table migration and one endpoint over the matcher.
//! The clinical record and the FHIR facade are planned in `tasks.md` and are not built yet.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod adt_feed;
pub mod allergies;
pub mod api_error;
pub mod app;
pub mod audit;
pub mod auth;
pub mod authz;
pub mod break_glass;
pub mod consent;
pub mod consents_store;
pub mod controllers;
pub mod feed_queue;
pub mod ids;
pub mod interop;
pub mod links;
pub mod metrics;
pub mod mllp;
pub mod patients_store;
pub mod photos_store;
pub mod policy;
pub mod relationships;
pub mod routes;
pub mod store;
